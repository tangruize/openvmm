// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

// The `StateUnit` spec of a chipset device's state unit, for human review.

use super::ArcMutexChipsetDeviceUnit;
use state_unit::StateUnitSpec;
use vstd::prelude::*;
use vstd::resource::Loc;

verus! {

/// A chipset device's unit keeps its share of its FracGhost with the
/// device, in the device's lock (`DeviceSlot`; its lock invariant ties the
/// share to the device's snapshot). The unit's id is the id in the lock's
/// parameter (the lock's own invariant names that FracGhost); its
/// `omit_saved_state` (whether its save returns `None`) is the parameter's
/// other part, so the lock's snapshot and the unit's save agree; its
/// running flag is its own.
impl StateUnitSpec for ArcMutexChipsetDeviceUnit {
    closed spec fn wf(&self) -> bool {
        (*self.device).param().1 == self.omit_saved_state
    }

    closed spec fn id(&self) -> Loc {
        (*self.device).param().0
    }

    closed spec fn running(&self) -> bool {
        self.running
    }
}

} // verus!
