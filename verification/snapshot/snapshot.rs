// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

use vstd::prelude::*;

verus! {

pub mod snapshot_spec {
    use super::*;

    pub struct MachineView {
        pub architecture: nat,
        pub profile: nat,
        pub format_version: nat,
    }

    pub struct VpView {
        pub stable_id: nat,
        pub architectural_state: Seq<u8>,
        pub optional_fields: Map<nat, Seq<u8>>,
    }

    pub struct ComponentView {
        pub stable_id: Seq<u8>,
        pub payload: Option<Seq<u8>>,
        pub deferred_private_state: Option<Seq<u8>>,
    }

    pub struct ClockView {
        pub vm_time_100ns: nat,
        pub tsc: nat,
        pub apic_ticks_remaining: nat,
        pub rtc_unix_ms: int,
        pub backend_clock_ns: nat,
    }

    pub struct LogicalVmState {
        pub machine: MachineView,
        pub ram: Map<nat, u8>,
        pub vps: Seq<VpView>,
        pub components: Seq<ComponentView>,
        pub clocks: ClockView,
        pub generation_id: Seq<u8>,
        pub entropy: Seq<u8>,
    }

    pub struct SnapshotArtifactsView {
        pub machine: MachineView,
        pub memory: Map<nat, u8>,
        pub vps: Seq<VpView>,
        pub components: Seq<ComponentView>,
        pub inventory: Seq<Seq<u8>>,
        pub clocks_at_cut: ClockView,
        pub generation_id: Seq<u8>,
        pub entropy: Seq<u8>,
        pub memory_generation: nat,
        pub capture_wall_ns: nat,
        pub tsc_frequency_hz: nat,
        pub apic_frequency_hz: nat,
    }

    pub struct RuntimeVmView {
        pub machine: MachineView,
        pub memory: Map<nat, u8>,
        pub vps: Seq<VpView>,
        pub components: Seq<ComponentView>,
        pub clocks: ClockView,
        pub generation_id: Seq<u8>,
        pub entropy: Seq<u8>,
        pub guest_can_execute: bool,
        pub externally_ready: bool,
    }

    pub struct CaptureBoundary {
        pub state_units_stopped: bool,
        pub vps_stopped: bool,
        pub host_input_gated: bool,
        pub retained_mutations_excluded: bool,
        pub registration_stable: bool,
        pub clock_cut_aligned: bool,
        pub ram_matches_cut: bool,
        pub generation_committed: bool,
    }

    pub struct RestorePolicy {
        pub replace_generation_id: bool,
        pub replace_entropy: bool,
    }

    pub struct RestoreEnvironment {
        pub destination_machine: MachineView,
        pub destination_backend: nat,
        pub backend_pair_supported: bool,
        pub supported_optional_vp_fields: Set<nat>,
        pub downtime_ns: nat,
        pub tsc_frequency_hz: nat,
        pub apic_frequency_hz: nat,
        pub destination_generation_id: Seq<u8>,
        pub destination_entropy: Seq<u8>,
        pub policy: RestorePolicy,
    }

    pub struct SaveFailureAftermath {
        pub snapshot_published: bool,
        pub source_may_require_termination: bool,
    }

    pub struct RestoreFailureAftermath {
        pub guest_can_execute: bool,
        pub externally_ready: bool,
        pub resources_may_be_partially_mutated: bool,
    }

    pub open spec fn inventory_matches(
        components: Seq<ComponentView>,
        inventory: Seq<Seq<u8>>,
    ) -> bool {
        components.len() == inventory.len()
            && forall|i: int| 0 <= i < components.len()
                ==> components[i].stable_id == inventory[i]
    }

    pub open spec fn snapshot_view(snapshot: SnapshotArtifactsView) -> LogicalVmState {
        LogicalVmState {
            machine: snapshot.machine,
            ram: snapshot.memory,
            vps: snapshot.vps,
            components: snapshot.components,
            clocks: snapshot.clocks_at_cut,
            generation_id: snapshot.generation_id,
            entropy: snapshot.entropy,
        }
    }

    pub open spec fn project_vm(vm: RuntimeVmView) -> LogicalVmState {
        LogicalVmState {
            machine: vm.machine,
            ram: vm.memory,
            vps: vm.vps,
            components: vm.components,
            clocks: vm.clocks,
            generation_id: vm.generation_id,
            entropy: vm.entropy,
        }
    }

    pub open spec fn capture_boundary_established(boundary: CaptureBoundary) -> bool {
        boundary.state_units_stopped
            && boundary.vps_stopped
            && boundary.host_input_gated
            && boundary.retained_mutations_excluded
            && boundary.registration_stable
            && boundary.clock_cut_aligned
            && boundary.ram_matches_cut
            && boundary.generation_committed
    }

    pub open spec fn capture_relation(
        source_at_cut: RuntimeVmView,
        snapshot: SnapshotArtifactsView,
        boundary: CaptureBoundary,
    ) -> bool {
        !source_at_cut.guest_can_execute
            && capture_boundary_established(boundary)
            && inventory_matches(snapshot.components, snapshot.inventory)
            && project_vm(source_at_cut) == snapshot_view(snapshot)
    }

    pub open spec fn vp_fields_supported(
        vps: Seq<VpView>,
        supported_fields: Set<nat>,
    ) -> bool {
        forall|i: int, field: nat|
            0 <= i < vps.len() && vps[i].optional_fields.dom().contains(field)
                ==> supported_fields.contains(field)
    }

    pub open spec fn compatible(
        snapshot: SnapshotArtifactsView,
        environment: RestoreEnvironment,
    ) -> bool {
        snapshot.machine == environment.destination_machine
            && environment.backend_pair_supported
            && snapshot.tsc_frequency_hz == environment.tsc_frequency_hz
            && snapshot.apic_frequency_hz == environment.apic_frequency_hz
            && inventory_matches(snapshot.components, snapshot.inventory)
            && vp_fields_supported(snapshot.vps, environment.supported_optional_vp_fields)
    }

    pub open spec fn elapsed_tsc(environment: RestoreEnvironment) -> nat {
        environment.downtime_ns * environment.tsc_frequency_hz / 1_000_000_000
    }

    pub open spec fn elapsed_apic_ticks(environment: RestoreEnvironment) -> nat {
        environment.downtime_ns * environment.apic_frequency_hz / 1_000_000_000
    }

    pub open spec fn adjusted_clocks(
        clocks: ClockView,
        environment: RestoreEnvironment,
    ) -> ClockView {
        let apic_elapsed = elapsed_apic_ticks(environment);
        ClockView {
            vm_time_100ns: clocks.vm_time_100ns + environment.downtime_ns / 100,
            tsc: clocks.tsc + elapsed_tsc(environment),
            apic_ticks_remaining: if clocks.apic_ticks_remaining > apic_elapsed {
                (clocks.apic_ticks_remaining - apic_elapsed) as nat
            } else {
                0
            },
            rtc_unix_ms: clocks.rtc_unix_ms + (environment.downtime_ns / 1_000_000) as int,
            backend_clock_ns: clocks.backend_clock_ns + environment.downtime_ns,
        }
    }

    pub open spec fn allowed_restore_transform(
        state: LogicalVmState,
        environment: RestoreEnvironment,
    ) -> LogicalVmState {
        LogicalVmState {
            machine: state.machine,
            ram: state.ram,
            vps: state.vps,
            components: state.components,
            clocks: adjusted_clocks(state.clocks, environment),
            generation_id: if environment.policy.replace_generation_id {
                environment.destination_generation_id
            } else {
                state.generation_id
            },
            entropy: if environment.policy.replace_entropy {
                environment.destination_entropy
            } else {
                state.entropy
            },
        }
    }

    pub open spec fn restore_relation(
        snapshot: SnapshotArtifactsView,
        returned_vm: RuntimeVmView,
        environment: RestoreEnvironment,
    ) -> bool {
        compatible(snapshot, environment)
            && !returned_vm.guest_can_execute
            && !returned_vm.externally_ready
            && project_vm(returned_vm)
                == allowed_restore_transform(snapshot_view(snapshot), environment)
    }

    pub open spec fn round_trip_relation(
        source_at_cut: RuntimeVmView,
        snapshot: SnapshotArtifactsView,
        returned_vm: RuntimeVmView,
        capture_boundary: CaptureBoundary,
        environment: RestoreEnvironment,
    ) -> bool {
        capture_relation(source_at_cut, snapshot, capture_boundary)
            && restore_relation(snapshot, returned_vm, environment)
            && project_vm(returned_vm)
                == allowed_restore_transform(project_vm(source_at_cut), environment)
    }

    pub open spec fn save_failure_contract(aftermath: SaveFailureAftermath) -> bool {
        !aftermath.snapshot_published
    }

    pub open spec fn restore_failure_contract(aftermath: RestoreFailureAftermath) -> bool {
        !aftermath.guest_can_execute && !aftermath.externally_ready
    }

    pub proof fn capture_and_restore_compose(
        source_at_cut: RuntimeVmView,
        snapshot: SnapshotArtifactsView,
        returned_vm: RuntimeVmView,
        capture_boundary: CaptureBoundary,
        environment: RestoreEnvironment,
    )
        requires
            capture_relation(source_at_cut, snapshot, capture_boundary),
            restore_relation(snapshot, returned_vm, environment),
        ensures
            round_trip_relation(
                source_at_cut,
                snapshot,
                returned_vm,
                capture_boundary,
                environment,
            ),
    {
    }

    pub proof fn zero_downtime_identity_transform(
        state: LogicalVmState,
        environment: RestoreEnvironment,
    )
        requires
            environment.downtime_ns == 0,
            !environment.policy.replace_generation_id,
            !environment.policy.replace_entropy,
        ensures
            allowed_restore_transform(state, environment) == state,
    {
    }
}

}
