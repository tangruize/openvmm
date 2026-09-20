// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

#[path = "../../openvmm/openvmm_core/src/worker/dispatch.spec.rs"]
mod frozen;

use frozen::*;
use vstd::prelude::*;

verus! {

proof fn omitted_vp_is_admitted_and_preserved()
{
    let inventory = Set::empty().insert(ComponentId { value: 1 });
    let initial = LoadedVmView {
        state: VmStateView {
            memory: RamView { regions: Seq::empty() },
            compatibility: CompatibilityClass { value: 0 },
            vp_capacity: 1,
            partition_state: PartitionStateView { value: 0 },
            vp_states: Map::empty().insert(0nat, VpStateView { value: 77 }),
            component_inventory: inventory,
            active_component_state: Map::empty(),
            pending_component_state: Map::empty(),
            virtual_time: VirtualTimeView {
                vm_time_100ns: 0,
                elapsed_since_snapshot_ns: 0,
            },
            resources: ExternalResourcesView { attachments: Map::empty() },
            host_operational_state: HostOperationalStateView { value: 0 },
        },
        active_vp_count: 1,
        execution_phase: VmExecutionPhase::PreparingRestore,
    };
    let request = RestoreRequestView {
        saved_state: SavedVmStateView {
            partition_state: initial.state.partition_state,
            vp_states: Map::empty(),
            component_inventory: inventory,
            active_component_state: Map::empty(),
            pending_component_state: Map::empty(),
            virtual_time: initial.state.virtual_time,
        },
        selected_vp_count: 1,
        downtime_ns: 250_000_000,
        has_time_adjustment: true,
    };
    assert(request.valid_for_loaded_vm(initial));
    let projection = restore_snapshot_projection(snapshot_state(initial.state), request, 1);
    assert(projection.vp_states.dom().contains(0nat));
    assert(projection.vp_states[0nat].value == 77);
    assert(projection.vp_states[0nat].value != 1077);
}

} // verus!

fn main() {}
