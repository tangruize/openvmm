// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

// The `StateUnit` spec of the input distributor's state unit, for human review.

use super::Inner;
use state_unit::StateUnitSpec;
use vmcore::save_restore::spec::UnitSnapshot;
use vstd::prelude::*;
use vstd::resource::Loc;

verus! {

/// The input distributor keeps its share of its FracGhost in `Inner`'s
/// ghost field (taken in `InputDistributor::run`). It has no saved state
/// (its `save` returns `Ok(None)`), so the share, once known, records
/// `NoSavedState`; its running flag is `Inner`'s `running`.
impl StateUnitSpec for Inner {
    closed spec fn wf(&self) -> bool {
        &&& self.ghost@ is Some
        &&& self.ghost@->Some_0.follows_rule(self.id(), UnitSnapshot::NoSavedState)
        &&& self.ghost@->Some_0.running == self.running
    }

    closed spec fn id(&self) -> Loc {
        self.ghost@->Some_0.frac.id()
    }

    closed spec fn running(&self) -> bool {
        self.running
    }
}

} // verus!
