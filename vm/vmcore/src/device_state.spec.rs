// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

// High-level spec of a device's ghost state, for human review.

use crate::save_restore::spec::UnitSnapshot;
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

} // verus!
