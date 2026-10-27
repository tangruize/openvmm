// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

// High-level spec of a device's ghost state, for human review.

use crate::save_restore::RestoreError;
use crate::save_restore::SaveError;
use crate::save_restore::SavedStateBlob;
use crate::save_restore::spec::UnitSnapshot;
use closeable_mutex::LockInvariant;
use vstd::prelude::*;
use vstd::resource::Loc;
use vstd::resource::frac::FracGhost;

verus! {

/// A device's ghost fields: its share of its state unit's FracGhost, and
/// whether it runs. A device keeps one of these as a ghost field (see
/// "Design change: the FracGhost lives with the device" in
/// `SNAPSHOT-SPEC-DESIGN.md`).
pub tracked struct DeviceGhostState {
    /// The device's share: the whole while running, one half while stopped.
    pub tracked frac: FracGhost<UnitSnapshot>,
    /// Whether the device is running.
    pub ghost running: bool,
}

impl DeviceGhostState {
    /// The rule, the same for every device, for a device whose FracGhost has
    /// id `id` and whose state is `snapshot`: the device holds the whole or
    /// one half of that FracGhost, the whole while running; and the
    /// FracGhost, unless still `Unknown`, records `snapshot`.
    pub open spec fn wf(self, id: Loc, snapshot: UnitSnapshot) -> bool {
        &&& self.frac.id() == id
        &&& (self.frac.frac() == 1real || self.frac.frac() == 0.5real)
        &&& self.running ==> self.whole()
        &&& !(self.frac@ is Unknown) ==> self.frac@ == snapshot
    }

    /// The device holds the whole FracGhost.
    pub open spec fn whole(self) -> bool {
        self.frac.frac() == 1real
    }
}

/// A device that keeps a `DeviceGhostState` as a ghost field. Lets generic
/// code (the device's state unit) reach that field, to change it in its own
/// proof blocks.
pub trait DeviceGhost {
    /// The device's ghost field.
    spec fn ghost(&self) -> DeviceGhostState;

    /// The id of the device's FracGhost (its unit channel's constant).
    spec fn id(&self) -> Loc;

    /// The device's state, as its FracGhost records it.
    spec fn snapshot(&self) -> UnitSnapshot;

    /// The device's own invariant, on its non-ghost state.
    spec fn device_inv(&self) -> bool;

    /// A device's state is always known (`Unknown` is only the FracGhost's
    /// initial value).
    proof fn lemma_snapshot_known(&self)
        ensures
            !(self.snapshot() is Unknown),
    ;

    /// Hands out the device's ghost field. The device's state is unchanged.
    fn ghost_mut(&mut self) -> (r: &mut Tracked<DeviceGhostState>)
        ensures
            r@ == old(self).ghost(),
            final(self).ghost() == final(r)@,
            final(self).id() == old(self).id(),
            final(self).snapshot() == old(self).snapshot(),
            final(self).device_inv() == old(self).device_inv(),
    ;
}

/// Declares `std::task::Context` to Verus, as an opaque type.
#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExContext<'a>(std::task::Context<'a>);

/// A device that a verified state unit can drive: its state transitions as
/// synchronous methods with contracts. Each keeps the device's invariant
/// (`device_inv()`) and the rule (`DeviceGhostState::wf`), and its id.
/// Methods that change the device's state while it holds the whole
/// FracGhost update the FracGhost; the others leave the ghost alone.
pub trait StateUnitDevice: DeviceGhost + LockInvariant<Key = Loc> {
    /// The device's `CloseableMutex` invariant is its invariant and the
    /// rule, and the mutex's key is the FracGhost's id.
    proof fn lemma_lock_inv(&self)
        ensures
            self.inv() == (self.device_inv() && self.ghost().wf(self.id(), self.snapshot())),
            self.key() == self.id(),
    ;

    /// Starts the running device (the unit has already given it the whole
    /// FracGhost).
    fn start_device(&mut self)
        requires
            old(self).device_inv(),
            old(self).ghost().wf(old(self).id(), old(self).snapshot()),
            old(self).ghost().running,
        ensures
            final(self).device_inv(),
            final(self).ghost().wf(final(self).id(), final(self).snapshot()),
            final(self).ghost().running,
            final(self).id() == old(self).id(),
    ;

    /// Stops the device's own activity, while it still runs (the unit then
    /// records its state).
    fn stop_device(&mut self)
        requires
            old(self).device_inv(),
            old(self).ghost().wf(old(self).id(), old(self).snapshot()),
            old(self).ghost().running,
        ensures
            final(self).device_inv(),
            final(self).ghost().wf(final(self).id(), final(self).snapshot()),
            final(self).ghost().running,
            final(self).id() == old(self).id(),
    ;

    /// Polls the running device (its timers and other asynchronous work).
    fn poll_device(&mut self, cx: &mut std::task::Context<'_>)
        requires
            old(self).device_inv(),
            old(self).ghost().wf(old(self).id(), old(self).snapshot()),
            old(self).ghost().running,
        ensures
            final(self).device_inv(),
            final(self).ghost().wf(final(self).id(), final(self).snapshot()),
            final(self).ghost().running,
            final(self).id() == old(self).id(),
    ;

    /// Resets the stopped device, holding the whole FracGhost (lent with
    /// `Reset`).
    fn reset_device(&mut self)
        requires
            old(self).device_inv(),
            old(self).ghost().wf(old(self).id(), old(self).snapshot()),
            old(self).ghost().whole(),
            !old(self).ghost().running,
        ensures
            final(self).device_inv(),
            final(self).ghost().wf(final(self).id(), final(self).snapshot()),
            final(self).ghost().whole(),
            !final(self).ghost().running,
            final(self).id() == old(self).id(),
    ;

    /// Saves the device as a blob; changes neither its state nor its ghost.
    fn save_blob(&mut self) -> (result: Result<SavedStateBlob, SaveError>)
        requires
            old(self).device_inv(),
            old(self).ghost().wf(old(self).id(), old(self).snapshot()),
        ensures
            final(self).device_inv(),
            final(self).ghost() == old(self).ghost(),
            final(self).id() == old(self).id(),
            final(self).snapshot() == old(self).snapshot(),
            result matches Ok(blob) ==> old(self).snapshot() == UnitSnapshot::Saved(blob@),
    ;

    /// Restores the stopped device from a blob, holding the whole
    /// FracGhost (lent with `Restore`).
    fn restore_blob(&mut self, blob: SavedStateBlob) -> (result: Result<(), RestoreError>)
        requires
            old(self).device_inv(),
            old(self).ghost().wf(old(self).id(), old(self).snapshot()),
            old(self).ghost().whole(),
            !old(self).ghost().running,
        ensures
            final(self).device_inv(),
            final(self).ghost().wf(final(self).id(), final(self).snapshot()),
            final(self).ghost().whole(),
            !final(self).ghost().running,
            final(self).id() == old(self).id(),
            result is Ok ==> final(self).snapshot() == UnitSnapshot::Saved(blob@),
    ;
}

} // verus!
