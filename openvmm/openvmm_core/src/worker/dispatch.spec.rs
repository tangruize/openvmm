use vstd::prelude::*;

verus! {

pub struct MachineView {
    pub architecture: nat,
    pub topology: Seq<nat>,
    pub memory_layout: Seq<(nat, nat)>,
}

pub struct RamView {
    pub bytes: Seq<u8>,
    pub shared_ranges: Seq<(nat, nat)>,
    pub confidential_ranges: Seq<(nat, nat)>,
}

pub struct VpView {
    pub identity: nat,
    pub architectural_state: Seq<u8>,
}

pub struct ComponentIdentity {
    pub value: nat,
}

/// Complete runtime state of one component. `runtime_only_fields` are rebuilt
/// for the destination and are not serialized in a snapshot.
pub struct ComponentStateView {
    pub identity: ComponentIdentity,
    pub snapshot_fields: Seq<u8>,
    pub runtime_only_fields: Seq<u8>,
}

/// The serializable state carried for one component.
pub struct ComponentSnapshotStateView {
    pub identity: ComponentIdentity,
    pub snapshot_fields: Seq<u8>,
}

pub struct ComponentStatesView {
    pub entries: Seq<ComponentStateView>,
}

pub struct ComponentSnapshotStatesView {
    pub entries: Seq<ComponentSnapshotStateView>,
}

impl ComponentStateView {
    pub open spec fn snapshot_state(self) -> ComponentSnapshotStateView {
        ComponentSnapshotStateView {
            identity: self.identity,
            snapshot_fields: self.snapshot_fields,
        }
    }
}

impl ComponentSnapshotStateView {
    pub open spec fn overlay_on(self, initialized: ComponentStateView) -> ComponentStateView
        recommends
            self.identity == initialized.identity,
    {
        ComponentStateView {
            identity: self.identity,
            snapshot_fields: self.snapshot_fields,
            runtime_only_fields: initialized.runtime_only_fields,
        }
    }
}

impl ComponentSnapshotStatesView {
    pub open spec fn captures(self, runtime: ComponentStatesView) -> bool {
        &&& self.entries.len() == runtime.entries.len()
        &&& forall|index: int| 0 <= index < self.entries.len() ==>
            self.entries[index] == runtime.entries[index].snapshot_state()
    }

    pub open spec fn overlays(
        self,
        initialized: ComponentStatesView,
        restored: ComponentStatesView,
    ) -> bool {
        &&& self.entries.len() == initialized.entries.len()
        &&& self.entries.len() == restored.entries.len()
        &&& forall|index: int| 0 <= index < self.entries.len() ==>
            self.entries[index].identity == initialized.entries[index].identity
            && restored.entries[index]
                == self.entries[index].overlay_on(initialized.entries[index])
    }

    pub open spec fn matches_inventory(self, inventory: Seq<ComponentIdentity>) -> bool {
        &&& self.entries.len() == inventory.len()
        &&& forall|index: int| 0 <= index < self.entries.len() ==>
            self.entries[index].identity == inventory[index]
    }
}

pub struct ClockView {
    pub guest_reference_time: int,
    pub partition_reference_time: int,
    pub tsc_reference_time: int,
}

/// One logical runtime VM state. Component runtime-only fields remain present
/// here and are deliberately absent from snapshot artifacts.
pub struct LogicalVmState {
    pub machine: MachineView,
    pub ram: RamView,
    pub vps: Seq<VpView>,
    pub components: ComponentStatesView,
    pub inventory: Seq<ComponentIdentity>,
    pub clocks: ClockView,
    pub generation_id: Seq<u8>,
    pub entropy_state: Seq<u8>,
}

pub struct CaptureBoundary {
    pub state_units_quiesced: bool,
    pub vps_stopped: bool,
    pub memory_copied_before_release: bool,
}

/// Complete published snapshot artifacts, including component snapshots rather
/// than complete runtime component states.
pub struct PublishedSnapshotView {
    pub machine: MachineView,
    pub ram: RamView,
    pub vps: Seq<VpView>,
    pub components: ComponentSnapshotStatesView,
    pub inventory: Seq<ComponentIdentity>,
    pub clocks: ClockView,
    pub generation_id: Seq<u8>,
    pub entropy_state: Seq<u8>,
}

/// The state-unit envelope encoded as `SavedState`. It is not the complete
/// published snapshot.
pub struct SavedStateEnvelopeView {
    pub components: ComponentSnapshotStatesView,
    pub inventory: Seq<ComponentIdentity>,
}

pub struct RestoreEnvironment {
    pub machine: MachineView,
    pub ram_backing_compatible: bool,
    pub initialized_components: ComponentStatesView,
    pub component_inventory: Seq<ComponentIdentity>,
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
    pub destination_ram_backing_compatible: bool,
    pub destination_components: ComponentStatesView,
    pub destination_component_inventory: Seq<ComponentIdentity>,
}

pub struct SnapshotSaveResponseView {
    pub snapshot: PublishedSnapshotView,
    pub boundary: CaptureBoundary,
}

impl PublishedSnapshotView {
    pub open spec fn saved_state_envelope(self) -> SavedStateEnvelopeView {
        SavedStateEnvelopeView {
            components: self.components,
            inventory: self.inventory,
        }
    }

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
        &&& self.ram == source_at_capture_cut.logical.ram
        &&& self.vps == source_at_capture_cut.logical.vps
        &&& self.components.captures(source_at_capture_cut.logical.components)
        &&& self.inventory == source_at_capture_cut.logical.inventory
        &&& self.components.matches_inventory(self.inventory)
        &&& self.clocks == source_at_capture_cut.logical.clocks
        &&& self.generation_id == source_at_capture_cut.logical.generation_id
        &&& self.entropy_state == source_at_capture_cut.logical.entropy_state
    }

    pub open spec fn restores_vps_as(
        self,
        requested_vp_count: Option<nat>,
        restored_vps: Seq<VpView>,
    ) -> bool {
        match requested_vp_count {
            Some(count) => restored_vps == self.vps.take(count as int),
            None => restored_vps == self.vps,
        }
    }

    pub open spec fn restores_into(
        self,
        environment: RestoreEnvironment,
        restored: RuntimeVmView,
    ) -> bool {
        &&& restored.logical.machine == environment.machine
        &&& restored.logical.ram == self.ram
        &&& self.restores_vps_as(
            environment.requested_vp_count,
            restored.logical.vps,
        )
        &&& self.components.overlays(
            environment.initialized_components,
            restored.logical.components,
        )
        &&& restored.logical.inventory == self.inventory
        &&& restored.logical.clocks == ClockView {
            guest_reference_time: self.clocks.guest_reference_time + environment.downtime,
            partition_reference_time: self.clocks.partition_reference_time
                + environment.downtime,
            tsc_reference_time: self.clocks.tsc_reference_time + environment.downtime,
        }
        &&& restored.logical.generation_id == match environment.generation_id_update {
            Some(value) => value,
            None => self.generation_id,
        }
        &&& restored.logical.entropy_state == match environment.entropy_reseed {
            Some(value) => value,
            None => self.entropy_state,
        }
        &&& !restored.guest_can_execute
        &&& !restored.externally_ready
    }
}

