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

pub struct ComponentView {
    pub identity: nat,
    pub payload: Seq<u8>,
}

pub struct ClockView {
    pub guest_reference_time: int,
    pub partition_reference_time: int,
    pub tsc_reference_time: int,
}

pub struct LogicalVmState {
    pub machine: MachineView,
    pub ram: RamView,
    pub vps: Seq<VpView>,
    pub components: Seq<ComponentView>,
    pub inventory: Seq<nat>,
    pub clocks: ClockView,
    pub generation_id: Seq<u8>,
    pub entropy_state: Seq<u8>,
}

pub struct CaptureBoundary {
    pub state_units_quiesced: bool,
    pub vps_stopped: bool,
    pub memory_copied_before_release: bool,
}

pub struct SnapshotArtifactsView {
    pub machine: MachineView,
    pub ram: RamView,
    pub vps: Seq<VpView>,
    pub components: Seq<ComponentView>,
    pub inventory: Seq<nat>,
    pub clocks: ClockView,
    pub generation_id: Seq<u8>,
    pub entropy_state: Seq<u8>,
}

pub struct SavedEnvelopeView {
    pub components: Seq<ComponentView>,
    pub inventory: Seq<nat>,
}

pub struct RestoreEnvironment {
    pub machine: MachineView,
    pub ram_backing_compatible: bool,
    pub component_inventory: Seq<nat>,
    pub requested_vp_count: Option<nat>,
    pub downtime: int,
    pub generation_id_update: Option<Seq<u8>>,
    pub entropy_reseed: Option<Seq<u8>>,
}

pub struct RestoreRequestView {
    pub snapshot: SnapshotArtifactsView,
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
    pub destination_component_inventory: Seq<nat>,
}

pub struct SnapshotSaveResponseView {
    pub snapshot: SnapshotArtifactsView,
    pub boundary: CaptureBoundary,
}

pub open spec fn inventory_matches(
    inventory: Seq<nat>,
    components: Seq<ComponentView>,
) -> bool {
    &&& inventory.len() == components.len()
    &&& forall|index: int| 0 <= index < inventory.len() ==>
        inventory[index] == components[index].identity
}

pub open spec fn vp_prefix(vps: Seq<VpView>, count: Option<nat>) -> Seq<VpView> {
    match count {
        Some(n) => vps.take(n as int),
        None => vps,
    }
}

pub open spec fn snapshot_view(snapshot: SnapshotArtifactsView) -> LogicalVmState {
    LogicalVmState {
        machine: snapshot.machine,
        ram: snapshot.ram,
        vps: snapshot.vps,
        components: snapshot.components,
        inventory: snapshot.inventory,
        clocks: snapshot.clocks,
        generation_id: snapshot.generation_id,
        entropy_state: snapshot.entropy_state,
    }
}

pub open spec fn saved_envelope(snapshot: SnapshotArtifactsView) -> SavedEnvelopeView {
    SavedEnvelopeView {
        components: snapshot.components,
        inventory: snapshot.inventory,
    }
}

pub open spec fn capture_relation(
    source_at_capture_cut: RuntimeVmView,
    snapshot: SnapshotArtifactsView,
    boundary: CaptureBoundary,
) -> bool {
    &&& !source_at_capture_cut.guest_can_execute
    &&& boundary.state_units_quiesced
    &&& boundary.vps_stopped
    &&& boundary.memory_copied_before_release
    &&& snapshot_view(snapshot) == source_at_capture_cut.logical
    &&& inventory_matches(snapshot.inventory, snapshot.components)
}

pub open spec fn allowed_restore_transform(
    snapshot: SnapshotArtifactsView,
    environment: RestoreEnvironment,
) -> LogicalVmState {
    LogicalVmState {
        machine: environment.machine,
        ram: snapshot.ram,
        vps: vp_prefix(snapshot.vps, environment.requested_vp_count),
        components: snapshot.components,
        inventory: snapshot.inventory,
        clocks: ClockView {
            guest_reference_time: snapshot.clocks.guest_reference_time + environment.downtime,
            partition_reference_time: snapshot.clocks.partition_reference_time
                + environment.downtime,
            tsc_reference_time: snapshot.clocks.tsc_reference_time + environment.downtime,
        },
        generation_id: match environment.generation_id_update {
            Some(value) => value,
            None => snapshot.generation_id,
        },
        entropy_state: match environment.entropy_reseed {
            Some(value) => value,
            None => snapshot.entropy_state,
        },
    }
}

