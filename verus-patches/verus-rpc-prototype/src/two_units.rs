//! Two state units connected by an interrupt pin, after PIT -> PIC.
//!
//! - The source (PIT-like) has a counter; its interrupt output is
//!   `SourceDevice::output_of(counter)`. Running or restoring it re-drives the
//!   pin.
//! - The target (PIC-like) has registers and the level it sees on the pin. As
//!   in OpenVMM, restoring the target does not touch the pin level; it is
//!   kept live.
//!
//! Resources:
//! - one unit `FracGhost` per unit, as in `unit.rs`; the target's covers its
//!   registers only;
//! - one `FracGhost<bool>` for the pin level. One half always stays with the
//!   target (tied to its stored level); the other is with the source while it
//!   runs (tied to its output) and with the caller while it is stopped.
//!
//! Changing the level needs the whole pin resource, so the source drives the
//! pin by sending its half to the target with `SetLevel`. In OpenVMM this is
//! a direct call under the PIC's mutex; here it is an RPC.
//!
//! Everything in this file is verified.

use crate::rpc::*;
use vstd::prelude::*;
use vstd::resource::Loc;
use vstd::resource::frac::FracGhost;

verus! {

// ---------------------------------------------------------------------------
// Target (PIC-like)
// ---------------------------------------------------------------------------

pub struct TargetDevice {
    regs: u64,
    level: bool,
}

impl TargetDevice {
    pub closed spec fn regs(&self) -> u64 {
        self.regs
    }

    pub closed spec fn level(&self) -> bool {
        self.level
    }

    pub fn new(regs: u64, level: bool) -> (result: Self)
        ensures
            result.regs() == regs,
            result.level() == level,
    {
        TargetDevice { regs, level }
    }

    pub(crate) fn save(&self) -> (blob: u64)
        ensures
            blob == self.regs(),
    {
        self.regs
    }

    /// Restores the registers; the pin level is kept live.
    pub(crate) fn restore(&mut self, blob: u64)
        ensures
            final(self).regs() == blob,
            final(self).level() == old(self).level(),
    {
        self.regs = blob;
    }

    pub(crate) fn set_level(&mut self, level: bool)
        ensures
            final(self).regs() == old(self).regs(),
            final(self).level() == level,
    {
        self.level = level;
    }

    pub(crate) fn read_level(&self) -> (level: bool)
        ensures
            level == self.level(),
    {
        self.level
    }
}

pub enum TargetRequest {
    Save,
    Restore(u64),
    Resume,
    /// Drive the pin to a level. The sender lends its pin half.
    SetLevel(bool),
    /// Read the pin level. The sender lends its pin half.
    ReadLevel,
}

pub enum TargetReply {
    Saved(u64),
    Done,
    Level(bool),
    Refused,
}

pub struct TargetIds {
    pub unit: Loc,
    pub pin: Loc,
}

pub struct TargetProtocol;

impl RpcProtocol for TargetProtocol {
    type Const = TargetIds;
    type Request = TargetRequest;
    type Reply = TargetReply;
    /// (half of the unit ghost, half of the pin)
    type GhostIn = (Option<FracGhost<u64>>, Option<FracGhost<bool>>);
    type GhostOut = (Option<FracGhost<u64>>, Option<FracGhost<bool>>);

    open spec fn request_ok(
        k: TargetIds,
        req: TargetRequest,
        gin: (Option<FracGhost<u64>>, Option<FracGhost<bool>>),
    ) -> bool {
        match req {
            TargetRequest::Save => gin.0 is None && gin.1 is None,
            TargetRequest::Restore(_) | TargetRequest::Resume => gin.0 matches Some(h) && h.valid(
                k.unit,
                0.5real,
            ) && gin.1 is None,
            TargetRequest::SetLevel(_) | TargetRequest::ReadLevel => gin.0 is None && (
            gin.1 matches Some(p) && p.valid(k.pin, 0.5real)),
        }
    }

    open spec fn reply_ok(
        k: TargetIds,
        req: TargetRequest,
        gin: (Option<FracGhost<u64>>, Option<FracGhost<bool>>),
        rep: TargetReply,
        gout: (Option<FracGhost<u64>>, Option<FracGhost<bool>>),
    ) -> bool {
        match (req, rep) {
            (TargetRequest::Save, TargetReply::Saved(blob)) => gout.0 matches Some(h) && h.valid(
                k.unit,
                0.5real,
            ) && h@ == blob && gout.1 is None,
            (TargetRequest::Restore(blob), TargetReply::Done) => gout.0 matches Some(h) && h.valid(
                k.unit,
                0.5real,
            ) && h@ == blob && gout.1 is None,
            (TargetRequest::Resume, TargetReply::Done) => gout.0 is None && gout.1 is None,
            // The returned pin half carries the new level.
            (TargetRequest::SetLevel(level), TargetReply::Done) => gout.0 is None && (
            gout.1 matches Some(p) && p.valid(k.pin, 0.5real) && p@ == level),
            // The lent pin half comes back unchanged, and the level read is its value.
            (TargetRequest::ReadLevel, TargetReply::Level(level)) => gout == gin && (
            gin.1 matches Some(p) && level == p@),
            (_, TargetReply::Refused) => gout == gin,
            _ => false,
        }
    }
}

#[allow(dead_code)]  // ghost-only fields are erased at runtime
pub struct TargetUnit {
    device: TargetDevice,
    running: bool,
    share: Tracked<FracGhost<u64>>,
    pin: Tracked<FracGhost<bool>>,
    ids: Ghost<TargetIds>,
}

impl TargetUnit {
    pub closed spec fn ids(&self) -> TargetIds {
        self.ids@
    }

    pub closed spec fn inv(&self) -> bool {
        &&& self.share@.id() == self.ids@.unit
        &&& self.share@@ == self.device.regs()
        &&& self.share@.frac() == if self.running { 1real } else { 0.5real }
        // The target always holds half of the pin, tied to its stored level.
        &&& self.pin@.valid(self.ids@.pin, 0.5real)
        &&& self.pin@@ == self.device.level()
    }

    pub fn new(
        device: TargetDevice,
        share: Tracked<FracGhost<u64>>,
        pin: Tracked<FracGhost<bool>>,
    ) -> (result: Self)
        requires
            share@.frac() == 1real,
            share@@ == device.regs(),
            pin@.frac() == 0.5real,
            pin@@ == device.level(),
        ensures
            result.inv(),
            result.ids() == (TargetIds { unit: share@.id(), pin: pin@.id() }),
    {
        let ghost ids = TargetIds { unit: share@.id(), pin: pin@.id() };
        TargetUnit { device, running: true, share, pin, ids: Ghost(ids) }
    }

    #[verifier::exec_allows_no_decreases_clause]
    pub fn serve(self, server: RpcServer<TargetProtocol>)
        requires
            self.inv(),
            server.constant() == self.ids(),
    {
        let mut unit = self;
        loop
            invariant
                unit.inv(),
                server.constant() == unit.ids(),
        {
            let Some((req, gin, responder)) = server.recv() else {
                break;
            };
            unit.handle(req, gin, responder);
        }
    }

    fn handle(
        &mut self,
        req: TargetRequest,
        gin: Tracked<(Option<FracGhost<u64>>, Option<FracGhost<bool>>)>,
        responder: RpcResponder<TargetProtocol>,
    )
        requires
            old(self).inv(),
            TargetProtocol::request_ok(old(self).ids(), req, gin@),
            responder.constant() == old(self).ids(),
            responder.request() == req,
            responder.ghost_in() == gin@,
        ensures
            final(self).inv(),
            final(self).ids() == old(self).ids(),
    {
        let Tracked(gin) = gin;
        match req {
            TargetRequest::Save => {
                if !self.running {
                    responder.respond(TargetReply::Refused, Tracked(gin));
                    return;
                }
                self.running = false;
                let blob = self.device.save();
                let tracked half = self.share.borrow_mut().split();
                responder.respond(TargetReply::Saved(blob), Tracked((Some(half), None)));
            },
            TargetRequest::Restore(blob) => {
                if self.running {
                    responder.respond(TargetReply::Refused, Tracked(gin));
                    return;
                }
                let tracked (caller_half, _) = gin;
                proof {
                    self.share.borrow_mut().combine(caller_half.tracked_unwrap());
                }
                self.device.restore(blob);
                let tracked half;
                proof {
                    self.share.borrow_mut().update(blob);
                    half = self.share.borrow_mut().split();
                }
                responder.respond(TargetReply::Done, Tracked((Some(half), None)));
            },
            TargetRequest::Resume => {
                if self.running {
                    responder.respond(TargetReply::Refused, Tracked(gin));
                    return;
                }
                let tracked (caller_half, _) = gin;
                proof {
                    self.share.borrow_mut().combine(caller_half.tracked_unwrap());
                }
                self.running = true;
                responder.respond(TargetReply::Done, Tracked((None, None)));
            },
            TargetRequest::SetLevel(level) => {
                // Accepted in any state: the pin is kept live, as in OpenVMM.
                let tracked (_, lent) = gin;
                let tracked lent = lent.tracked_unwrap();
                self.device.set_level(level);
                let tracked back;
                proof {
                    // Both halves: the whole pin resource, so the level may change.
                    self.pin.borrow_mut().combine(lent);
                    self.pin.borrow_mut().update(level);
                    back = self.pin.borrow_mut().split();
                }
                responder.respond(TargetReply::Done, Tracked((None, Some(back))));
            },
            TargetRequest::ReadLevel => {
                let level = self.device.read_level();
                proof {
                    // The lent half agrees with ours, which is tied to the level.
                    self.pin.borrow().agree(gin.1.tracked_borrow());
                }
                responder.respond(TargetReply::Level(level), Tracked(gin));
            },
        }
    }
}

// ---------------------------------------------------------------------------
// Source (PIT-like)
// ---------------------------------------------------------------------------

pub struct SourceDevice {
    counter: u64,
}

impl View for SourceDevice {
    type V = u64;

    closed spec fn view(&self) -> u64 {
        self.counter
    }
}

impl SourceDevice {
    /// The interrupt output for a counter value.
    pub open spec fn output_of(counter: u64) -> bool {
        counter % 2 == 1
    }

    pub fn new(counter: u64) -> (result: Self)
        ensures
            result@ == counter,
    {
        SourceDevice { counter }
    }

    pub(crate) fn output(&self) -> (level: bool)
        ensures
            level == Self::output_of(self@),
    {
        self.counter % 2 == 1
    }

    pub(crate) fn run(&mut self, counter: u64)
        ensures
            final(self)@ == counter,
    {
        self.counter = counter;
    }

    pub(crate) fn save(&self) -> (blob: u64)
        ensures
            blob == self@,
    {
        self.counter
    }

    pub(crate) fn restore(&mut self, blob: u64)
        ensures
            final(self)@ == blob,
    {
        self.counter = blob;
    }
}

pub enum SourceRequest {
    Save,
    Restore(u64),
    Resume,
    Run(u64),
}

pub enum SourceReply {
    Saved(u64),
    Done,
    Refused,
    /// The target could not be reached; the ghost sent with the request is
    /// lost and the unit refuses further requests.
    Failed,
}

pub struct SourceIds {
    pub unit: Loc,
    pub pin: Loc,
}

pub struct SourceProtocol;

impl RpcProtocol for SourceProtocol {
    type Const = SourceIds;
    type Request = SourceRequest;
    type Reply = SourceReply;
    /// (half of the unit ghost, half of the pin)
    type GhostIn = Option<(FracGhost<u64>, FracGhost<bool>)>;
    type GhostOut = Option<(FracGhost<u64>, FracGhost<bool>)>;

    open spec fn request_ok(
        k: SourceIds,
        req: SourceRequest,
        gin: Option<(FracGhost<u64>, FracGhost<bool>)>,
    ) -> bool {
        match req {
            SourceRequest::Save | SourceRequest::Run(_) => gin is None,
            SourceRequest::Restore(_) => gin matches Some((h, p)) && h.valid(k.unit, 0.5real)
                && p.valid(k.pin, 0.5real),
            // The caller returns the pair it got from `Save`: the pin level
            // is the output for the unit's state.
            SourceRequest::Resume => gin matches Some((h, p)) && h.valid(k.unit, 0.5real) && p.valid(
                k.pin,
                0.5real,
            ) && p@ == SourceDevice::output_of(h@),
        }
    }

    open spec fn reply_ok(
        k: SourceIds,
        req: SourceRequest,
        gin: Option<(FracGhost<u64>, FracGhost<bool>)>,
        rep: SourceReply,
        gout: Option<(FracGhost<u64>, FracGhost<bool>)>,
    ) -> bool {
        match (req, rep) {
            // The caller gets the unit half (the counter) and the pin half
            // (the output for that counter).
            (SourceRequest::Save, SourceReply::Saved(blob)) => gout matches Some((h, p))
                && h.valid(k.unit, 0.5real) && h@ == blob && p.valid(k.pin, 0.5real) && p@
                == SourceDevice::output_of(blob),
            (SourceRequest::Restore(blob), SourceReply::Done) => gout matches Some((h, p))
                && h.valid(k.unit, 0.5real) && h@ == blob && p.valid(k.pin, 0.5real) && p@
                == SourceDevice::output_of(blob),
            (SourceRequest::Resume, SourceReply::Done) => gout is None,
            (SourceRequest::Run(_), SourceReply::Done) => gout is None,
            (_, SourceReply::Refused) => gout == gin,
            (_, SourceReply::Failed) => gout is None,
            _ => false,
        }
    }
}

#[allow(dead_code)]  // ghost-only fields are erased at runtime
pub struct SourceUnit {
    device: SourceDevice,
    running: bool,
    /// False once a call to the target has failed and its pin half was lost.
    healthy: bool,
    share: Tracked<FracGhost<u64>>,
    /// The source's pin half: present while running and healthy; with the
    /// caller while stopped.
    pin: Tracked<Option<FracGhost<bool>>>,
    ids: Ghost<SourceIds>,
    target: RpcClient<TargetProtocol>,
}

impl SourceUnit {
    pub closed spec fn ids(&self) -> SourceIds {
        self.ids@
    }

    pub closed spec fn inv(&self) -> bool {
        &&& self.share@.id() == self.ids@.unit
        &&& self.share@@ == self.device@
        &&& self.share@.frac() == if self.running { 1real } else { 0.5real }
        &&& self.target.constant().pin == self.ids@.pin
        &&& if self.running && self.healthy {
            self.pin@ matches Some(p) && p.valid(self.ids@.pin, 0.5real) && p@
                == SourceDevice::output_of(self.device@)
        } else {
            self.pin@ is None
        }
    }

    pub fn new(
        device: SourceDevice,
        share: Tracked<FracGhost<u64>>,
        pin: Tracked<FracGhost<bool>>,
        target: RpcClient<TargetProtocol>,
    ) -> (result: Self)
        requires
            share@.frac() == 1real,
            share@@ == device@,
            pin@.frac() == 0.5real,
            pin@@ == SourceDevice::output_of(device@),
            target.constant().pin == pin@.id(),
        ensures
            result.inv(),
            result.ids() == (SourceIds { unit: share@.id(), pin: pin@.id() }),
    {
        let ghost ids = SourceIds { unit: share@.id(), pin: pin@.id() };
        let Tracked(pin) = pin;
        SourceUnit {
            device,
            running: true,
            healthy: true,
            share,
            pin: Tracked(Some(pin)),
            ids: Ghost(ids),
            target,
        }
    }

    #[verifier::exec_allows_no_decreases_clause]
    pub fn serve(self, server: RpcServer<SourceProtocol>)
        requires
            self.inv(),
            server.constant() == self.ids(),
    {
        let mut unit = self;
        loop
            invariant
                unit.inv(),
                server.constant() == unit.ids(),
        {
            let Some((req, gin, responder)) = server.recv() else {
                break;
            };
            unit.handle(req, gin, responder);
        }
    }

    /// Drives the pin to the current output, lending `lent` to the target.
    /// On success returns `true` and the pin half back, carrying the new
    /// level; returns `false` if the target could not be reached.
    fn drive_pin(&self, lent: Tracked<FracGhost<bool>>) -> (result: (
        bool,
        Tracked<Option<FracGhost<bool>>>,
    ))
        requires
            self.target.constant().pin == self.ids().pin,
            lent@.valid(self.ids().pin, 0.5real),
        ensures
            result.0 ==> (result.1@ matches Some(p) && p.valid(self.ids().pin, 0.5real) && p@
                == SourceDevice::output_of(self.device@)),
            !result.0 ==> result.1@ is None,
    {
        let level = self.device.output();
        let Tracked(lent) = lent;
        let result = self.target.call(TargetRequest::SetLevel(level), Tracked((None, Some(lent))));
        if let Ok((TargetReply::Done, gout)) = result {
            let Tracked(gout) = gout;
            let tracked (_, back) = gout;
            (true, Tracked(back))
        } else {
            (false, Tracked(None))
        }
    }

    fn handle(
        &mut self,
        req: SourceRequest,
        gin: Tracked<Option<(FracGhost<u64>, FracGhost<bool>)>>,
        responder: RpcResponder<SourceProtocol>,
    )
        requires
            old(self).inv(),
            SourceProtocol::request_ok(old(self).ids(), req, gin@),
            responder.constant() == old(self).ids(),
            responder.request() == req,
            responder.ghost_in() == gin@,
        ensures
            final(self).inv(),
            final(self).ids() == old(self).ids(),
    {
        let Tracked(gin) = gin;
        if !self.healthy {
            responder.respond(SourceReply::Refused, Tracked(gin));
            return;
        }
        match req {
            SourceRequest::Save => {
                if !self.running {
                    responder.respond(SourceReply::Refused, Tracked(gin));
                    return;
                }
                self.running = false;
                let blob = self.device.save();
                let tracked half = self.share.borrow_mut().split();
                let tracked pin = self.pin.borrow_mut().tracked_take();
                responder.respond(SourceReply::Saved(blob), Tracked(Some((half, pin))));
            },
            SourceRequest::Restore(blob) => {
                if self.running {
                    responder.respond(SourceReply::Refused, Tracked(gin));
                    return;
                }
                let tracked (caller_half, caller_pin) = gin.tracked_unwrap();
                proof {
                    self.share.borrow_mut().combine(caller_half);
                }
                self.device.restore(blob);
                proof {
                    self.share.borrow_mut().update(blob);
                }
                // Re-drive the pin for the restored counter.
                let (ok, Tracked(back)) = self.drive_pin(Tracked(caller_pin));
                let tracked half = self.share.borrow_mut().split();
                if !ok {
                    self.healthy = false;
                    responder.respond(SourceReply::Failed, Tracked(None));
                    return;
                }
                let tracked back = back.tracked_unwrap();
                responder.respond(SourceReply::Done, Tracked(Some((half, back))));
            },
            SourceRequest::Resume => {
                if self.running {
                    responder.respond(SourceReply::Refused, Tracked(gin));
                    return;
                }
                let tracked (caller_half, caller_pin) = gin.tracked_unwrap();
                proof {
                    self.share.borrow_mut().combine(caller_half);
                }
                self.pin = Tracked(Some(caller_pin));
                self.running = true;
                responder.respond(SourceReply::Done, Tracked(None));
            },
            SourceRequest::Run(counter) => {
                if !self.running {
                    responder.respond(SourceReply::Refused, Tracked(gin));
                    return;
                }
                self.device.run(counter);
                proof {
                    self.share.borrow_mut().update(counter);
                }
                let tracked mine = self.pin.borrow_mut().tracked_take();
                let (ok, Tracked(back)) = self.drive_pin(Tracked(mine));
                if !ok {
                    self.healthy = false;
                    self.pin = Tracked(None);
                    responder.respond(SourceReply::Failed, Tracked(None));
                    return;
                }
                self.pin = Tracked(back);
                responder.respond(SourceReply::Done, Tracked(None));
            },
        }
    }
}

// ---------------------------------------------------------------------------
// Caller (LoadedVm-like)
// ---------------------------------------------------------------------------

/// Saves both units, runs the guest further, then restores both snapshots
/// (target first, then source, following the dependency order) and reads the
/// target's pin level. On success, that level is the output of the restored
/// source counter.
pub fn two_unit_round_trip() -> (result: Option<(u64, u64, bool)>)
    ensures
        match result {
            Some((_target_snapshot, source_snapshot, level)) => level == SourceDevice::output_of(
                source_snapshot,
            ),
            None => true,
        },
{
    // The pin starts low; one half to the target, one to the source.
    let tracked mut pin_target = FracGhost::<bool>::new(false);
    let tracked pin_source = pin_target.split();

    let tracked target_share = FracGhost::<u64>::new(1);
    let target_unit = TargetUnit::new(
        TargetDevice::new(1, false),
        Tracked(target_share),
        Tracked(pin_target),
    );
    let ghost target_ids = target_unit.ids();
    let (target_client, target_server) = rpc_channel::<TargetProtocol>(Ghost(target_ids));

    // The source drives the pin through its own client to the target.
    let tracked source_share = FracGhost::<u64>::new(0);
    let source_unit = SourceUnit::new(
        SourceDevice::new(0),
        Tracked(source_share),
        Tracked(pin_source),
        target_client.clone(),
    );
    let ghost source_ids = source_unit.ids();
    let (source_client, source_server) = rpc_channel::<SourceProtocol>(Ghost(source_ids));

    let _target_task = vstd::thread::spawn(
        move ||
            requires
                target_unit.ids() == target_ids,
                target_server.constant() == target_ids,
            {
                target_unit.serve(target_server);
            },
    );
    let _source_task = vstd::thread::spawn(
        move ||
            requires
                source_unit.ids() == source_ids,
                source_server.constant() == source_ids,
            {
                source_unit.serve(source_server);
            },
    );

    // Guest activity: the counter becomes 5, so the pin goes high.
    let Ok((SourceReply::Done, _)) = source_client.call(SourceRequest::Run(5), Tracked(None)) else {
        return None;
    };

    // Save both units.
    let Ok((TargetReply::Saved(target_snapshot), Tracked(t))) = target_client.call(
        TargetRequest::Save,
        Tracked((None, None)),
    ) else {
        return None;
    };
    let Ok((SourceReply::Saved(source_snapshot), Tracked(s))) = source_client.call(
        SourceRequest::Save,
        Tracked(None),
    ) else {
        return None;
    };

    // Resume both, and run the guest further: the counter becomes 4, the pin low.
    let tracked (t_half, _) = t;
    let Ok((TargetReply::Done, _)) = target_client.call(
        TargetRequest::Resume,
        Tracked((Some(t_half.tracked_unwrap()), None)),
    ) else {
        return None;
    };
    let Ok((SourceReply::Done, _)) = source_client.call(SourceRequest::Resume, Tracked(s)) else {
        return None;
    };
    let Ok((SourceReply::Done, _)) = source_client.call(SourceRequest::Run(4), Tracked(None)) else {
        return None;
    };

    // Stop both again (saving the current state, which we discard).
    let Ok((TargetReply::Saved(_), Tracked(t))) = target_client.call(
        TargetRequest::Save,
        Tracked((None, None)),
    ) else {
        return None;
    };
    let Ok((SourceReply::Saved(_), Tracked(s))) = source_client.call(
        SourceRequest::Save,
        Tracked(None),
    ) else {
        return None;
    };

    // Restore the first snapshots: target first, then source.
    let tracked (t_half, _) = t;
    let Ok((TargetReply::Done, _)) = target_client.call(
        TargetRequest::Restore(target_snapshot),
        Tracked((Some(t_half.tracked_unwrap()), None)),
    ) else {
        return None;
    };
    let Ok((SourceReply::Done, Tracked(s))) = source_client.call(
        SourceRequest::Restore(source_snapshot),
        Tracked(s),
    ) else {
        return None;
    };

    // Read the target's level, lending the pin half we now hold.
    let tracked (_, pin_half) = s.tracked_unwrap();
    let Ok((TargetReply::Level(level), _)) = target_client.call(
        TargetRequest::ReadLevel,
        Tracked((None, Some(pin_half))),
    ) else {
        return None;
    };
    Some((target_snapshot, source_snapshot, level))
}

} // verus!
