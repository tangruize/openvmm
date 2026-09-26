// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

// Human-owned success semantics for
// `VmController::handle_guest_snapshot_request`.

use vstd::prelude::*;

verus! {

pub type SnapshotEpoch = nat;
pub type UnitId = nat;

pub enum UnitPhase {
    Running,
    Stopped,
    Saved,
}

pub enum SnapshotPhase {
    BoundaryEstablished,
    Quiescing,
    Quiescent,
    Saving,
    Captured,
    Published,
}

pub struct VmStateView {
    pub component_state: Map<UnitId, int>,
    pub ram: Map<nat, u8>,
    pub virtual_time: int,
}

pub struct UnitRecord {
    pub phase: UnitPhase,
    pub state: int,
    pub saved_payload: Option<int>,
}

/// The linear ghost resource returned by one state unit.
pub struct UnitSnapshotFragment {
    pub epoch: SnapshotEpoch,
    pub unit: UnitId,
    pub record: UnitRecord,
}

/// PCM carrier: independently owned unit fragments compose by disjoint map
/// union. Overlapping unit IDs are incompatible.
pub struct UnitSnapshotPcm {
    pub fragments: Map<UnitId, UnitSnapshotFragment>,
}

pub open spec fn pcm_empty() -> UnitSnapshotPcm {
    UnitSnapshotPcm {
        fragments: Map::<UnitId, UnitSnapshotFragment>::empty(),
    }
}

pub open spec fn pcm_compatible(
    left: UnitSnapshotPcm,
    right: UnitSnapshotPcm,
) -> bool {
    left.fragments.dom().disjoint(right.fragments.dom())
}

pub open spec fn pcm_compose(
    left: UnitSnapshotPcm,
    right: UnitSnapshotPcm,
) -> UnitSnapshotPcm {
    UnitSnapshotPcm {
        fragments: left.fragments.union_prefer_right(right.fragments),
    }
}

pub open spec fn unit_fragment_well_formed(
    epoch: SnapshotEpoch,
    unit: UnitId,
    fragment: UnitSnapshotFragment,
) -> bool {
    fragment.epoch == epoch
    && fragment.unit == unit
    && match fragment.record.phase {
        UnitPhase::Running => fragment.record.saved_payload.is_none(),
        UnitPhase::Stopped => fragment.record.saved_payload.is_none(),
        UnitPhase::Saved => true,
    }
}

pub uninterp spec fn unit_reachable(
    unit: UnitId,
    before: int,
    after: int,
) -> bool;

pub uninterp spec fn snapshot_component(
    unit: UnitId,
    state: int,
) -> Option<int>;

/// Local obligation proved by each state unit when its stop request completes.
pub open spec fn unit_stop_success(
    epoch: SnapshotEpoch,
    unit: UnitId,
    before: UnitSnapshotFragment,
    stopped: UnitSnapshotFragment,
) -> bool {
    unit_fragment_well_formed(epoch, unit, before)
    && before.record.phase == UnitPhase::Running
    && unit_fragment_well_formed(epoch, unit, stopped)
    && stopped.record.phase == UnitPhase::Stopped
    && unit_reachable(unit, before.record.state, stopped.record.state)
}

/// Local obligation proved by each stopped unit when its save reply completes.
pub open spec fn unit_save_success(
    epoch: SnapshotEpoch,
    unit: UnitId,
    stopped: UnitSnapshotFragment,
    saved: UnitSnapshotFragment,
) -> bool {
    unit_fragment_well_formed(epoch, unit, stopped)
    && stopped.record.phase == UnitPhase::Stopped
    && unit_fragment_well_formed(epoch, unit, saved)
    && saved.record.phase == UnitPhase::Saved
    && saved.record.state == stopped.record.state
    && saved.record.saved_payload
        == snapshot_component(unit, stopped.record.state)
}

pub struct SnapshotCutView {
    pub epoch: SnapshotEpoch,
    pub request_state: VmStateView,
    pub captured_state: VmStateView,
    pub inventory: Set<UnitId>,
    pub payload_units: Set<UnitId>,
    pub input_gated: bool,
    pub vps_held: bool,
}

pub struct SnapshotArtifactView {
    pub epoch: SnapshotEpoch,
    pub saved_components: Map<UnitId, int>,
    pub memory: Map<nat, u8>,
    pub manifest_inventory: Set<UnitId>,
    pub scratch_policy: int,
    pub committed: bool,
}

/// Ghost interpretation of a successful `VmRpc::QuiesceForSnapshot`
/// response. The concrete response carries the encoded saved state and
/// inventory; `cut` and `unit_pcm` are the proof evidence attached to it.
pub struct SnapshotCaptureResponseView {
    pub epoch: SnapshotEpoch,
    pub cut: SnapshotCutView,
    pub unit_pcm: UnitSnapshotPcm,
    pub saved_components: Map<UnitId, int>,
    pub inventory: Set<UnitId>,
    pub mapped_ram_flushed: bool,
}

/// Facts checked by the first closure in
/// `handle_guest_snapshot_request`.
pub struct SnapshotPreflightView {
    pub microvm_profile: bool,
    pub supported_hypervisor: bool,
    pub block_layout_valid: bool,
    pub allowed_scratch_policies: Set<int>,
    pub destination_absent: bool,
    pub memory_backing_present: bool,
    pub memory_handle_exact: bool,
    pub command_line_present: bool,
}

/// Output of the second closure before the final directory publication.
///
/// These fields summarize the direct callees that build the block and machine
/// contracts, construct the manifest, encode saved state, flush the RAM
/// handle, and select the optional scratch source.
pub struct SnapshotPublicationPreparationView {
    pub epoch: SnapshotEpoch,
    pub saved_components: Map<UnitId, int>,
    pub memory: Map<nat, u8>,
    pub manifest_inventory: Set<UnitId>,
    pub scratch_policy: int,
    pub block_contract_valid: bool,
    pub machine_contract_matches: bool,
    pub manifest_matches: bool,
    pub saved_state_encoding_matches: bool,
    pub memory_handle_flushed: bool,
    pub scratch_source_matches: bool,
}

pub uninterp spec fn vm_reachable(
    request_state: VmStateView,
    captured_state: VmStateView,
) -> bool;

pub open spec fn fragment_matches_cut(
    cut: SnapshotCutView,
    unit: UnitId,
    fragment: UnitSnapshotFragment,
) -> bool {
    cut.inventory.contains(unit)
    && cut.captured_state.component_state.dom().contains(unit)
    && unit_fragment_well_formed(cut.epoch, unit, fragment)
    && fragment.record.phase == UnitPhase::Saved
    && fragment.record.state == cut.captured_state.component_state.index(unit)
    && fragment.record.saved_payload
        == snapshot_component(unit, fragment.record.state)
    && (
        cut.payload_units.contains(unit)
            <==> fragment.record.saved_payload.is_some()
    )
}

pub open spec fn pcm_complete_for_cut(
    pcm: UnitSnapshotPcm,
    cut: SnapshotCutView,
) -> bool {
    pcm.fragments.dom() == cut.inventory
    && forall |unit: UnitId|
        cut.inventory.contains(unit) ==>
            fragment_matches_cut(cut, unit, pcm.fragments.index(unit))
}

pub open spec fn capture_response_matches_cut(
    response: SnapshotCaptureResponseView,
) -> bool {
    response.epoch == response.cut.epoch
    && response.inventory == response.cut.inventory
    && response.saved_components.dom() == response.cut.payload_units
    && response.mapped_ram_flushed
    && pcm_complete_for_cut(response.unit_pcm, response.cut)
    && forall |unit: UnitId|
        response.cut.payload_units.contains(unit) ==>
            response.saved_components.index(unit)
                == snapshot_component(
                    unit,
                    response.cut.captured_state.component_state.index(unit),
                ).unwrap()
}

pub open spec fn snapshot_preflight_closure_success(
    preflight: SnapshotPreflightView,
    requested_scratch_policy: int,
) -> bool {
    preflight.microvm_profile
    && preflight.supported_hypervisor
    && preflight.block_layout_valid
    && preflight.allowed_scratch_policies.contains(requested_scratch_policy)
    && preflight.destination_absent
    && preflight.memory_backing_present
    && preflight.memory_handle_exact
    && preflight.command_line_present
}

pub open spec fn prepare_snapshot_publication_success(
    response: SnapshotCaptureResponseView,
    requested_scratch_policy: int,
    prepared: SnapshotPublicationPreparationView,
) -> bool {
    capture_response_matches_cut(response)
    && prepared.epoch == response.epoch
    && prepared.saved_components == response.saved_components
    && prepared.memory == response.cut.captured_state.ram
    && prepared.manifest_inventory == response.inventory
    && prepared.scratch_policy == requested_scratch_policy
    && prepared.block_contract_valid
    && prepared.machine_contract_matches
    && prepared.manifest_matches
    && prepared.saved_state_encoding_matches
    && prepared.memory_handle_flushed
    && prepared.scratch_source_matches
}

pub open spec fn artifact_matches_cut(
    artifact: SnapshotArtifactView,
    cut: SnapshotCutView,
) -> bool {
    artifact.epoch == cut.epoch
    && artifact.committed
    && artifact.manifest_inventory == cut.inventory
    && artifact.saved_components.dom() == cut.payload_units
    && artifact.memory == cut.captured_state.ram
    && forall |unit: UnitId|
        cut.payload_units.contains(unit) ==>
            artifact.saved_components.index(unit)
                == snapshot_component(
                    unit,
                    cut.captured_state.component_state.index(unit),
                ).unwrap()
}

/// Authoritative ghost state protected by the snapshot atomic invariant.
pub struct SnapshotAtomicState {
    pub epoch: SnapshotEpoch,
    pub phase: SnapshotPhase,
    pub request_state: VmStateView,
    pub current_state: VmStateView,
    pub inventory: Set<UnitId>,
    pub payload_units: Set<UnitId>,
    pub units: Map<UnitId, UnitRecord>,
    pub input_gated: bool,
    pub vps_held: bool,
    pub cut: Option<SnapshotCutView>,
    pub artifact: Option<SnapshotArtifactView>,
}

pub open spec fn all_units_at_least_stopped(
    state: SnapshotAtomicState,
) -> bool {
    forall |unit: UnitId|
        state.inventory.contains(unit) ==>
            match state.units.index(unit).phase {
                UnitPhase::Running => false,
                UnitPhase::Stopped => true,
                UnitPhase::Saved => true,
            }
}

pub open spec fn all_units_saved(
    state: SnapshotAtomicState,
) -> bool {
    forall |unit: UnitId|
        state.inventory.contains(unit) ==>
            state.units.index(unit).phase == UnitPhase::Saved
}

pub open spec fn cut_matches_authority(
    state: SnapshotAtomicState,
    cut: SnapshotCutView,
) -> bool {
    cut.epoch == state.epoch
    && cut.request_state == state.request_state
    && cut.captured_state == state.current_state
    && cut.inventory == state.inventory
    && cut.payload_units == state.payload_units
    && cut.input_gated == state.input_gated
    && cut.vps_held == state.vps_held
}

/// The invariant shared by the controller, snapshot worker, and state units.
///
/// It records one request epoch and one authoritative VM state. Unit PCM
/// fragments may be distributed, but any fragment for this epoch must agree
/// with `units`. Once the cut exists, all snapshotable writers are stopped and
/// the cut remains stable through save and publication.
pub open spec fn snapshot_atomic_invariant(
    state: SnapshotAtomicState,
) -> bool {
    state.units.dom() == state.inventory
    && state.payload_units.subset_of(state.inventory)
    && state.current_state.component_state.dom() == state.inventory
    && vm_reachable(state.request_state, state.current_state)
    && forall |unit: UnitId|
        state.inventory.contains(unit) ==>
            state.units.index(unit).state
                == state.current_state.component_state.index(unit)
    && match state.phase {
        SnapshotPhase::BoundaryEstablished => {
            state.input_gated
            && state.vps_held
            && forall |unit: UnitId|
                state.inventory.contains(unit) ==>
                    state.units.index(unit).phase == UnitPhase::Running
            && state.cut.is_none()
            && state.artifact.is_none()
        },
        SnapshotPhase::Quiescing => {
            state.input_gated
            && state.vps_held
            && state.cut.is_none()
            && state.artifact.is_none()
        },
        SnapshotPhase::Quiescent => {
            state.input_gated
            && state.vps_held
            && all_units_at_least_stopped(state)
            && state.cut.is_some()
            && cut_matches_authority(state, state.cut.unwrap())
            && state.artifact.is_none()
        },
        SnapshotPhase::Saving => {
            state.input_gated
            && state.vps_held
            && all_units_at_least_stopped(state)
            && state.cut.is_some()
            && cut_matches_authority(state, state.cut.unwrap())
            && state.artifact.is_none()
        },
        SnapshotPhase::Captured => {
            state.input_gated
            && state.vps_held
            && all_units_saved(state)
            && state.cut.is_some()
            && cut_matches_authority(state, state.cut.unwrap())
            && state.artifact.is_none()
        },
        SnapshotPhase::Published => {
            state.input_gated
            && state.vps_held
            && all_units_saved(state)
            && state.cut.is_some()
            && cut_matches_authority(state, state.cut.unwrap())
            && state.artifact.is_some()
            && artifact_matches_cut(
                state.artifact.unwrap(),
                state.cut.unwrap(),
            )
        },
    }
}

pub open spec fn pcm_agrees_with_authority(
    pcm: UnitSnapshotPcm,
    state: SnapshotAtomicState,
) -> bool {
    pcm.fragments.dom().subset_of(state.inventory)
    && forall |unit: UnitId|
        pcm.fragments.dom().contains(unit) ==>
            pcm.fragments.index(unit).epoch == state.epoch
            && pcm.fragments.index(unit).unit == unit
            && pcm.fragments.index(unit).record == state.units.index(unit)
}

pub struct VmControllerSnapshotView {
    pub protocol: SnapshotAtomicState,
    pub collected_units: UnitSnapshotPcm,
    pub preflight: SnapshotPreflightView,
    pub boundary_owned: bool,
    pub source_quiescent: bool,
    pub publication_committed: bool,
    pub success_cleanup_complete: bool,
}

/// Success contract for the direct `VmRpc::QuiesceForSnapshot` call.
///
/// The response carries one complete PCM assembled from the independently
/// stopped and saved state units. The returned cut is still protected by the
/// same input gate and vCPU-stop boundary.
pub open spec fn quiesce_for_snapshot_rpc_success(
    initial: VmControllerSnapshotView,
    captured: VmControllerSnapshotView,
    response: SnapshotCaptureResponseView,
) -> bool {
    snapshot_atomic_invariant(initial.protocol)
    && initial.protocol.phase == SnapshotPhase::BoundaryEstablished
    && initial.boundary_owned
    && snapshot_atomic_invariant(captured.protocol)
    && captured.protocol.phase == SnapshotPhase::Captured
    && captured.protocol.epoch == initial.protocol.epoch
    && captured.protocol.request_state == initial.protocol.request_state
    && captured.protocol.cut == Some(response.cut)
    && captured.collected_units == response.unit_pcm
    && captured.preflight == initial.preflight
    && pcm_agrees_with_authority(
        response.unit_pcm,
        captured.protocol,
    )
    && capture_response_matches_cut(response)
    && captured.boundary_owned
    && captured.source_quiescent
    && !captured.publication_committed
}

/// Success contract shared by
/// `write_snapshot_from_owned_memory_and_scratch_files` and
/// `write_snapshot_from_memory_and_scratch_files`.
pub open spec fn write_snapshot_success(
    captured: VmControllerSnapshotView,
    response: SnapshotCaptureResponseView,
    prepared: SnapshotPublicationPreparationView,
    published: VmControllerSnapshotView,
) -> bool {
    snapshot_atomic_invariant(captured.protocol)
    && captured.protocol.phase == SnapshotPhase::Captured
    && captured.protocol.cut == Some(response.cut)
    && captured.collected_units == response.unit_pcm
    && prepared.epoch == response.epoch
    && snapshot_atomic_invariant(published.protocol)
    && published.protocol.phase == SnapshotPhase::Published
    && published.protocol.epoch == captured.protocol.epoch
    && published.protocol.request_state == captured.protocol.request_state
    && published.protocol.cut == captured.protocol.cut
    && published.collected_units == captured.collected_units
    && published.preflight == captured.preflight
    && published.protocol.artifact.is_some()
    && published.protocol.artifact.unwrap().epoch == prepared.epoch
    && published.protocol.artifact.unwrap().saved_components
        == prepared.saved_components
    && published.protocol.artifact.unwrap().memory == prepared.memory
    && published.protocol.artifact.unwrap().manifest_inventory
        == prepared.manifest_inventory
    && published.protocol.artifact.unwrap().scratch_policy
        == prepared.scratch_policy
    && published.boundary_owned
    && published.source_quiescent
    && published.publication_committed
}

/// Success contract for the second closure in
/// `handle_guest_snapshot_request`.
pub open spec fn snapshot_publication_closure_success(
    captured: VmControllerSnapshotView,
    response: SnapshotCaptureResponseView,
    requested_scratch_policy: int,
    published: VmControllerSnapshotView,
) -> bool {
    exists |prepared: SnapshotPublicationPreparationView|
        prepare_snapshot_publication_success(
            response,
            requested_scratch_policy,
            prepared,
        )
        && write_snapshot_success(
            captured,
            response,
            prepared,
            published,
        )
}

/// Successful console cleanup occurs after publication. It cannot change the
/// captured cut, PCM evidence, or committed artifact.
pub open spec fn post_publication_cleanup_success(
    published: VmControllerSnapshotView,
    cleaned: VmControllerSnapshotView,
) -> bool {
    snapshot_atomic_invariant(published.protocol)
    && published.protocol.phase == SnapshotPhase::Published
    && cleaned.protocol == published.protocol
    && cleaned.collected_units == published.collected_units
    && cleaned.preflight == published.preflight
    && cleaned.boundary_owned == published.boundary_owned
    && cleaned.source_quiescent == published.source_quiescent
    && cleaned.publication_committed == published.publication_committed
    && cleaned.success_cleanup_complete
}

/// Success contract for `handle_guest_snapshot_request`.
///
/// `returned_success` means the concrete result is
/// `GuestSnapshotAction::Terminate { exit_code: 0 }`.
pub open spec fn handle_guest_snapshot_request_success(
    initial: VmControllerSnapshotView,
    final_state: VmControllerSnapshotView,
    requested_scratch_policy: int,
    returned_success: bool,
) -> bool {
    returned_success ==>
        snapshot_preflight_closure_success(
            initial.preflight,
            requested_scratch_policy,
        )
        && exists |captured: VmControllerSnapshotView,
                   response: SnapshotCaptureResponseView,
                   published: VmControllerSnapshotView|
            quiesce_for_snapshot_rpc_success(
                initial,
                captured,
                response,
            )
            && snapshot_publication_closure_success(
                captured,
                response,
                requested_scratch_policy,
                published,
            )
            && post_publication_cleanup_success(
                published,
                final_state,
            )
}

} // verus!
