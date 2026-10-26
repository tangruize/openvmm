//! A stopped state unit served over a real mesh channel, using the trusted
//! spec in `mesh_rpc_t.rs`.
//!
//! The caller holds half of the unit's `FracGhost` (the unit is stopped; how
//! the half got there, the hand-off at stop, is not modelled here). The unit
//! holds the other half, tied to its device.
//!
//! - `Save`: the caller lends its half; the unit returns it with a blob equal
//!   to its value. The caller learns the blob is the device state.
//! - `Restore(blob)`: the caller sends its half; the unit recombines, restores
//!   the device, sets the value to `blob`, and returns a half with that value.
//!
//! Everything in this file is verified.

use crate::mesh_rpc_t::*;
use crate::unit::Device;
use mesh::rpc::Rpc;
use vstd::prelude::*;
use vstd::resource::Loc;
use vstd::resource::frac::FracGhost;

verus! {

/// Requests, in the style of OpenVMM's `StateRequest`.
pub enum Req {
    Save(Rpc<(), u64>),
    Restore(Rpc<u64, ()>),
}

impl MeshProtocol for Req {
    /// The id of the unit's ghost.
    type Const = Loc;
    type GhostIn = FracGhost<u64>;
    type GhostOut = FracGhost<u64>;

    open spec fn request_ok(k: Loc, msg: Req, gin: FracGhost<u64>) -> bool {
        // Every request carries the caller's half.
        &&& gin.valid(k, 0.5real)
        &&& match msg {
            // Reply: the same half back, and a blob equal to its value.
            Req::Save(rpc) => forall|blob: u64, gout: FracGhost<u64>|
                gout.valid(k, 0.5real) && gout@ == gin@ && blob == gin@
                    ==> #[trigger] rpc.reply_ok(blob, gout),
            // Reply: a half whose value is the restored blob.
            Req::Restore(rpc) => forall|r: (), gout: FracGhost<u64>|
                gout.valid(k, 0.5real) && gout@ == rpc.spec_input()
                    ==> #[trigger] rpc.reply_ok(r, gout),
        }
    }
}

#[allow(dead_code)]  // ghost-only fields are erased at runtime
pub struct StoppedUnit {
    device: Device,
    share: Tracked<FracGhost<u64>>,
    id: Ghost<Loc>,
}

impl StoppedUnit {
    pub closed spec fn id(&self) -> Loc {
        self.id@
    }

    pub closed spec fn inv(&self) -> bool {
        &&& self.share@.valid(self.id@, 0.5real)
        &&& self.share@@ == self.device@
    }

    pub fn new(device: Device, share: Tracked<FracGhost<u64>>) -> (result: Self)
        requires
            share@.frac() == 0.5real,
            share@@ == device@,
        ensures
            result.inv(),
            result.id() == share@.id(),
    {
        StoppedUnit { device, id: Ghost(share@.id()), share }
    }

    #[verifier::exec_allows_no_decreases_clause]
    pub async fn serve(self, recv_end: mesh::Receiver<Req>)
        requires
            self.inv(),
            recv_end.constant() == self.id(),
    {
        let mut unit = self;
        let mut recv_end = recv_end;
        loop
            invariant
                unit.inv(),
                recv_end.constant() == unit.id(),
        {
            let Some((msg, gin)) = recv(&mut recv_end).await else {
                break;
            };
            unit.handle(msg, gin);
        }
    }

    fn handle(&mut self, msg: Req, gin: Tracked<FracGhost<u64>>)
        requires
            old(self).inv(),
            Req::request_ok(old(self).id(), msg, gin@),
        ensures
            final(self).inv(),
            final(self).id() == old(self).id(),
    {
        let Tracked(lent) = gin;
        match msg {
            Req::Save(rpc) => {
                proof {
                    // The lent half agrees with ours, which is tied to the device.
                    self.share.borrow().agree(&lent);
                }
                let blob = self.device.save();
                complete(rpc, blob, Tracked(lent));
            },
            Req::Restore(rpc) => {
                let (blob, rpc) = split::<u64, (), FracGhost<u64>>(rpc);
                proof {
                    self.share.borrow_mut().combine(lent);
                }
                self.device.restore(blob);
                let tracked half;
                proof {
                    self.share.borrow_mut().update(blob);
                    half = self.share.borrow_mut().split();
                }
                complete(rpc, (), Tracked(half));
            },
        }
    }
}

/// Saves the unit, restores a different value, then restores the saved blob.
/// On success, the caller's half (the unit's state) is the saved value again.
/// Returns the saved blob and the caller's half after the final restore.
pub async fn round_trip() -> (result: Option<(u64, Tracked<FracGhost<u64>>)>)
    ensures
        result matches Some((saved, h)) ==> saved == 3 && h@@ == saved,
{
    let tracked mut caller_half = FracGhost::<u64>::new(3);
    let tracked unit_half = caller_half.split();
    let unit = StoppedUnit::new(Device::new(3), Tracked(unit_half));
    let ghost k = unit.id();
    let (send, recv_end) = channel::<Req>(Ghost(k));
    let _task = vstd::thread::spawn(
        move ||
            requires
                unit.inv(),
                unit.id() == k,
                recv_end.constant() == k,
            {
                crate::rpc_async_t::block_on(unit.serve(recv_end));
            },
    );

    // Save: lend our half, get it back with the blob.
    let ghost v = caller_half@;
    let Ok((saved, Tracked(h))) = call(
        &send,
        |rpc: Rpc<(), u64>| -> (msg: Req) ensures msg == Req::Save(rpc) { Req::Save(rpc) },
        (),
        Tracked(caller_half),
        Ghost(|blob: u64, gout: FracGhost<u64>| gout.valid(k, 0.5real) && gout@ == v && blob == v),
    ).recv().await else {
        return None;
    };
    assert(saved == 3);

    // Restore some other state.
    let Ok(((), Tracked(h))) = call(
        &send,
        |rpc: Rpc<u64, ()>| -> (msg: Req) ensures msg == Req::Restore(rpc) { Req::Restore(rpc) },
        7,
        Tracked(h),
        Ghost(|r: (), gout: FracGhost<u64>| gout.valid(k, 0.5real) && gout@ == 7),
    ).recv().await else {
        return None;
    };
    assert(h@ == 7);

    // Restore the saved blob.
    let Ok(((), Tracked(h))) = call(
        &send,
        |rpc: Rpc<u64, ()>| -> (msg: Req) ensures msg == Req::Restore(rpc) { Req::Restore(rpc) },
        saved,
        Tracked(h),
        Ghost(|r: (), gout: FracGhost<u64>| gout.valid(k, 0.5real) && gout@ == saved),
    ).recv().await else {
        return None;
    };
    // The unit's state is the saved state again.
    assert(h@ == saved);
    Some((saved, Tracked(h)))
}

} // verus!
