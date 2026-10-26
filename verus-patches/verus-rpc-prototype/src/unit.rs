//! A state unit: a device owned by its own task, reachable only by RPC.
//!
//! The unit's task holds a `FracGhost` whose value is the device's abstract
//! state. While running it holds the whole permission and may change the
//! device. On save it hands half to the caller, which then knows the device
//! state and that it cannot change. On restore the caller hands that half
//! back.
//!
//! Everything in this file is verified.

use crate::rpc_t::*;
use vstd::prelude::*;
use vstd::resource::Loc;
use vstd::resource::frac::FracGhost;

verus! {

/// A toy device with one register.
pub struct Device {
    value: u64,
}

impl View for Device {
    type V = u64;

    closed spec fn view(&self) -> u64 {
        self.value
    }
}

impl Device {
    pub fn new(value: u64) -> (result: Self)
        ensures
            result@ == value,
    {
        Device { value }
    }

    /// Guest activity changes the register.
    pub(crate) fn run(&mut self, value: u64)
        ensures
            final(self)@ == value,
    {
        self.value = value;
    }

    pub(crate) fn save(&self) -> (blob: u64)
        ensures
            blob == self@,
    {
        self.value
    }

    pub(crate) fn restore(&mut self, blob: u64)
        ensures
            final(self)@ == blob,
    {
        self.value = blob;
    }
}

pub enum UnitRequest {
    /// Stop and save. The unit returns half of its ghost.
    Save,
    /// Restore `blob`. The caller sends its half; the unit returns a half.
    Restore(u64),
    /// Resume running. The caller sends its half back.
    Resume,
    /// Simulated guest activity; only allowed while running.
    Run(u64),
}

pub enum UnitReply {
    Saved(u64),
    Done,
    /// The request is not valid in the unit's current state. Any ghost sent
    /// with the request is returned unchanged.
    Refused,
}

pub struct UnitProtocol;

impl RpcProtocol for UnitProtocol {
    /// The id of the unit's ghost.
    type Const = Loc;
    type Request = UnitRequest;
    type Reply = UnitReply;
    type GhostIn = Option<FracGhost<u64>>;
    type GhostOut = Option<FracGhost<u64>>;

    open spec fn request_ok(k: Loc, req: UnitRequest, gin: Option<FracGhost<u64>>) -> bool {
        match req {
            UnitRequest::Save | UnitRequest::Run(_) => gin is None,
            UnitRequest::Restore(_) | UnitRequest::Resume => gin matches Some(h) && h.valid(k, 0.5real),
        }
    }

    open spec fn reply_ok(
        k: Loc,
        req: UnitRequest,
        gin: Option<FracGhost<u64>>,
        rep: UnitReply,
        gout: Option<FracGhost<u64>>,
    ) -> bool {
        match (req, rep) {
            // The returned half's value is the saved device state.
            (UnitRequest::Save, UnitReply::Saved(blob)) => gout matches Some(h) && h.valid(k, 0.5real)
                && h@ == blob,
            // The returned half's value is the restored state.
            (UnitRequest::Restore(blob), UnitReply::Done) => gout matches Some(h) && h.valid(k, 0.5real)
                && h@ == blob,
            (UnitRequest::Resume, UnitReply::Done) => gout is None,
            (UnitRequest::Run(_), UnitReply::Done) => gout is None,
            (_, UnitReply::Refused) => gout == gin,
            _ => false,
        }
    }
}

/// The unit's private state: the device and the unit's share of the ghost.
#[allow(dead_code)]  // `share` and `id` are ghost-only and erased at runtime
pub struct Unit {
    device: Device,
    running: bool,
    share: Tracked<FracGhost<u64>>,
    id: Ghost<Loc>,
}

impl Unit {
    /// The id of the unit's ghost.
    pub closed spec fn id(&self) -> Loc {
        self.id@
    }

    pub closed spec fn inv(&self) -> bool {
        &&& self.share@.id() == self.id@
        &&& self.share@@ == self.device@
        // While running the unit holds the whole permission; while stopped it
        // holds half, and the other half is with the caller.
        &&& self.share@.frac() == if self.running { 1real } else { 0.5real }
    }

    /// A running unit owning `device` and the whole of `ghost`.
    pub fn new(device: Device, ghost: Tracked<FracGhost<u64>>) -> (result: Self)
        requires
            ghost@.frac() == 1real,
            ghost@@ == device@,
        ensures
            result.inv(),
            result.id() == ghost@.id(),
    {
        Unit { device, running: true, id: Ghost(ghost@.id()), share: ghost }
    }

    /// The unit's task: serves requests until every client is gone. It runs
    /// for as long as clients exist, so it has no termination measure.
    #[verifier::exec_allows_no_decreases_clause]
    pub fn serve(self, server: RpcServer<UnitProtocol>)
        requires
            self.inv(),
            server.constant() == self.id(),
    {
        let mut unit = self;
        loop
            invariant
                unit.inv(),
                server.constant() == unit.id(),
        {
            let Some((req, gin, responder)) = server.recv() else {
                break;
            };
            unit.handle(req, gin, responder);
        }
    }

    fn handle(
        &mut self,
        req: UnitRequest,
        gin: Tracked<Option<FracGhost<u64>>>,
        responder: RpcResponder<UnitProtocol>,
    )
        requires
            old(self).inv(),
            UnitProtocol::request_ok(old(self).id@, req, gin@),
            responder.constant() == old(self).id@,
            responder.request() == req,
            responder.ghost_in() == gin@,
        ensures
            final(self).inv(),
            final(self).id@ == old(self).id@,
    {
        let Tracked(gin) = gin;
        match req {
            UnitRequest::Save => {
                if !self.running {
                    responder.respond(UnitReply::Refused, Tracked(gin));
                    return;
                }
                self.running = false;
                let blob = self.device.save();
                let tracked half = self.share.borrow_mut().split();
                responder.respond(UnitReply::Saved(blob), Tracked(Some(half)));
            },
            UnitRequest::Restore(blob) => {
                if self.running {
                    responder.respond(UnitReply::Refused, Tracked(gin));
                    return;
                }
                let tracked caller_half = gin.tracked_unwrap();
                proof {
                    // Recombine: now the unit holds the whole permission.
                    self.share.borrow_mut().combine(caller_half);
                }
                self.device.restore(blob);
                let tracked half;
                proof {
                    self.share.borrow_mut().update(blob);
                    half = self.share.borrow_mut().split();
                }
                responder.respond(UnitReply::Done, Tracked(Some(half)));
            },
            UnitRequest::Resume => {
                if self.running {
                    responder.respond(UnitReply::Refused, Tracked(gin));
                    return;
                }
                let tracked caller_half = gin.tracked_unwrap();
                proof {
                    self.share.borrow_mut().combine(caller_half);
                }
                self.running = true;
                responder.respond(UnitReply::Done, Tracked(None));
            },
            UnitRequest::Run(value) => {
                if !self.running {
                    responder.respond(UnitReply::Refused, Tracked(gin));
                    return;
                }
                self.device.run(value);
                proof {
                    self.share.borrow_mut().update(value);
                }
                responder.respond(UnitReply::Done, Tracked(None));
            },
        }
    }
}

} // verus!
