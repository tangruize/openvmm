// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

// The `StateUnit` spec of a line set's state unit, for human review.

use super::LineSetUnit;
use state_unit::StateUnitSpec;
use vmcore::save_restore::spec::UnitSnapshot;
use vstd::prelude::*;
use vstd::resource::Loc;

verus! {

/// A line set's unit keeps its share of its FracGhost in its ghost field.
/// A line set has no saved state (its `save` returns `Ok(None)`), so the
/// share, once known, records `NoSavedState`.
impl StateUnitSpec for LineSetUnit<'_> {
    closed spec fn wf(&self) -> bool {
        self.1@.follows_rule(self.id(), UnitSnapshot::NoSavedState)
    }

    closed spec fn id(&self) -> Loc {
        self.1@.frac.id()
    }

    closed spec fn running(&self) -> bool {
        self.1@.running
    }
}

} // verus!
