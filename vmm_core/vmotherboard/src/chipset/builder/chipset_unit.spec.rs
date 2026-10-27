// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

// The `StateUnit` spec of the chipset unit, for human review.

use super::ChipsetUnit;
use state_unit::StateUnitSpec;
use vmcore::save_restore::spec::UnitSnapshot;
use vstd::prelude::*;
use vstd::resource::Loc;

verus! {

/// The chipset unit keeps its share of its FracGhost in its ghost field.
/// It has no saved state (its `save` returns `Ok(None)`; the devices are
/// their own units), so the share, once known, records `NoSavedState`.
impl StateUnitSpec for ChipsetUnit<'_> {
    closed spec fn wf(&self) -> bool {
        self.1@.wf(self.id(), UnitSnapshot::NoSavedState)
    }

    closed spec fn id(&self) -> Loc {
        self.1@.frac.id()
    }

    closed spec fn running(&self) -> bool {
        self.1@.running
    }
}

} // verus!
