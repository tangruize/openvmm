// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

// Declarations and lemmas supporting `dispatch.spec.rs`; no human review
// needed.

use super::LoadedVmInner;
use super::microvm::SnapshotBoundary;
use super::restore::SnapshotRestore;
use super::spec::LoadedVmView;
use openvmm_defs::worker::spec::SavedStateView;
use vstd::prelude::*;

verus! {

// The types of `LoadedVm`'s fields, declared to Verus as opaque types.

#[verifier::external_type_specification]
#[verifier::external_body]
pub(super) struct ExLoadedVmInner(LoadedVmInner);

#[verifier::external_type_specification]
#[verifier::external_body]
pub(super) struct ExSnapshotRestore(SnapshotRestore);

#[verifier::external_type_specification]
#[verifier::external_body]
pub(super) struct ExSnapshotBoundary(SnapshotBoundary);

// (`anyhow::Error` is declared in `state_unit`.)

/// Restoring a saved state into a VM with the same stateful units and the
/// same inventory gives back the snapshot state the saved state holds. With
/// `saved` from `LoadedVm::save`, this is the round trip: the VM's snapshot
/// state is the one at save time.
pub proof fn lemma_restore_gives_saved_state(
    saved: SavedStateView,
    before: LoadedVmView,
    after: LoadedVmView,
)
    requires
        // The VM restored into has the units the saved state has.
        before.snapshot.units.dom() == saved.units.dom(),
        before.snapshot.inventory == saved.inventory,
        // `restore`'s postcondition on success.
        after.snapshot.inventory == before.snapshot.inventory,
        after.snapshot.units == before.snapshot.units.union_prefer_right(saved.units),
    ensures
        after.snapshot == saved,
{
    assert(after.snapshot.units =~= saved.units);
}

} // verus!
