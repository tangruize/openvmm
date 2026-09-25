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
    pub boundary_owned: bool,
    pub source_quiescent: bool,
    pub publication_committed: bool,
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
        initial.protocol.phase == SnapshotPhase::BoundaryEstablished
        && snapshot_atomic_invariant(initial.protocol)
        && snapshot_atomic_invariant(final_state.protocol)
        && final_state.protocol.epoch == initial.protocol.epoch
        && final_state.protocol.request_state == initial.protocol.request_state
        && final_state.protocol.phase == SnapshotPhase::Published
        && final_state.protocol.cut.is_some()
        && final_state.protocol.artifact.is_some()
        && final_state.protocol.artifact.unwrap().scratch_policy
            == requested_scratch_policy
        && pcm_agrees_with_authority(
            final_state.collected_units,
            final_state.protocol,
        )
        && pcm_complete_for_cut(
            final_state.collected_units,
            final_state.protocol.cut.unwrap(),
        )
        && final_state.boundary_owned
        && final_state.source_quiescent
        && final_state.publication_committed
}

} // verus!
