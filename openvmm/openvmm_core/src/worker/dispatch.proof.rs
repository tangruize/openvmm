// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

// Declarations and lemmas supporting `dispatch.spec.rs`; no human review
// needed.

use super::LoadedVmView;
use crate::worker::dispatch::LoadedVmInner;
use crate::worker::dispatch::microvm::SnapshotBoundary;
use crate::worker::dispatch::restore::SnapshotRestore;
use openvmm_defs::worker::spec::SavedStateView;
use vstd::prelude::*;

verus! {

// The types of `LoadedVm`'s fields, declared to Verus as opaque types.

#[verifier::external_type_specification]
#[verifier::external_body]
pub(in crate::worker::dispatch) struct ExLoadedVmInner(LoadedVmInner);

#[verifier::external_type_specification]
#[verifier::external_body]
pub(in crate::worker::dispatch) struct ExSnapshotRestore(SnapshotRestore);

#[verifier::external_type_specification]
#[verifier::external_body]
pub(in crate::worker::dispatch) struct ExSnapshotBoundary(SnapshotBoundary);

// (`anyhow::Error` is declared in `state_unit`.)

impl LoadedVmView {
    /// Restoring a saved state into a VM with the same stateful units and the
    /// same inventory gives back the snapshot state the saved state holds.
    /// With `saved` from `LoadedVm::save`, this is the round trip: the VM's
    /// snapshot state is the one at save time. `self` is the VM before the
    /// restore, `after` the VM after it.
    pub proof fn lemma_restore_gives_saved_state(self, saved: SavedStateView, after: LoadedVmView)
        requires
            // The VM restored into has the units the saved state has.
            self.snapshot.units.dom() == saved.units.dom(),
            self.snapshot.inventory == saved.inventory,
            // `restore`'s postcondition on success.
            after.snapshot.inventory == self.snapshot.inventory,
            after.snapshot.units == self.snapshot.units.union_prefer_right(saved.units),
        ensures
            after.snapshot == saved,
    {
        assert(after.snapshot.units =~= saved.units);
    }
}

} // verus!
