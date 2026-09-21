// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

// Engineer-owned closed specifications and proof lemmas for `dispatch.rs`.
//
// Keep Human-owned restore semantics in `dispatch.spec.rs`. Add substantial
// proof functions here as frontend limitations are removed.

use super::InitializedVm;
use super::LoadedVm;
use super::restore_spec::ComponentId;
use super::restore_spec::ComponentStateView;
use super::restore_spec::InitializedVmView;
use super::restore_spec::LoadedVmView;
use super::restore_spec::PartitionStateView;
use super::restore_spec::RestoreRequestView;
use super::restore_spec::SavedVmStateView;
use super::restore_spec::SnapshotVmStateView;
use super::restore_spec::VirtualTimeView;
use super::restore_spec::VpStateView;
use super::restore_spec::restore_snapshot_projection;
use super::restore_spec::restored_virtual_time;
use super::restore_spec::snapshot_restore_success;
use openvmm_defs::worker::SavedState;
use openvmm_defs::worker::saved_state_proof::component_name_id;
use openvmm_defs::worker::saved_state_proof::component_name_identity;
use openvmm_defs::worker::saved_state_proof::saved_inventory_empty;
use openvmm_defs::worker::saved_state_proof::saved_inventory_id_membership;
use openvmm_defs::worker::saved_state_proof::saved_inventory_ids;
use openvmm_defs::worker::saved_state_proof::saved_inventory_membership;
use state_unit::SavedStateUnit;
use std::time::Duration;
use vmcore::vmtime::duration_observation::elapsed_nanoseconds;
use vstd::prelude::*;

verus! {

// Only the carried type is opaque; no file operations are specified or trusted.
#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExRestoreReadyFile(std::fs::File);

pub struct SavedPayloadView {
    pub partition_state: PartitionStateView,
    pub vp_states: Map<nat, VpStateView>,
    pub active_component_state: Map<ComponentId, ComponentStateView>,
    pub pending_component_state: Map<ComponentId, ComponentStateView>,
    pub virtual_time: VirtualTimeView,
}

// TODO(uninterp): Decode the named protobuf blobs consumed by StateUnits::restore
// into partition, VP, mutable component, and saved virtual-time state.
// Complete inventory is not derived from these optional payloads.
pub uninterp spec fn decoded_saved_payload_view(units: &Vec<SavedStateUnit>) -> SavedPayloadView;

pub closed spec fn saved_component_inventory(saved_state: &SavedState) -> Set<ComponentId> {
    saved_inventory_ids(saved_state).map(|id: nat| ComponentId { value: id as int })
}

pub proof fn saved_component_inventory_membership(saved_state: &SavedState, component: ComponentId)
    ensures
        saved_component_inventory(saved_state).contains(component)
            <==> exists|i: int| 0 <= i < saved_state.inventory@.len()
                && #[trigger] component_name_id(saved_state.inventory@[i]@) as int == component.value,
{
    reveal(saved_component_inventory);
    if saved_component_inventory(saved_state).contains(component) {
        let id = choose|id: nat| #[trigger] saved_inventory_ids(saved_state).contains(id)
            && component == ComponentId { value: id as int };
        saved_inventory_id_membership(saved_state, id);
    } else if exists|i: int| 0 <= i < saved_state.inventory@.len()
        && #[trigger] component_name_id(saved_state.inventory@[i]@) as int == component.value {
        let i = choose|i: int| 0 <= i < saved_state.inventory@.len()
            && #[trigger] component_name_id(saved_state.inventory@[i]@) as int == component.value;
        let id = component_name_id(saved_state.inventory@[i]@);
        saved_inventory_id_membership(saved_state, id);
        assert(saved_inventory_ids(saved_state).contains(id));
    }
}

pub closed spec fn decoded_saved_state_view(saved_state: &SavedState) -> SavedVmStateView {
    let payload = decoded_saved_payload_view(&saved_state.units);
    SavedVmStateView {
        partition_state: payload.partition_state,
        vp_states: payload.vp_states,
        component_inventory: saved_component_inventory(saved_state),
        active_component_state: payload.active_component_state,
        pending_component_state: payload.pending_component_state,
        virtual_time: payload.virtual_time,
    }
}

pub closed spec fn decoded_restore_request_view(
    saved_state: &SavedState,
    restore_time: &Option<(Duration, u64, Option<u64>)>,
    selected_vp_count: nat,
) -> RestoreRequestView {
    RestoreRequestView {
        saved_state: decoded_saved_state_view(saved_state),
        selected_vp_count,
        downtime_ns: match *restore_time {
            Some((downtime, _, _)) => elapsed_nanoseconds(&downtime),
            None => 0,
        },
        has_time_adjustment: restore_time.is_some(),
    }
}

pub proof fn decoded_restore_inventory_membership(
    saved_state: &SavedState,
    restore_time: &Option<(Duration, u64, Option<u64>)>,
    selected_vp_count: nat,
    name: Seq<char>,
)
    ensures
        decoded_restore_request_view(saved_state, restore_time, selected_vp_count)
            .saved_state.component_inventory
            .contains(ComponentId { value: component_name_id(name) as int })
            <==> exists|i: int| 0 <= i < saved_state.inventory@.len()
                && #[trigger] saved_state.inventory@[i]@ == name,
        decoded_restore_request_view(saved_state, restore_time, selected_vp_count)
            .saved_state.component_inventory == saved_component_inventory(saved_state),
{
    reveal(decoded_restore_request_view);
    reveal(decoded_saved_state_view);
    reveal(saved_component_inventory);
    saved_inventory_membership(saved_state, name);
}

