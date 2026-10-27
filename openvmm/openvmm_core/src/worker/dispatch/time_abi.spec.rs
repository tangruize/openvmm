// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

// The `StateUnit` spec of the time ABI's state unit, for human review.

use super::TimeAbiUnit;
use state_unit::StateUnitSpec;
use vstd::prelude::*;
use vstd::resource::Loc;

verus! {

/// The time-ABI unit's share of its FracGhost is kept with the MSR value it
/// saves, in `msrs`'s atomic (`virt::time_abi::msr::spec::TscControlPred`):
/// the VPs change that value too. The unit's id is the atomic's key; its
/// running flag is the unit's ghost field.
impl StateUnitSpec for TimeAbiUnit {
    closed spec fn wf(&self) -> bool {
        (*self.msrs).wf()
    }

    closed spec fn id(&self) -> Loc {
        (*self.msrs).key()
    }

    closed spec fn running(&self) -> bool {
        self.running@
    }
}

} // verus!
