// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

// Engineer-owned closed specifications and proof lemmas for `dispatch.rs`.
//
// Keep Human-owned restore semantics in `dispatch.spec.rs`. Add substantial
// proof functions here as frontend limitations are removed.

use super::LoadedVm;
use super::restore_spec::LoadedVmView;
use super::restore_spec::UnitName;
use openvmm_defs::worker::SavedState;
use state_unit::SavedStateUnit;
use vmcore::save_restore::SavedStateBlob;
use vstd::prelude::*;

verus! {

// Only the carried type is opaque; no file operations are specified or trusted.
#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExRestoreReadyFile(std::fs::File);

// The snapshot wire form. `SavedState` and `SavedStateUnit` are transparent:
// Verus sees their declared public fields. A `SavedStateBlob` is an opaque
// protobuf payload compared only as a value; its schema is owned by the unit.
#[verifier::external_type_specification]
pub struct ExSavedState(SavedState);

#[verifier::external_type_specification]
pub struct ExSavedStateUnit(SavedStateUnit);

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExSavedStateBlob(SavedStateBlob);

// TODO(uninterp): Define as the snapshot-owned state of `vm`, i.e. the
// decoded `SavedState` that `LoadedVm::save` would return (every state unit's
// `save` output plus `StateUnits::inventory`). The `LoadedVm::save` contract
// fixes this meaning: its output is real data, so no other definition can
// verify against a verified `save` body.
pub uninterp spec fn loaded_vm_representation(vm: &LoadedVm) -> LoadedVmView;

impl View for LoadedVm {
    type V = LoadedVmView;

    closed spec fn view(&self) -> LoadedVmView {
        loaded_vm_representation(self)
    }
}

// TODO(uninterp): Define per unit from its saved-state schema: the state its
// `advance_time` produces (plus `PartitionUnit::advance_tsc` and the backend
// snapshot clock for the partition unit), as observed by its next `save`.
// Identity for units with the default no-op `advance_time`. Anchored by the
// `LoadedVm::save` contract on the restored VM.
pub uninterp spec fn unit_after_downtime(
    name: UnitName,
    state: SavedStateBlob,
    downtime_ns: nat,
) -> SavedStateBlob;

pub closed spec fn pre_execution_representation(
    loaded: &LoadedVm,
    restored_from_snapshot: bool,
) -> bool {
    loaded.restored_from_snapshot == restored_from_snapshot
    && loaded.restore_start_guard.is_some() == restored_from_snapshot
    && !loaded.running
}

} // verus!
