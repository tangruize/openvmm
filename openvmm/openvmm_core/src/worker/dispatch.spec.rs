use vstd::prelude::*;

verus! {

pub struct MachineView {
    pub architecture: nat,
    pub topology: Seq<nat>,
    pub memory_layout: Seq<(nat, nat)>,
}

pub struct CompatibilityClass {
    pub value: int,
}

pub struct RamRegionView {
    pub base_gpa: nat,
    pub bytes: Seq<u8>,
}

pub struct RamView {
    pub regions: Seq<RamRegionView>,
}

pub struct VpStateView {
    pub identity: nat,
    pub architectural_state: Seq<u8>,
}

pub struct PartitionStateView {
    pub value: int,
}

pub type ComponentId = Seq<char>;

pub struct ComponentSnapshotStateView {
    pub value: int,
}

pub struct ComponentRuntimeContextView {
    pub value: int,
}

pub struct ComponentStateView {
    pub value: int,
}

impl ComponentStateView {
    pub uninterp spec fn runtime_context(self) -> ComponentRuntimeContextView;
}

impl ComponentSnapshotStateView {
    pub uninterp spec fn captures(self, runtime: ComponentStateView) -> bool;

    pub open spec fn overlays(
        self,
        initialized: ComponentStateView,
        restored: ComponentStateView,
    ) -> bool {
        self.captures(restored)
        && restored.runtime_context() == initialized.runtime_context()
    }
}

pub struct ComponentStatesView {
    pub active: Map<ComponentId, ComponentStateView>,
    pub pending: Map<ComponentId, ComponentStateView>,
}

pub struct ComponentSnapshotStatesView {
    pub active: Map<ComponentId, ComponentSnapshotStateView>,
    pub pending: Map<ComponentId, ComponentSnapshotStateView>,
}

impl ComponentStatesView {
    pub open spec fn matches_inventory(self, inventory: Seq<ComponentId>) -> bool {
        inventory.no_duplicates()
        && self.active.dom().subset_of(inventory.to_set())
        && self.pending.dom().subset_of(inventory.to_set())
    }
}

impl ComponentSnapshotStatesView {
    pub open spec fn captures(self, runtime: ComponentStatesView) -> bool {
        &&& self.active.dom() == runtime.active.dom()
        &&& self.pending.dom() == runtime.pending.dom()
        &&& forall|component: ComponentId| self.active.dom().contains(component) ==>
            self.active[component].captures(runtime.active[component])
        &&& forall|component: ComponentId| self.pending.dom().contains(component) ==>
            self.pending[component].captures(runtime.pending[component])
    }

    pub open spec fn overlays(
        self,
        initialized: ComponentStatesView,
        restored: ComponentStatesView,
    ) -> bool {
        &&& self.active.dom().subset_of(initialized.active.dom())
        &&& self.pending.dom().subset_of(initialized.pending.dom())
        &&& restored.active.dom() == initialized.active.dom()
        &&& restored.pending.dom() == initialized.pending.dom()
        &&& forall|component: ComponentId| initialized.active.dom().contains(component) ==>
            if self.active.dom().contains(component) {
                self.active[component].overlays(
                    initialized.active[component],
                    restored.active[component],
                )
            } else {
                restored.active[component] == initialized.active[component]
            }
        &&& forall|component: ComponentId| initialized.pending.dom().contains(component) ==>
            if self.pending.dom().contains(component) {
                self.pending[component].overlays(
                    initialized.pending[component],
                    restored.pending[component],
                )
            } else {
                restored.pending[component] == initialized.pending[component]
            }
    }

    pub open spec fn matches_inventory(self, inventory: Seq<ComponentId>) -> bool {
        inventory.no_duplicates()
        && self.active.dom().subset_of(inventory.to_set())
        && self.pending.dom().subset_of(inventory.to_set())
    }
}

pub struct ClockView {
    pub guest_reference_time: int,
    pub partition_reference_time: int,
    pub tsc_reference_time: int,
}

pub struct ExternalResourcesView {
    pub value: int,
}

pub struct HostOperationalStateView {
    pub value: int,
}

/// Complete runtime VM state.
pub struct LogicalVmState {
    pub machine: MachineView,
    pub compatibility: CompatibilityClass,
    pub ram: RamView,
    pub partition_state: PartitionStateView,
    pub vps: Seq<VpStateView>,
    pub component_inventory: Seq<ComponentId>,
    pub components: ComponentStatesView,
    pub clocks: ClockView,
    pub generation_id: Seq<u8>,
    pub entropy_state: Seq<u8>,
    pub resources: ExternalResourcesView,
    pub host_operational_state: HostOperationalStateView,
}

pub struct CaptureBoundary {
    pub state_units_quiesced: bool,
    pub vps_stopped: bool,
    pub memory_copied_before_release: bool,
}

/// The state-unit envelope encoded as `SavedState`.
pub struct SavedStateEnvelopeView {
    pub partition_state: PartitionStateView,
    pub vps: Seq<VpStateView>,
    pub component_inventory: Seq<ComponentId>,
    pub components: ComponentSnapshotStatesView,
    pub clocks: ClockView,
}

impl SavedStateEnvelopeView {
    pub open spec fn structurally_valid(self) -> bool {
        &&& self.component_inventory.no_duplicates()
        &&& self.components.matches_inventory(self.component_inventory)
    }
}

/// Complete published snapshot artifacts.
pub struct PublishedSnapshotView {
    pub machine: MachineView,
    pub compatibility: CompatibilityClass,
    pub ram: RamView,
    pub saved_state: SavedStateEnvelopeView,
    pub generation_id: Seq<u8>,
    pub entropy_state: Seq<u8>,
}

pub struct RestoreEnvironment {
    pub machine: MachineView,
    pub compatibility: CompatibilityClass,
    pub ram_backing_compatible: bool,
    pub initialized_components: ComponentStatesView,
    pub component_inventory: Seq<ComponentId>,
    pub resources: ExternalResourcesView,
    pub host_operational_state: HostOperationalStateView,
    pub requested_vp_count: Option<nat>,
    pub downtime: int,
    pub generation_id_update: Option<Seq<u8>>,
    pub entropy_reseed: Option<Seq<u8>>,
}

pub struct RestoreRequestView {
    pub snapshot: PublishedSnapshotView,
    pub environment: RestoreEnvironment,
}

pub struct RuntimeVmView {
    pub logical: LogicalVmState,
    pub prepared_restore: Option<RestoreRequestView>,
    pub guest_can_execute: bool,
    pub externally_ready: bool,
}

pub struct InitializedVmView {
    pub destination_machine: MachineView,
    pub destination_compatibility: CompatibilityClass,
    pub destination_ram_backing_compatible: bool,
    pub destination_components: ComponentStatesView,
    pub destination_component_inventory: Seq<ComponentId>,
    pub destination_resources: ExternalResourcesView,
    pub destination_host_operational_state: HostOperationalStateView,
}

pub struct SnapshotSaveResponseView {
    pub snapshot: PublishedSnapshotView,
    pub boundary: CaptureBoundary,
    pub state_unit_names: Seq<ComponentId>,
    pub effective_command_line: Seq<char>,
}

impl PublishedSnapshotView {
    pub open spec fn captures(
        self,
        source_at_capture_cut: RuntimeVmView,
        boundary: CaptureBoundary,
    ) -> bool {
        &&& !source_at_capture_cut.guest_can_execute
        &&& boundary.state_units_quiesced
        &&& boundary.vps_stopped
        &&& boundary.memory_copied_before_release
        &&& self.machine == source_at_capture_cut.logical.machine
        &&& self.compatibility == source_at_capture_cut.logical.compatibility
        &&& self.ram == source_at_capture_cut.logical.ram
        &&& self.saved_state.partition_state
            == source_at_capture_cut.logical.partition_state
        &&& self.saved_state.vps == source_at_capture_cut.logical.vps
        &&& self.saved_state.component_inventory
            == source_at_capture_cut.logical.component_inventory
        &&& self.saved_state.components.captures(
            source_at_capture_cut.logical.components,
        )
        &&& self.saved_state.structurally_valid()
        &&& self.saved_state.clocks == source_at_capture_cut.logical.clocks
        &&& self.generation_id == source_at_capture_cut.logical.generation_id
        &&& self.entropy_state == source_at_capture_cut.logical.entropy_state
    }

    pub open spec fn restores_vps_as(
        self,
        requested_vp_count: Option<nat>,
        restored_vps: Seq<VpStateView>,
    ) -> bool {
        match requested_vp_count {
            Some(count) => restored_vps == self.saved_state.vps.take(count as int),
            None => restored_vps == self.saved_state.vps,
        }
    }

    pub open spec fn restores_into(
        self,
        environment: RestoreEnvironment,
        restored: RuntimeVmView,
    ) -> bool {
        &&& restored.logical.machine == environment.machine
        &&& restored.logical.compatibility == environment.compatibility
        &&& restored.logical.ram == self.ram
        &&& restored.logical.partition_state == self.saved_state.partition_state
        &&& self.restores_vps_as(
            environment.requested_vp_count,
            restored.logical.vps,
        )
        &&& self.saved_state.components.overlays(
            environment.initialized_components,
            restored.logical.components,
        )
        &&& restored.logical.component_inventory
            == self.saved_state.component_inventory
        &&& restored.logical.clocks == ClockView {
            guest_reference_time: self.saved_state.clocks.guest_reference_time
                + environment.downtime,
            partition_reference_time: self.saved_state.clocks.partition_reference_time
                + environment.downtime,
            tsc_reference_time: self.saved_state.clocks.tsc_reference_time
                + environment.downtime,
        }
        &&& restored.logical.generation_id == match environment.generation_id_update {
            Some(value) => value,
            None => self.generation_id,
        }
        &&& restored.logical.entropy_state == match environment.entropy_reseed {
            Some(value) => value,
            None => self.entropy_state,
        }
        &&& restored.logical.resources == environment.resources
        &&& restored.logical.host_operational_state
            == environment.host_operational_state
        &&& !restored.guest_can_execute
        &&& !restored.externally_ready
    }
}

impl SavedStateEnvelopeView {
    pub open spec fn matches_snapshot(self, snapshot: PublishedSnapshotView) -> bool {
        self == snapshot.saved_state
    }

    pub open spec fn restores_components(
        self,
        before: RuntimeVmView,
        after: RuntimeVmView,
    ) -> bool {
        &&& self.structurally_valid()
        &&& self.components.overlays(
            before.logical.components,
            after.logical.components,
        )
        &&& after.logical.machine == before.logical.machine
        &&& after.logical.compatibility == before.logical.compatibility
        &&& after.logical.ram == before.logical.ram
        &&& after.logical.partition_state == self.partition_state
        &&& after.logical.vps == self.vps
        &&& after.logical.component_inventory == self.component_inventory
        &&& after.logical.clocks == self.clocks
        &&& after.logical.generation_id == before.logical.generation_id
        &&& after.logical.entropy_state == before.logical.entropy_state
        &&& after.logical.resources == before.logical.resources
        &&& after.logical.host_operational_state
            == before.logical.host_operational_state
        &&& after.guest_can_execute == before.guest_can_execute
        &&& after.externally_ready == before.externally_ready
    }
}

impl RestoreRequestView {
    pub open spec fn valid_for(self, initialized: InitializedVmView) -> bool {
        &&& self.environment.machine == initialized.destination_machine
        &&& self.environment.compatibility
            == initialized.destination_compatibility
        &&& self.snapshot.compatibility == self.environment.compatibility
        &&& self.environment.ram_backing_compatible
        &&& initialized.destination_ram_backing_compatible
        &&& self.environment.initialized_components
            == initialized.destination_components
        &&& self.environment.component_inventory
            == initialized.destination_component_inventory
        &&& self.environment.resources == initialized.destination_resources
        &&& self.environment.host_operational_state
            == initialized.destination_host_operational_state
        &&& self.snapshot.saved_state.component_inventory
            == self.environment.component_inventory
        &&& self.snapshot.saved_state.structurally_valid()
        &&& self.environment.initialized_components.matches_inventory(
            self.environment.component_inventory,
        )
        &&& match self.environment.requested_vp_count {
            Some(count) => count <= self.snapshot.saved_state.vps.len(),
            None => true,
        }
    }

    pub open spec fn restore_succeeds(
        self,
        before: RuntimeVmView,
        saved: SavedStateEnvelopeView,
        restored: RuntimeVmView,
    ) -> bool {
        &&& before.prepared_restore == Some(self)
        &&& saved.matches_snapshot(self.snapshot)
        &&& self.snapshot.restores_into(self.environment, restored)
    }

    pub open spec fn load_succeeds(
        self,
        initialized: InitializedVmView,
        saved: SavedStateEnvelopeView,
        loaded: RuntimeVmView,
    ) -> bool {
        &&& self.valid_for(initialized)
        &&& saved.matches_snapshot(self.snapshot)
        &&& self.snapshot.restores_into(self.environment, loaded)
    }
}

impl RuntimeVmView {
    pub open spec fn saves_as(self, saved: SavedStateEnvelopeView) -> bool {
        &&& !self.guest_can_execute
        &&& saved.partition_state == self.logical.partition_state
        &&& saved.vps == self.logical.vps
        &&& saved.component_inventory == self.logical.component_inventory
        &&& saved.components.captures(self.logical.components)
        &&& saved.structurally_valid()
        &&& saved.clocks == self.logical.clocks
    }
}

impl SnapshotSaveResponseView {
    pub open spec fn quiesce_succeeds(self, stopped_vm: RuntimeVmView) -> bool {
        &&& self.state_unit_names == self.snapshot.saved_state.component_inventory
        &&& self.snapshot.captures(stopped_vm, self.boundary)
    }
}

pub proof fn capture_then_restore_composes(
    source: RuntimeVmView,
    response: SnapshotSaveResponseView,
    request: RestoreRequestView,
    restored: RuntimeVmView,
)
    requires
        response.quiesce_succeeds(source),
        request.snapshot == response.snapshot,
        request.snapshot.restores_into(request.environment, restored),
    ensures
        request.snapshot.saved_state.components.captures(
            source.logical.components,
        ),
        request.snapshot.saved_state.components.overlays(
            request.environment.initialized_components,
            restored.logical.components,
        ),
        restored.logical.ram == source.logical.ram,
        restored.logical.partition_state == source.logical.partition_state,
        request.environment.requested_vp_count.is_none() ==>
            restored.logical.vps == source.logical.vps,
        restored.logical.clocks == (ClockView {
            guest_reference_time: source.logical.clocks.guest_reference_time
                + request.environment.downtime,
            partition_reference_time: source.logical.clocks.partition_reference_time
                + request.environment.downtime,
            tsc_reference_time: source.logical.clocks.tsc_reference_time
                + request.environment.downtime,
        }),
        !restored.guest_can_execute,
        !restored.externally_ready,
{
}

}