impl RestoreRequestView {
    pub open spec fn valid_for(
        self,
        initialized: InitializedVmView,
    ) -> bool {
        &&& self.environment.machine == initialized.destination_machine
        &&& self.environment.ram_backing_compatible
        &&& initialized.destination_ram_backing_compatible
        &&& self.environment.component_inventory
            == initialized.destination_component_inventory
        &&& inventory_matches(self.snapshot.inventory, self.snapshot.components)
        &&& self.snapshot.inventory == self.environment.component_inventory
        &&& match self.environment.requested_vp_count {
            Some(count) => count <= self.snapshot.vps.len(),
            None => true,
        }
    }
}

pub open spec fn restore_relation(
    request: RestoreRequestView,
    restored: RuntimeVmView,
) -> bool {
    &&& restored.logical
        == allowed_restore_transform(request.snapshot, request.environment)
    &&& !restored.guest_can_execute
    &&& !restored.externally_ready
}

pub open spec fn save_success(
    stopped_vm: RuntimeVmView,
    saved: SavedEnvelopeView,
) -> bool {
    &&& !stopped_vm.guest_can_execute
    &&& saved.components == stopped_vm.logical.components
    &&& saved.inventory == stopped_vm.logical.inventory
    &&& inventory_matches(saved.inventory, saved.components)
}

pub open spec fn component_restore_success(
    before: RuntimeVmView,
    saved: SavedEnvelopeView,
    after: RuntimeVmView,
) -> bool {
    &&& inventory_matches(saved.inventory, saved.components)
    &&& after.logical.machine == before.logical.machine
    &&& after.logical.ram == before.logical.ram
    &&& after.logical.vps == before.logical.vps
    &&& after.logical.components == saved.components
    &&& after.logical.inventory == saved.inventory
    &&& after.logical.clocks == before.logical.clocks
    &&& after.logical.generation_id == before.logical.generation_id
    &&& after.logical.entropy_state == before.logical.entropy_state
    &&& after.guest_can_execute == before.guest_can_execute
    &&& after.externally_ready == before.externally_ready
}

pub open spec fn restore_success(
    before: RuntimeVmView,
    request: RestoreRequestView,
    saved: SavedEnvelopeView,
    restored: RuntimeVmView,
) -> bool {
    &&& before.prepared_restore == Some(request)
    &&& saved == saved_envelope(request.snapshot)
    &&& saved.inventory == request.environment.component_inventory
    &&& restore_relation(request, restored)
}

pub open spec fn load_restore_success(
    initialized: InitializedVmView,
    request: RestoreRequestView,
    saved: Option<SavedEnvelopeView>,
    loaded: RuntimeVmView,
) -> bool {
    &&& request.valid_for(initialized)
    &&& saved == Some(saved_envelope(request.snapshot))
    &&& restore_relation(request, loaded)
}

pub open spec fn quiesce_success(
    stopped_vm: RuntimeVmView,
    response: SnapshotSaveResponseView,
) -> bool {
    capture_relation(stopped_vm, response.snapshot, response.boundary)
}

pub proof fn capture_then_restore_composes(
    source: RuntimeVmView,
    response: SnapshotSaveResponseView,
    environment: RestoreEnvironment,
    restored: RuntimeVmView,
)
    requires
        quiesce_success(source, response),
        restore_relation(
            RestoreRequestView {
                snapshot: response.snapshot,
                environment,
            },
            restored,
        ),
    ensures
        restored.logical == allowed_restore_transform(response.snapshot, environment),
        !restored.guest_can_execute,
        !restored.externally_ready,
{
}

}
