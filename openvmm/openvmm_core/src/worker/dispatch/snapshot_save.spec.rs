// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

// Human-owned snapshot-capture semantics for `LoadedVm::capture_snapshot_state`.

use vstd::prelude::*;

verus! {

pub struct ComponentId {
    pub value: Seq<char>,
}

pub enum UnitPhase {
    Running,
    Stopped,
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

pub struct PostQuiesceCutView {
    pub serializable_state: SerializableGuestStateView,
    pub ram: LiveRamView,
    pub component_inventory: Seq<ComponentId>,
    pub boundary: BoundaryCapabilityView,
}

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

} // verus!
