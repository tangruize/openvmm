// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

// Engineer-owned closed specifications and proof lemmas for `dispatch.rs`.
//
// Keep Human-owned restore semantics in `dispatch.spec.rs`. Add substantial
// proof functions here as frontend limitations are removed.

use super::InitializedVm;
use super::LoadedVm;
use super::restore_spec::ComponentSnapshotStateView;
use super::restore_spec::InitializedVmView;
use super::restore_spec::LoadedVmView;
use super::restore_spec::PartitionStateView;
use super::restore_spec::RestoreRequestView;
use super::restore_spec::VpStateView;
use openvmm_defs::worker::SavedState;
use std::time::Duration;
use vstd::prelude::*;

verus! {

// Only the carried type is opaque; no file operations are specified or trusted.
#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExRestoreReadyFile(std::fs::File);

// This bridge exposes only SavedState-owned state and the restore-time policy.
// Prepared memory, compatibility, resources, and VP selection are already
// represented by the pre-state of LoadedVm at this TOP boundary.
pub uninterp spec fn decoded_restore_request_view(
    saved_state: &SavedState,
    restore_time: &Option<(Duration, u64, Option<u64>)>,
    selected_vp_count: nat,
) -> RestoreRequestView;

// The caller resolves the selected count from its explicit selection or
// destination capacity before decoding the optional snapshot.
pub uninterp spec fn decoded_load_restore_request_view(
    saved_state: &Option<SavedState>,
    restore_time: &Option<(Duration, u64, Option<u64>)>,
    selected_vp_count: nat,
) -> RestoreRequestView;

// TODO(uninterp): Replace with component Views for processor topology, memory,
// component identity, compatibility, and external resource identity.
pub uninterp spec fn initialized_vm_representation(vm: &InitializedVm) -> InitializedVmView;

impl View for InitializedVm {
    type V = InitializedVmView;

    closed spec fn view(&self) -> InitializedVmView {
        initialized_vm_representation(self)
    }
}

// TODO(uninterp): Replace with component Views for partition/VP state,
// components, memory, virtual time, deferred devices, and lifecycle state.
pub uninterp spec fn loaded_vm_representation(vm: &LoadedVm) -> LoadedVmView;

impl View for LoadedVm {
    type V = LoadedVmView;

    closed spec fn view(&self) -> LoadedVmView {
        loaded_vm_representation(self)
    }
}

// Downtime compensation of opaque leaf snapshot state, used by
// `VmSnapshotView::after_downtime`. Each advances every guest-visible clock
// the leaf carries by `downtime_ns` and leaves every other field unchanged.
impl VpStateView {
    // TODO(uninterp): Define with the VP state View: TSC advanced by the
    // downtime cycles at the saved frequency and the local APIC timer by the
    // corresponding APIC ticks (`PartitionUnit::advance_tsc`, x86_64 only;
    // identity elsewhere).
    pub uninterp spec fn after_downtime(self, downtime_ns: nat) -> VpStateView;
}

impl PartitionStateView {
    // TODO(uninterp): Define with the partition state View: backend snapshot
    // clock advanced by the downtime (`HvlitePartition::advance_snapshot_time`).
    pub uninterp spec fn after_downtime(self, downtime_ns: nat) -> PartitionStateView;
}

impl ComponentSnapshotStateView {
    // TODO(uninterp): Define per component with its snapshot View: the
    // component's `advance_time` effect, e.g. the CMOS RTC clock; identity for
    // components using the default no-op (`StateUnits::advance_time`).
    pub uninterp spec fn after_downtime(self, downtime_ns: nat) -> ComponentSnapshotStateView;
}

pub closed spec fn pre_execution_representation(
    loaded: &LoadedVm,
    restored_from_snapshot: bool,
) -> bool {
    loaded.restored_from_snapshot == restored_from_snapshot
    && loaded.restore_start_guard.is_some() == restored_from_snapshot
    && !loaded.running
}

} // verus!
