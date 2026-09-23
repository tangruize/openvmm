// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

// Human-owned open specification vocabulary for `dispatch.rs` snapshot restore.
//
// This module contains no executable restore implementation. Its abstract
// Views describe the pre/post-state of `LoadedVm::restore_snapshot_state`.
//
// Two kinds of state are kept apart throughout:
//
// - runtime state (`*StateView`): everything a live VM or component holds,
//   including destination configuration and host-bound runtime objects;
// - snapshot state (`*SnapshotStateView`, `VmSnapshotView`): only the part
//   that a snapshot serializes and that restore installs.
//
// Snapshot state is never obtained by selecting fields of the VM-level View.
// Each component owns its split: its runtime View carries its snapshot state
// separately from its non-snapshot configuration and host-runtime state.

use vstd::prelude::*;

verus! {

pub struct RamRegionView {
    pub base_gpa: nat,
    pub bytes: Seq<u8>,
}

pub struct RamView {
    pub regions: Seq<RamRegionView>,
}

// Destination-side attachment point, such as a configured disk slot or VMGS
// binding, to which a host resource is attached.
pub struct ResourceSlotId {
    pub value: int,
}

// Logical identity of the host backing object bound to a slot, such as the
// approved disk image. Host handle or descriptor values are not identities.
pub struct BackingResourceId {
    pub value: int,
}

pub struct ExternalResourcesView {
    pub bindings: Map<ResourceSlotId, BackingResourceId>,
}

// Host-managed runtime state may be created or changed while restore runs.
// It is part of the complete VM View so that concurrency and lifecycle
// specifications can observe it, but it is not snapshot state.
pub struct HostOperationalStateView {
    pub value: int,
}

pub struct CompatibilityClass {
    pub value: int,
}

pub struct VpStateView {
    pub value: int,
}

pub struct PartitionStateView {
    pub value: int,
}

pub struct VirtualTimeView {
    pub vm_time_100ns: u64,
    pub elapsed_since_snapshot_ns: nat,
}

pub struct ComponentId {
    pub value: int,
}

// The serialized state of one component: exactly what its saved-state blob
// carries and what its restore implementation installs.
pub struct ComponentSnapshotStateView {
    pub value: int,
}

// Destination-constructed component configuration, such as bindings, sizes,
// and wiring. It is not serialized and restore does not change it.
pub struct ComponentConfigView {
    pub value: int,
}

// Host-bound component runtime objects, such as tasks, timers, queue workers,
// and handles. They are not serialized; restore may rebuild them.
pub struct ComponentHostStateView {
    pub value: int,
}

// The complete runtime state of one live component. The component's View is
// responsible for placing every serialized field in `snapshot` and every
// other field in `config` or `host`.
pub struct ComponentStateView {
    pub config: ComponentConfigView,
    pub snapshot: ComponentSnapshotStateView,
    pub host: ComponentHostStateView,
}

pub struct ComponentStates {
    pub states: Map<ComponentId, ComponentStateView>,
}

pub struct ComponentSnapshotStates {
    pub states: Map<ComponentId, ComponentSnapshotStateView>,
}

pub struct VpStates {
    pub states: Map<nat, VpStateView>,
}

// The complete logical runtime state at the restore boundary.
pub struct VmStateView {
    pub memory: RamView,
    pub compatibility: CompatibilityClass,
    pub vp_capacity: nat,
    pub partition_state: PartitionStateView,
    pub vp_states: VpStates,
    pub component_inventory: Set<ComponentId>,
    // Live components whose state has been applied.
    pub active_components: ComponentStates,
    // Deferred components: saved state held until activation. Only snapshot
    // state exists for them before activation.
    pub pending_component_state: ComponentSnapshotStates,
    pub virtual_time: VirtualTimeView,
    pub resources: ExternalResourcesView,
    pub host_operational_state: HostOperationalStateView,
}

// The VM state carried by a snapshot's decoded `SavedState`, and equally the
// snapshot-owned part of a live VM. RAM is carried by the separately prepared
// memory file and is not part of this View.
pub struct VmSnapshotView {
    pub partition_state: PartitionStateView,
    pub vp_states: VpStates,
    pub component_inventory: Set<ComponentId>,
    pub active_component_state: ComponentSnapshotStates,
    pub pending_component_state: ComponentSnapshotStates,
    pub virtual_time: VirtualTimeView,
}

pub struct InitializedVmView {
    pub state: VmStateView,
    pub boot_online_vps: nat,
}

pub struct RestoreRequestView {
    pub snapshot: VmSnapshotView,
    pub selected_vp_count: nat,
    pub downtime_ns: nat,
    pub has_time_adjustment: bool,
}

pub enum VmExecutionPhase {
    PreparingRestore,
    PreExecutionRestored,
    ReadyToRun,
    Running,
}

pub struct LoadedVmView {
    pub state: VmStateView,
    pub active_vp_count: nat,
    pub execution_phase: VmExecutionPhase,
}

impl VirtualTimeView {
    // `VmTime::wrapping_add`: truncating 100ns downtime, wrapping to `u64`.
    pub open spec fn after_downtime(self, downtime_ns: nat) -> VirtualTimeView {
        VirtualTimeView {
            vm_time_100ns: ((self.vm_time_100ns as nat + downtime_ns / 100)
                % 0x1_0000_0000_0000_0000) as u64,
            elapsed_since_snapshot_ns: downtime_ns,
        }
    }
}

impl VpStates {
    // Selected VPs present in `saved` take the saved state; every other
    // destination VP keeps its initial/default state.
    pub open spec fn restore_selected(self, saved: VpStates, selected_vp_count: nat) -> VpStates {
        VpStates {
            states: Map::new(
                self.states.dom(),
                |vp_index: nat|
                    if vp_index < selected_vp_count && saved.states.dom().contains(vp_index) {
                        saved.states[vp_index]
                    } else {
                        self.states[vp_index]
                    },
            ),
        }
    }
}

impl ComponentStates {
    pub open spec fn snapshot(self) -> ComponentSnapshotStates {
        ComponentSnapshotStates {
            states: Map::new(
                self.states.dom(),
                |component: ComponentId| self.states[component].snapshot,
            ),
        }
    }

    pub open spec fn configs(self) -> Map<ComponentId, ComponentConfigView> {
        Map::new(self.states.dom(), |component: ComponentId| self.states[component].config)
    }
}

impl ComponentSnapshotStates {
    // Saved component snapshot state replaces the initial/default snapshot
    // state of the same component; components without saved state keep it.
    pub open spec fn overlay(self, saved: ComponentSnapshotStates) -> ComponentSnapshotStates {
        ComponentSnapshotStates {
            states: Map::new(
                self.states.dom(),
                |component: ComponentId|
                    if saved.states.dom().contains(component) {
                        saved.states[component]
                    } else {
                        self.states[component]
                    },
            ),
        }
    }
}

impl VmStateView {
    // The snapshot-owned part of the live VM, composed from each component's
    // own snapshot state.
    pub open spec fn snapshot(self) -> VmSnapshotView {
        VmSnapshotView {
            partition_state: self.partition_state,
            vp_states: self.vp_states,
            component_inventory: self.component_inventory,
            active_component_state: self.active_components.snapshot(),
            pending_component_state: self.pending_component_state,
            virtual_time: self.virtual_time,
        }
    }

    pub open spec fn has_stable_vp_identities(self) -> bool {
        forall |vp_index: nat|
            self.vp_states.states.dom().contains(vp_index) <==> vp_index < self.vp_capacity
    }

    // State that restore must leave unchanged: RAM installed by preparation,
    // the destination frame, and every component's configuration. Host-bound
    // component runtime state and host-operational state are unconstrained.
    pub open spec fn preserves_destination_of(self, initial: VmStateView) -> bool {
        self.memory == initial.memory
        && self.compatibility == initial.compatibility
        && self.vp_capacity == initial.vp_capacity
        && self.resources == initial.resources
        && self.active_components.configs() == initial.active_components.configs()
    }

    // The single definition of a successful restore result.
    pub open spec fn restores_to(
        self,
        request: RestoreRequestView,
        selected_vp_count: nat,
        restored: LoadedVmView,
    ) -> bool {
        restored.state.snapshot() == self.snapshot().restore_from(request, selected_vp_count)
        && restored.state.preserves_destination_of(self)
        && restored.active_vp_count == selected_vp_count
        && restored.execution_phase == VmExecutionPhase::PreExecutionRestored
    }
}

impl VmSnapshotView {
    // The snapshot state produced by restoring `request` onto `self`, the
    // destination's initial/default snapshot state.
    pub open spec fn restore_from(
        self,
        request: RestoreRequestView,
        selected_vp_count: nat,
    ) -> VmSnapshotView {
        VmSnapshotView {
            partition_state: request.snapshot.partition_state,
            vp_states: self.vp_states.restore_selected(
                request.snapshot.vp_states,
                selected_vp_count,
            ),
            component_inventory: request.snapshot.component_inventory,
            active_component_state: self.active_component_state.overlay(
                request.snapshot.active_component_state,
            ),
            pending_component_state: self.pending_component_state.overlay(
                request.snapshot.pending_component_state,
            ),
            virtual_time: request.restored_virtual_time(),
        }
    }
}

impl RestoreRequestView {
    pub open spec fn restored_virtual_time(self) -> VirtualTimeView {
        if self.has_time_adjustment {
            self.snapshot.virtual_time.after_downtime(self.downtime_ns)
        } else {
            VirtualTimeView {
                vm_time_100ns: self.snapshot.virtual_time.vm_time_100ns,
                elapsed_since_snapshot_ns: 0,
            }
        }
    }

    pub open spec fn valid_for_loaded_vm(self, initial: LoadedVmView) -> bool {
        initial.execution_phase == VmExecutionPhase::PreparingRestore
        && initial.active_vp_count <= initial.state.vp_capacity
        && self.selected_vp_count == initial.active_vp_count
        && initial.state.has_stable_vp_identities()
        && self.is_compatible_with(initial.state)
    }

    pub open spec fn valid_for_initialized_vm(self, initial: InitializedVmView) -> bool {
        initial.boot_online_vps <= self.selected_vp_count
        && self.selected_vp_count <= initial.state.vp_capacity
        && initial.state.has_stable_vp_identities()
        && self.is_compatible_with(initial.state)
    }

    pub open spec fn is_compatible_with(self, initial: VmStateView) -> bool {
        let saved = self.snapshot;
        saved.vp_states.states.dom().subset_of(initial.vp_states.states.dom())
        && saved.component_inventory == initial.component_inventory
        && saved.active_component_state.states.dom().subset_of(saved.component_inventory)
        && saved.pending_component_state.states.dom().subset_of(saved.component_inventory)
        && saved.active_component_state.states.dom()
            .subset_of(initial.active_components.states.dom())
        && saved.pending_component_state.states.dom()
            .subset_of(initial.pending_component_state.states.dom())
        && saved.virtual_time.elapsed_since_snapshot_ns == 0
    }
}

impl LoadedVmView {
    pub open spec fn snapshot_restore_success(
        self,
        request: RestoreRequestView,
        restored: LoadedVmView,
    ) -> bool {
        self.state.restores_to(request, self.active_vp_count, restored)
    }
}

impl InitializedVmView {
    pub open spec fn snapshot_load_success(
        self,
        request: RestoreRequestView,
        loaded: LoadedVmView,
    ) -> bool {
        self.state.restores_to(request, request.selected_vp_count, loaded)
    }
}

} // verus!
