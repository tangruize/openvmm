//! The caller, in the role of `LoadedVm`: it never holds the device, only an
//! RPC client, yet proves that restoring a snapshot brings back the saved
//! device state.
//!
//! Everything in this file is verified.

use crate::rpc::*;
use crate::unit::*;
use vstd::prelude::*;
use vstd::resource::frac::FracGhost;

verus! {

/// Runs one save / resume / run / save / restore sequence against a unit and
/// returns the saved blob together with the caller's half after restore.
/// On success, the half's value (the unit's state) equals the first snapshot.
pub fn round_trip() -> (result: Option<(u64, Tracked<FracGhost<u64>>)>)
    ensures
        match result {
            Some((first_snapshot, half)) => half@@ == first_snapshot,
            None => true,
        },
{
    // Set up the unit: a device, and the whole ghost permission describing it.
    let device = Device::new(7);
    let tracked share = FracGhost::<u64>::new(7);
    let ghost id = share.id();
    let (client, server) = rpc_channel::<UnitProtocol>(Ghost(id));
    let unit = Unit::new(device, Tracked(share));
    let _task = vstd::thread::spawn(
        move ||
            requires
                unit.id() == id,
                server.constant() == id,
            {
                unit.serve(server);
            },
    );

    // Guest activity while running.
    let Ok((UnitReply::Done, _)) = client.call(UnitRequest::Run(3), Tracked(None)) else {
        return None;
    };

    // Save: the caller receives half of the unit's ghost.
    let Ok((UnitReply::Saved(first_snapshot), Tracked(half))) = client.call(
        UnitRequest::Save,
        Tracked(None),
    ) else {
        return None;
    };
    let tracked half = half.tracked_unwrap();
    // Holding this half, the caller knows the unit's state is `first_snapshot`.
    assert(half@ == first_snapshot);

    // Resume and change the device again: the caller must give its half back.
    let Ok((UnitReply::Done, _)) = client.call(UnitRequest::Resume, Tracked(Some(half))) else {
        return None;
    };
    let Ok((UnitReply::Done, _)) = client.call(UnitRequest::Run(9), Tracked(None)) else {
        return None;
    };

    // Save again (state 9), then restore the first snapshot.
    let Ok((UnitReply::Saved(_), Tracked(half))) = client.call(UnitRequest::Save, Tracked(None))
    else {
        return None;
    };
    let tracked half = half.tracked_unwrap();
    let Ok((UnitReply::Done, Tracked(half))) = client.call(
        UnitRequest::Restore(first_snapshot),
        Tracked(Some(half)),
    ) else {
        return None;
    };
    let tracked half = half.tracked_unwrap();
    // The unit's state is back to the first snapshot.
    assert(half@ == first_snapshot);
    Some((first_snapshot, Tracked(half)))
}

} // verus!