pub proof fn component_identity_preserves_names(left: Seq<char>, right: Seq<char>)
    ensures
        (ComponentId { value: component_name_id(left) as int }
            == ComponentId { value: component_name_id(right) as int })
            <==> left == right,
{
    component_name_identity(left, right);
}

pub proof fn restore_projection_inventory(
    initial: SnapshotVmStateView,
    saved_state: &SavedState,
    restore_time: &Option<(Duration, u64, Option<u64>)>,
    selected_vp_count: nat,
    name: Seq<char>,
)
    ensures
        restore_snapshot_projection(
            initial,
            decoded_restore_request_view(saved_state, restore_time, selected_vp_count),
            selected_vp_count,
        ).component_inventory == saved_component_inventory(saved_state),
        restore_snapshot_projection(
            initial,
            decoded_restore_request_view(saved_state, restore_time, selected_vp_count),
            selected_vp_count,
        ).component_inventory.contains(ComponentId { value: component_name_id(name) as int })
            <==> exists|i: int| 0 <= i < saved_state.inventory@.len()
                && #[trigger] saved_state.inventory@[i]@ == name,
{
    decoded_restore_inventory_membership(saved_state, restore_time, selected_vp_count, name);
}

pub proof fn snapshot_restore_success_inventory(
    initial: LoadedVmView,
    saved_state: &SavedState,
    restore_time: &Option<(Duration, u64, Option<u64>)>,
    restored: LoadedVmView,
    name: Seq<char>,
)
    requires
        snapshot_restore_success(
            initial,
            decoded_restore_request_view(saved_state, restore_time, initial.active_vp_count),
            restored,
        ),
    ensures
        restored.state.component_inventory == saved_component_inventory(saved_state),
        restored.state.component_inventory.contains(
            ComponentId { value: component_name_id(name) as int },
        ) <==> exists|i: int| 0 <= i < saved_state.inventory@.len()
            && #[trigger] saved_state.inventory@[i]@ == name,
{
    decoded_restore_inventory_membership(saved_state, restore_time, initial.active_vp_count, name);
}

pub proof fn decoded_restore_empty_inventory(
    initial: LoadedVmView,
    saved_state: &SavedState,
    restore_time: &Option<(Duration, u64, Option<u64>)>,
)
    requires saved_state.inventory@.len() == 0,
    ensures
        decoded_restore_request_view(saved_state, restore_time, initial.active_vp_count)
            .saved_state.component_inventory == Set::<ComponentId>::empty(),
        decoded_restore_request_view(saved_state, restore_time, initial.active_vp_count)
            .valid_for_loaded_vm(initial)
            ==> initial.state.component_inventory == Set::<ComponentId>::empty(),
{
    reveal(decoded_restore_request_view);
    reveal(decoded_saved_state_view);
    reveal(saved_component_inventory);
    saved_inventory_empty(saved_state);
    assert(saved_component_inventory(saved_state) =~= Set::<ComponentId>::empty());
}

pub proof fn decoded_restore_request_time_policy(
    saved_state: &SavedState,
    restore_time: &Option<(Duration, u64, Option<u64>)>,
    selected_vp_count: nat,
)
    ensures
        decoded_restore_request_view(saved_state, restore_time, selected_vp_count)
            .has_time_adjustment == restore_time.is_some(),
        match *restore_time {
            Some((downtime, _, _)) =>
                decoded_restore_request_view(saved_state, restore_time, selected_vp_count)
                    .downtime_ns == elapsed_nanoseconds(&downtime),
            None =>
                decoded_restore_request_view(saved_state, restore_time, selected_vp_count)
                    .downtime_ns == 0,
        },
        {
            let request = decoded_restore_request_view(saved_state, restore_time, selected_vp_count);
            restored_virtual_time(request.saved_state.virtual_time, request)
                .elapsed_since_snapshot_ns == request.downtime_ns
        },
{
    reveal(decoded_restore_request_view);
}

pub proof fn snapshot_restore_success_elapsed_time(
    initial: LoadedVmView,
    saved_state: &SavedState,
    restore_time: &Option<(Duration, u64, Option<u64>)>,
    restored: LoadedVmView,
)
    requires
        snapshot_restore_success(
            initial,
            decoded_restore_request_view(saved_state, restore_time, initial.active_vp_count),
            restored,
        ),
    ensures
        match *restore_time {
            Some((downtime, _, _)) =>
                restored.state.virtual_time.elapsed_since_snapshot_ns
                    == elapsed_nanoseconds(&downtime),
            None => restored.state.virtual_time.elapsed_since_snapshot_ns == 0,
        },
{
    decoded_restore_request_time_policy(saved_state, restore_time, initial.active_vp_count);
}

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

pub closed spec fn pre_execution_representation(
    loaded: &LoadedVm,
    restored_from_snapshot: bool,
) -> bool {
    loaded.restored_from_snapshot == restored_from_snapshot
    && loaded.restore_start_guard.is_some() == restored_from_snapshot
    && !loaded.running
}

} // verus!
