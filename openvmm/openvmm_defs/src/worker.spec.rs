// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

// High-level spec of the VM worker's saved state, for human review.

use super::SavedState;
use state_unit::SavedStateUnit;
use vmcore::save_restore::spec::SavedStateBlobView;
use vstd::prelude::*;

verus! {

/// The view of a `SavedState`: the snapshotable state of a VM.
///
/// Also used as the snapshotable part of a loaded VM's view
/// (`LoadedVmView::snapshot`), so that save and restore can compare the two.
pub struct SavedStateView {
    /// Each state unit's saved state, by unit name. Units with no saved state
    /// are absent.
    pub units: Map<Seq<char>, SavedStateBlobView>,
    /// The names of all state units, in registration order, including units
    /// with no saved state.
    pub inventory: Seq<Seq<char>>,
}

impl SavedStateView {
    /// The inventory lists each unit once, and every unit with saved state is
    /// in it.
    pub open spec fn inv(self) -> bool {
        &&& self.inventory.no_duplicates()
        &&& self.units.dom() <= self.inventory.to_set()
    }
}

impl SavedState {
    /// Well formed: the view's invariant holds, and the entries have
    /// distinct unit names (so the view loses none of them). `save` produces
    /// well-formed saved states; `restore` accepts any and rejects
    /// duplicates itself.
    pub closed spec fn wf(&self) -> bool {
        &&& self@.inv()
        &&& SavedStateUnit::distinct_names(self.units@)
    }
}

impl View for SavedState {
    type V = SavedStateView;

    /// If two entries have the same unit name, the map holds one of them.
    /// (Restore rejects such a saved state.)
    closed spec fn view(&self) -> SavedStateView {
        SavedStateView {
            units: SavedStateUnit::map_of(self.units@),
            inventory: self.inventory@.map_values(|name: String| name@),
        }
    }
}

} // verus!