impl SavedStateEnvelopeView {
    pub open spec fn matches_snapshot(self, snapshot: PublishedSnapshotView) -> bool {
        self == snapshot.saved_state_envelope()
    }

    pub open spec fn restores_components(
        self,
        before: RuntimeVmView,
        after: RuntimeVmView,
    ) -> bool {
        &&& self.components.matches_inventory(self.inventory)
        &&& self.components.overlays(
            before.logical.components,
            after.logical.components,
        )
        &&& after.logical.machine == before.logical.machine
        &&& after.logical.ram == before.logical.ram
        &&& after.logical.vps == before.logical.vps
        &&& after.logical.inventory == self.inventory
        &&& after.logical.clocks == before.logical.clocks
        &&& after.logical.generation_id == before.logical.generation_id
        &&& after.logical.entropy_state == before.logical.entropy_state
        &&& after.guest_can_execute == before.guest_can_execute
        &&& after.externally_ready == before.externally_ready
    }
}

impl RestoreRequestView {
    pub open spec fn valid_for(self, initialized: InitializedVmView) -> bool {
        &&& self.environment.machine == initialized.destination_machine
        &&& self.environment.ram_backing_compatible
        &&& initialized.destination_ram_backing_compatible
        &&& self.environment.initialized_components
            == initialized.destination_components
        &&& self.environment.component_inventory
            == initialized.destination_component_inventory
        &&& self.snapshot.inventory == self.environment.component_inventory
        &&& self.snapshot.components.matches_inventory(self.snapshot.inventory)
        &&& match self.environment.requested_vp_count {
            Some(count) => count <= self.snapshot.vps.len(),
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
        saved: Option<SavedStateEnvelopeView>,
        loaded: RuntimeVmView,
    ) -> bool {
        &&& self.valid_for(initialized)
        &&& saved == Some(self.snapshot.saved_state_envelope())
        &&& self.snapshot.restores_into(self.environment, loaded)
    }
}

impl RuntimeVmView {
    pub open spec fn saves_as(self, saved: SavedStateEnvelopeView) -> bool {
        &&& !self.guest_can_execute
        &&& saved.components.captures(self.logical.components)
        &&& saved.inventory == self.logical.inventory
        &&& saved.components.matches_inventory(saved.inventory)
    }
}

impl SnapshotSaveResponseView {
    pub open spec fn quiesce_succeeds(self, stopped_vm: RuntimeVmView) -> bool {
        self.snapshot.captures(stopped_vm, self.boundary)
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
        request.snapshot.components.captures(source.logical.components),
        request.snapshot.components.overlays(
            request.environment.initialized_components,
            restored.logical.components,
        ),
        !restored.guest_can_execute,
        !restored.externally_ready,
{
}

}
