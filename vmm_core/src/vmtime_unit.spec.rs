// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

// The `StateUnit` spec of the VM time keeper's state unit, for human review.

use super::KeeperUnit;
use state_unit::StateUnitSpec;
use vmcore::save_restore::spec::UnitSnapshot;
use vmcore::vmtime::spec::VmTimeKeeperView;
use vstd::prelude::*;
use vstd::resource::Loc;

verus! {

/// The time keeper's unit keeps its share of its FracGhost in its ghost
/// field. Its saved state is all time, so the share, once known, records
/// the one blob view every keeper's saved state has
/// (`VmTimeKeeperView::saved_blob`); its running flag is the keeper's.
impl StateUnitSpec for KeeperUnit<'_> {
    closed spec fn wf(&self) -> bool {
        &&& self.1@.follows_rule(self.id(), UnitSnapshot::Saved(VmTimeKeeperView::saved_blob()))
        &&& self.1@.running == (*self.0)@.running
    }

    closed spec fn id(&self) -> Loc {
        self.1@.frac.id()
    }

    closed spec fn running(&self) -> bool {
        (*self.0)@.running
    }
}

} // verus!
