// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

// Declarations and lemmas supporting `dispatch.spec.rs`; no human review
// needed.

use super::LoadedVmView;
use crate::worker::dispatch::InitializedVm;
use crate::worker::dispatch::LoadedVmInner;
use crate::worker::dispatch::microvm::SnapshotBoundary;
use crate::worker::dispatch::restore::SnapshotRestore;
use openvmm_defs::worker::spec::SavedStateView;
use vmcore::save_restore::spec::SavedStateBlobView;
use vmm_core_defs::HaltReason;
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

#[verifier::external_type_specification]
#[verifier::external_body]
pub(crate) struct ExInitializedVm(InitializedVm);

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExHaltReason(HaltReason);

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExMicrovmSnapshotBoundaryRequest(
    chipset_resources::microvm::MicrovmSnapshotBoundaryRequest,
);

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExRecvError(mesh::RecvError);

// (`anyhow::Error` is declared in `state_unit`.)

impl LoadedVmView {
    /// `InitializedVm::load`'s contract for a restore follows from the VM it
    /// builds and `restore_from_snapshot`'s: `self` is the freshly built VM
    /// (no unit's state recorded), `after` the VM after
    /// `restore_from_snapshot` succeeded.
    pub proof fn lemma_load_restores(self, state: SavedStateView, after: LoadedVmView)
        requires
            // The fresh VM.
            self.snapshot.units == Map::<Seq<char>, SavedStateBlobView>::empty(),
            // `restore_from_snapshot`'s `Ok` result.
            after.snapshot.units == self.snapshot.units.union_prefer_right(state.units),
        ensures
            after.snapshot.units == state.units,
    {
        assert(self.snapshot.units.union_prefer_right(state.units) =~= state.units);
    }

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
