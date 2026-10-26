//! Async version of `two_units.rs`: the same units, protocols, and caller,
//! with the units as async tasks served over `rpc_async_t.rs`. Tracked ghost
//! halves are held across `.await` points: in the caller, and in the source's
//! `drive_pin`, which lends its pin half to the target mid-request.
//!
//! Everything in this file is verified.

#[allow(unused_imports)]  // used only in specs
use crate::rpc_t::RpcProtocol;
use crate::rpc_async_t::*;
use crate::two_units::*;
use vstd::prelude::*;
use vstd::resource::frac::FracGhost;

verus! {

#[allow(dead_code)]  // ghost-only fields are erased at runtime
pub struct AsyncTargetUnit {
    device: TargetDevice,
    running: bool,
    share: Tracked<FracGhost<u64>>,
    pin: Tracked<FracGhost<bool>>,
    ids: Ghost<TargetIds>,
}

impl AsyncTargetUnit {
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
        AsyncTargetUnit { device, running: true, share, pin, ids: Ghost(ids) }
    }

    #[verifier::exec_allows_no_decreases_clause]
    pub async fn serve(self, server: AsyncRpcServer<TargetProtocol>)
        requires
            self.inv(),
            server.constant() == self.ids(),
    {
        let mut unit = self;
        let mut server = server;
        loop
            invariant
                unit.inv(),
                server.constant() == unit.ids(),
        {
            let Some((req, gin, responder)) = server.recv().await else {
                break;
            };
            unit.handle(req, gin, responder).await;
        }
    }

    async fn handle(
        &mut self,
        req: TargetRequest,
        gin: Tracked<(Option<FracGhost<u64>>, Option<FracGhost<bool>>)>,
        responder: AsyncRpcResponder<TargetProtocol>,
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

#[allow(dead_code)]  // ghost-only fields are erased at runtime
pub struct AsyncSourceUnit {
    device: SourceDevice,
    running: bool,
    /// False once a call to the target has failed and its pin half was lost.
    healthy: bool,
    share: Tracked<FracGhost<u64>>,
    /// The source's pin half: present while running and healthy; with the
    /// caller while stopped.
    pin: Tracked<Option<FracGhost<bool>>>,
    ids: Ghost<SourceIds>,
    target: AsyncRpcClient<TargetProtocol>,
}

impl AsyncSourceUnit {
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
        target: AsyncRpcClient<TargetProtocol>,
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
        AsyncSourceUnit {
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
    pub async fn serve(self, server: AsyncRpcServer<SourceProtocol>)
        requires
            self.inv(),
            server.constant() == self.ids(),
    {
        let mut unit = self;
        let mut server = server;
        loop
            invariant
                unit.inv(),
                server.constant() == unit.ids(),
        {
            let Some((req, gin, responder)) = server.recv().await else {
                break;
            };
            unit.handle(req, gin, responder).await;
        }
    }

    /// Drives the pin to the current output, lending `lent` to the target.
    /// On success returns `true` and the pin half back, carrying the new
    /// level; returns `false` if the target could not be reached.
    async fn drive_pin(&self, lent: Tracked<FracGhost<bool>>) -> (result: (
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
        let result = self.target.call(TargetRequest::SetLevel(level), Tracked((None, Some(lent)))).await;
        if let Ok((TargetReply::Done, gout)) = result {
            let Tracked(gout) = gout;
            let tracked (_, back) = gout;
            (true, Tracked(back))
        } else {
            (false, Tracked(None))
        }
    }

    async fn handle(
        &mut self,
        req: SourceRequest,
        gin: Tracked<Option<(FracGhost<u64>, FracGhost<bool>)>>,
        responder: AsyncRpcResponder<SourceProtocol>,
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
                let (ok, Tracked(back)) = self.drive_pin(Tracked(caller_pin)).await;
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
                let (ok, Tracked(back)) = self.drive_pin(Tracked(mine)).await;
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

/// Saves both units, runs the guest further, then restores both snapshots
/// (target first, then source, following the dependency order) and reads the
/// target's pin level. On success, that level is the output of the restored
/// source counter.
pub async fn two_unit_round_trip() -> (result: Option<(u64, u64, bool)>)
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
    let target_unit = AsyncTargetUnit::new(
        TargetDevice::new(1, false),
        Tracked(target_share),
        Tracked(pin_target),
    );
    let ghost target_ids = target_unit.ids();
    let (target_client, target_server) = async_rpc_channel::<TargetProtocol>(Ghost(target_ids));

    // The source drives the pin through its own client to the target.
    let tracked source_share = FracGhost::<u64>::new(0);
    let source_unit = AsyncSourceUnit::new(
        SourceDevice::new(0),
        Tracked(source_share),
        Tracked(pin_source),
        target_client.clone(),
    );
    let ghost source_ids = source_unit.ids();
    let (source_client, source_server) = async_rpc_channel::<SourceProtocol>(Ghost(source_ids));

    let _target_task = vstd::thread::spawn(
        move ||
            requires
                target_unit.ids() == target_ids,
                target_server.constant() == target_ids,
            {
                block_on(target_unit.serve(target_server));
            },
    );
    let _source_task = vstd::thread::spawn(
        move ||
            requires
                source_unit.ids() == source_ids,
                source_server.constant() == source_ids,
            {
                block_on(source_unit.serve(source_server));
            },
    );

    // Guest activity: the counter becomes 5, so the pin goes high.
    let Ok((SourceReply::Done, _)) = source_client.call(SourceRequest::Run(5), Tracked(None)).await else {
        return None;
    };

    // Save both units.
    let Ok((TargetReply::Saved(target_snapshot), Tracked(t))) = target_client.call(
        TargetRequest::Save,
        Tracked((None, None)),
    ).await else {
        return None;
    };
    let Ok((SourceReply::Saved(source_snapshot), Tracked(s))) = source_client.call(
        SourceRequest::Save,
        Tracked(None),
    ).await else {
        return None;
    };

    // Resume both, and run the guest further: the counter becomes 4, the pin low.
    let tracked (t_half, _) = t;
    let Ok((TargetReply::Done, _)) = target_client.call(
        TargetRequest::Resume,
        Tracked((Some(t_half.tracked_unwrap()), None)),
    ).await else {
        return None;
    };
    let Ok((SourceReply::Done, _)) = source_client.call(SourceRequest::Resume, Tracked(s)).await else {
        return None;
    };
    let Ok((SourceReply::Done, _)) = source_client.call(SourceRequest::Run(4), Tracked(None)).await else {
        return None;
    };

    // Stop both again (saving the current state, which we discard).
    let Ok((TargetReply::Saved(_), Tracked(t))) = target_client.call(
        TargetRequest::Save,
        Tracked((None, None)),
    ).await else {
        return None;
    };
    let Ok((SourceReply::Saved(_), Tracked(s))) = source_client.call(
        SourceRequest::Save,
        Tracked(None),
    ).await else {
        return None;
    };

    // Restore the first snapshots: target first, then source.
    let tracked (t_half, _) = t;
    let Ok((TargetReply::Done, _)) = target_client.call(
        TargetRequest::Restore(target_snapshot),
        Tracked((Some(t_half.tracked_unwrap()), None)),
    ).await else {
        return None;
    };
    let Ok((SourceReply::Done, Tracked(s))) = source_client.call(
        SourceRequest::Restore(source_snapshot),
        Tracked(s),
    ).await else {
        return None;
    };

    // Read the target's level, lending the pin half we now hold.
    let tracked (_, pin_half) = s.tracked_unwrap();
    let Ok((TargetReply::Level(level), _)) = target_client.call(
        TargetRequest::ReadLevel,
        Tracked((None, Some(pin_half))),
    ).await else {
        return None;
    };
    Some((target_snapshot, source_snapshot, level))
}

/// Runs the async round trip to completion.
pub fn run_two_unit_round_trip() -> (result: Option<(u64, u64, bool)>)
    ensures
        match result {
            Some((_target_snapshot, source_snapshot, level)) => level == SourceDevice::output_of(
                source_snapshot,
            ),
            None => true,
        },
{
    block_on(two_unit_round_trip())
}

} // verus!
