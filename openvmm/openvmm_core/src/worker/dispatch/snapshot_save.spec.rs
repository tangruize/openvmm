// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

// Human-owned snapshot-capture semantics for `LoadedVm` snapshot methods.

use vstd::prelude::*;

verus! {

pub struct ComponentId {
    pub value: Seq<char>,
}

pub enum UnitPhase {
    Running,
    Stopped,
    Saved,
    QuiesceUncertain,
}

pub struct SerializableGuestStateView {
    pub partition: int,
    pub vp_states: Map<nat, int>,
    pub component_state: Map<ComponentId, int>,
    pub virtual_time: int,
}

pub struct LiveRamView {
    pub bytes: Map<nat, u8>,
    pub layout: int,
}

pub struct BoundaryCapabilityView {
    pub identity: int,
    pub vps_held: bool,
    pub input_gated: bool,
}

pub struct LoadedVmCaptureView {
    pub serializable_state: SerializableGuestStateView,
    pub ram: LiveRamView,
    pub component_inventory: Seq<ComponentId>,
    pub unit_phase: Map<ComponentId, UnitPhase>,
    pub lifecycle_running: bool,
    pub units_running: bool,
    pub boundary: Option<BoundaryCapabilityView>,
}

pub struct SavedSnapshotStateView {
    pub serializable_state: SerializableGuestStateView,
    pub component_inventory: Seq<ComponentId>,
    pub payload_components: Seq<ComponentId>,
}

pub struct UnitSnapshotFragment {
    pub component: ComponentId,
    pub phase: UnitPhase,
    pub state: int,
    pub saved_payload: Option<int>,
}

/// Per-unit proof resources compose by disjoint component-name ownership.
pub struct UnitSnapshotPcm {
    pub fragments: Map<ComponentId, UnitSnapshotFragment>,
}

pub struct PostQuiesceCutView {
    pub serializable_state: SerializableGuestStateView,
    pub ram: LiveRamView,
    pub component_inventory: Seq<ComponentId>,
    pub boundary: BoundaryCapabilityView,
}

/// Ghost interpretation of the concrete `SnapshotSaveResponse`.
pub struct SnapshotSaveResponseView {
    pub saved_state: SavedSnapshotStateView,
    pub cut: PostQuiesceCutView,
    pub unit_pcm: UnitSnapshotPcm,
    pub mapped_ram_flushed: bool,
    pub command_line_matches: bool,
    pub tsc_frequency_matches: bool,
    pub apic_frequency_matches: bool,
    pub capture_wall_clock_matches: bool,
    pub cpu_contract_matches: bool,
}

pub uninterp spec fn snapshot_component(
    component: ComponentId,
    state: int,
) -> Option<int>;

pub open spec fn quiesce_transition(
    initial: LoadedVmCaptureView,
    cut: PostQuiesceCutView,
) -> bool {
    initial.lifecycle_running
    && initial.units_running
    && initial.boundary.is_some()
    && cut.boundary == initial.boundary.unwrap()
    && cut.boundary.vps_held
    && cut.boundary.input_gated
    && cut.component_inventory == initial.component_inventory
}

pub open spec fn saved_matches_cut(
    saved: SavedSnapshotStateView,
    cut: PostQuiesceCutView,
) -> bool {
    saved.component_inventory == cut.component_inventory
    && saved.serializable_state == cut.serializable_state
    && saved.payload_components.len() <= saved.component_inventory.len()
}

pub open spec fn captured_matches_cut(
    captured: LoadedVmCaptureView,
    cut: PostQuiesceCutView,
) -> bool {
    !captured.lifecycle_running
    && !captured.units_running
    && captured.boundary == Some(cut.boundary)
    && captured.serializable_state == cut.serializable_state
    && captured.ram == cut.ram
    && captured.component_inventory == cut.component_inventory
}

pub open spec fn unit_fragment_matches_cut(
    component: ComponentId,
    fragment: UnitSnapshotFragment,
    cut: PostQuiesceCutView,
) -> bool {
    fragment.component == component
    && fragment.phase == UnitPhase::Saved
    && cut.component_inventory.contains(component)
    && cut.serializable_state.component_state.dom().contains(component)
    && fragment.state
        == cut.serializable_state.component_state.index(component)
    && fragment.saved_payload
        == snapshot_component(component, fragment.state)
}

pub open spec fn unit_pcm_complete_for_cut(
    pcm: UnitSnapshotPcm,
    cut: PostQuiesceCutView,
) -> bool {
    forall |component: ComponentId|
        pcm.fragments.dom().contains(component)
            <==> cut.component_inventory.contains(component)
    && forall |component: ComponentId|
        cut.component_inventory.contains(component) ==>
            unit_fragment_matches_cut(
                component,
                pcm.fragments.index(component),
                cut,
            )
}

pub open spec fn snapshot_response_matches_cut(
    response: SnapshotSaveResponseView,
) -> bool {
    saved_matches_cut(response.saved_state, response.cut)
    && unit_pcm_complete_for_cut(response.unit_pcm, response.cut)
    && response.mapped_ram_flushed
    && response.command_line_matches
    && response.tsc_frequency_matches
    && response.apic_frequency_matches
    && response.capture_wall_clock_matches
    && response.cpu_contract_matches
}

// A successful return witnesses one post-quiesce cut. SavedState contains the
// serializable projection and ordered inventory; live RAM remains contextual.
pub open spec fn snapshot_state_capture_success(
    initial: LoadedVmCaptureView,
    saved: SavedSnapshotStateView,
    captured: LoadedVmCaptureView,
) -> bool {
    exists |cut: PostQuiesceCutView|
        quiesce_transition(initial, cut)
        && saved_matches_cut(saved, cut)
        && captured_matches_cut(captured, cut)
}

/// Direct success contract for the real
/// `LoadedVm::quiesce_for_snapshot` implementation.
pub open spec fn quiesce_for_snapshot_success(
    initial: LoadedVmCaptureView,
    response: SnapshotSaveResponseView,
    captured: LoadedVmCaptureView,
) -> bool {
    quiesce_transition(initial, response.cut)
    && snapshot_response_matches_cut(response)
    && captured_matches_cut(captured, response.cut)
}

} // verus!
