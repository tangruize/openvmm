// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

// Engineer-owned representation bridges for snapshot capture.

use super::LoadedVm;
use super::snapshot_save_spec::LoadedVmCaptureView;
use super::snapshot_save_spec::SavedSnapshotStateView;
use openvmm_defs::rpc::SnapshotQuiesceError;
use openvmm_defs::worker::SavedState;
use vstd::prelude::*;

verus! {

// Carrier declarations expose only Rust type identity. No behavior or field
// semantics are trusted here.
#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExStateUnits(state_unit::StateUnits);

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExStopGuard(vmm_core::partition_unit::StopGuard);

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExFile(std::fs::File);

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExInstant(pal_async::timer::Instant);

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExProfileSpan(openvmm_defs::profile::ProfileSpan);

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExSnapshotBoundaryRequest(
    chipset_resources::microvm::MicrovmSnapshotBoundaryRequest,
);

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExSnapshotScratchPolicy(
    chipset_resources::microvm::MicrovmSnapshotScratchPolicy,
);

#[verifier::external_type_specification]
#[verifier::external_body]
#[verifier::reject_recursive_types(T)]
pub struct ExReceiver<T>(mesh::Receiver<T>);

#[verifier::external_type_specification]
#[verifier::external_body]
#[verifier::reject_recursive_types(T)]
pub struct ExSender<T>(mesh::Sender<T>);

#[verifier::external_type_specification]
#[verifier::external_body]
#[verifier::reject_recursive_types(I)]
#[verifier::reject_recursive_types(R)]
pub struct ExRpc<I, R>(mesh::rpc::Rpc<I, R>);

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExTimestamp(mesh::payload::Timestamp);

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExSnapshotQuiesceError(SnapshotQuiesceError);

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExSavedState(SavedState);

// TODO(uninterp): Decode state-unit payloads and inventory into the
// Human-owned saved snapshot View.
pub uninterp spec fn saved_snapshot_state_view(
    saved_state: &SavedState,
) -> SavedSnapshotStateView;

// TODO(uninterp): Replace with component Views for VM state, boundary
// ownership, machine identity, clocks, and the CPU compatibility contract.
pub uninterp spec fn loaded_vm_capture_view(vm: &LoadedVm) -> LoadedVmCaptureView;

// TODO(uninterp): Replace with phase-specific concrete postconditions for
// Rejected, RollbackSafe-after-quiesce, Uncertain, and RollbackSafe-after-save.
pub uninterp spec fn snapshot_capture_error_post(
    initial: &LoadedVm,
    error: &SnapshotQuiesceError,
    captured: &LoadedVm,
) -> bool;

impl View for LoadedVm {
    type V = LoadedVmCaptureView;

    closed spec fn view(&self) -> LoadedVmCaptureView {
        loaded_vm_capture_view(self)
    }
}

pub closed spec fn snapshot_capture_request_is_valid(loaded: &LoadedVm) -> bool {
    loaded@.lifecycle_running == loaded.running
    && (
        !loaded.running
        || (
            loaded@.units_running
            && loaded@.boundary.is_some()
            && loaded.snapshot_stop_guard.is_some()
            && loaded.snapshot_transaction_complete.is_some()
            && loaded.snapshot_capture_wall_clock.is_some()
        )
    )
}

pub closed spec fn snapshot_stopped_representation(loaded: &LoadedVm) -> bool {
    !loaded.running
    && loaded.snapshot_stop_guard.is_some()
    && loaded.snapshot_transaction_complete.is_some()
    && loaded.snapshot_capture_wall_clock.is_some()
    && !loaded@.lifecycle_running
    && !loaded@.units_running
    && loaded@.boundary.is_some()
}

} // verus!
