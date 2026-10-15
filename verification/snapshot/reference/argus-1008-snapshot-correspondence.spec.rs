// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

// Open model for the complete snapshot projection and its lifecycle relations.

verus! {

pub mod snapshot_correspondence_spec {
    use super::*;

    pub struct ComponentEnvelopeView {
        pub units: Seq<state_unit::SavedStateUnit>,
        pub inventory: Seq<String>,
    }

    pub type TimeView = snapshot_clock_spec::ClockView;

    pub struct StoppedVmView {
        pub component_ledger: Seq<state_unit::SavedStateUnit>,
        pub inventory: Seq<String>,
        pub partition: partition_component_spec::PartitionView,
        pub memory: Seq<u8>,
        pub time: TimeView,
        pub execution_blocked: bool,
    }

    pub struct CompleteSnapshotView {
        pub components: ComponentEnvelopeView,
        pub partition_component_index: nat,
        pub partition_component_blob: Seq<u8>,
        pub partition_payload: partition_component_spec::PartitionPayload,
        pub memory_backing: Seq<u8>,
        pub memory_generation: nat,
        pub captured_time: TimeView,
        pub vmtime_payload: snapshot_clock_spec::VmtimePayloadView,
        pub capture_wall_ns: nat,
        pub tsc_frequency_hz: nat,
        pub apic_frequency_hz: nat,
        pub cpu_contract: Seq<u8>,
    }

    pub struct DestinationView {
        pub inventory: Seq<String>,
        pub memory_size: nat,
        pub tsc_frequency_hz: nat,
        pub apic_frequency_hz: nat,
        pub cpu_contract: Seq<u8>,
        pub partition_vp_count: nat,
    }

    pub struct CaptureDomain {
        pub linux_host: bool,
        pub x86_64_guest: bool,
        pub supported_microvm_profile: bool,
        pub shared_file_backed_memory: bool,
        pub snapshot_request_authorized: bool,
        pub vm_running_before_quiesce: bool,
        pub quiesce_deadline_policy_valid: bool,
        pub full_partition_vp_population: bool,
        pub typed_partition_state_supported: bool,
    }

    pub struct CaptureBoundary {
        pub stable_state_units: bool,
        pub host_input_gated: bool,
        pub vps_stopped_at_io_boundary: bool,
        pub registration_stable: bool,
        pub shared_memory_exact_and_flushable: bool,
        pub time_cut_aligned: bool,
    }

    pub struct RestoreDomain {
        pub saved_state_present: bool,
        pub linux_host: bool,
        pub x86_64_guest: bool,
        pub supported_microvm_profile: bool,
        pub snapshot_artifacts_available_and_valid: bool,
        pub destination_reconstruction_supported: bool,
        pub full_partition_vp_population: bool,
        pub typed_partition_state_supported: bool,
        pub pending_exception_payload_supported: bool,
    }

    pub struct RestoreBoundary {
        pub artifacts_pinned: bool,
        pub component_restore_succeeded: bool,
        pub time_adjustments_succeeded: bool,
        pub vp_stop_guard_held: bool,
    }

    pub struct RestoreBodyWitness {
        pub selected_envelope: ComponentEnvelopeView,
        pub destination_inventory: Seq<String>,
        pub supplied_payloads: Seq<state_unit::SavedStateUnit>,
        pub consumed_payloads: Seq<state_unit::SavedStateUnit>,
        pub declared_downtime: nat,
        pub state_unit_downtime: nat,
        pub vp_downtime: nat,
        pub backend_downtime: nat,
        pub return_guard_installed: bool,
    }

    pub struct RestoreValueLedgerWitness {
        pub destination_inventory: Seq<String>,
        pub supplied_payloads: Seq<state_unit::SavedStateUnit>,
        pub consumed_payloads: Seq<state_unit::SavedStateUnit>,
    }

    pub struct RegisteredStateUnit {
        pub id: nat,
        pub inventory_name: String,
        pub payload_name: Seq<u8>,
    }

    pub struct ResolvedRestoreEntry {
        pub id: nat,
        pub supplied_index: nat,
        pub unit: state_unit::SavedStateUnit,
    }

    pub struct RestoreCallbackInput {
        pub id: nat,
        pub input: Option<state_unit::SavedStateUnit>,
    }

    pub struct StateUnitsRestoreBoundaryWitness {
        pub registrations: Seq<RegisteredStateUnit>,
        pub resolved_entries: Seq<ResolvedRestoreEntry>,
        pub callback_inputs: Seq<RestoreCallbackInput>,
        pub leftovers: Seq<ResolvedRestoreEntry>,
    }

    pub struct SaveCallbackResult {
        pub id: nat,
        pub name: Seq<u8>,
        pub state: Option<state_unit::SavedStateUnit>,
    }

    pub struct StateUnitsSaveBoundaryWitness {
        pub registrations: Seq<RegisteredStateUnit>,
        pub callback_results: Seq<SaveCallbackResult>,
        pub saved_to_callback: Seq<nat>,
        pub saved_units: Seq<state_unit::SavedStateUnit>,
    }

    pub struct PartitionPathWitness {
        pub component_index: nat,
        pub saved_blob: Seq<u8>,
        pub supplied_blob: Seq<u8>,
        pub consumed_blob: Seq<u8>,
        pub payload: partition_component_spec::PartitionPayload,
        pub destination_full: bool,
        pub consumed_vp_indices: Seq<nat>,
    }

    pub struct PartitionSaveHandlerWitness {
        pub blob: Seq<u8>,
        pub payload: partition_component_spec::PartitionPayload,
    }

    pub struct PartitionSaveLedgerWitness {
        pub component_index: nat,
    }

    pub struct PartitionRestoreDispatchWitness {
        pub component_index: nat,
        pub delegate_input_blob: Seq<u8>,
    }

    pub struct PartitionRestoreHandlerWitness {
        pub decoded_payload: partition_component_spec::PartitionPayload,
        pub destination_full: bool,
        pub consumed_vp_indices: Seq<nat>,
    }

    pub struct RestoredMemoryMappingView {
        pub generation: nat,
        pub visible_memory: Seq<u8>,
    }

    pub enum SaveResultView {
        Success(CompleteSnapshotView),
        Failure,
    }

    pub enum RestoreResultView {
        Success(StoppedVmView),
        Failure,
    }

    pub open spec fn component_envelope_of(state: &SavedState) -> ComponentEnvelopeView {
        ComponentEnvelopeView {
            units: state.units@,
            inventory: state.inventory@,
        }
    }

    pub open spec fn partition_component_name() -> Seq<u8> {
        seq![112u8, 97u8, 114u8, 116u8, 105u8, 116u8, 105u8, 111u8, 110u8]
    }

    pub open spec fn partition_component_at(
        units: Seq<state_unit::SavedStateUnit>,
        index: nat,
        blob: Seq<u8>,
        payload: partition_component_spec::PartitionPayload,
    ) -> bool {
        index < units.len()
            && units[index as int].name == partition_component_name()
            && units[index as int].payload == blob
            && units[index as int].partition_payload == Some(payload)
            && forall|other: int| 0 <= other < units.len() && other != index
                ==> units[other].name != partition_component_name()
    }

    pub open spec fn capture_supported(domain: CaptureDomain) -> bool {
        domain.linux_host
            && domain.x86_64_guest
            && domain.supported_microvm_profile
            && domain.shared_file_backed_memory
            && domain.snapshot_request_authorized
            && domain.vm_running_before_quiesce
            && domain.quiesce_deadline_policy_valid
            && domain.full_partition_vp_population
            && domain.typed_partition_state_supported
    }

    pub open spec fn capture_boundary_established(boundary: CaptureBoundary) -> bool {
        boundary.stable_state_units
            && boundary.host_input_gated
            && boundary.vps_stopped_at_io_boundary
            && boundary.registration_stable
            && boundary.shared_memory_exact_and_flushable
            && boundary.time_cut_aligned
    }

    pub open spec fn saved_state_matches_snapshot_components(
        saved_state: &SavedState,
        assembled_state: &SavedState,
        snapshot: CompleteSnapshotView,
    ) -> bool {
        snapshot_save_bridge_spec::saved_state_assembled_from(
            saved_state,
            assembled_state,
        )
            && component_envelope_of(assembled_state) == snapshot.components
    }

    pub open spec fn restore_supported(domain: RestoreDomain) -> bool {
        domain.saved_state_present
            && domain.linux_host
            && domain.x86_64_guest
            && domain.supported_microvm_profile
            && domain.snapshot_artifacts_available_and_valid
            && domain.destination_reconstruction_supported
            && domain.full_partition_vp_population
            && domain.typed_partition_state_supported
            && domain.pending_exception_payload_supported
    }

    pub open spec fn restore_boundary_established(boundary: RestoreBoundary) -> bool {
        boundary.artifacts_pinned
            && boundary.component_restore_succeeded
            && boundary.time_adjustments_succeeded
            && boundary.vp_stop_guard_held
    }

    pub open spec fn compatible(
        snapshot: CompleteSnapshotView,
        destination: DestinationView,
    ) -> bool {
        snapshot.components.inventory == destination.inventory
            && snapshot.memory_backing.len() == destination.memory_size
            && snapshot.tsc_frequency_hz == destination.tsc_frequency_hz
            && snapshot.apic_frequency_hz == destination.apic_frequency_hz
            && snapshot.cpu_contract == destination.cpu_contract
            && snapshot.partition_payload.vps.len()
                == destination.partition_vp_count
    }

    pub open spec fn capture_relation(
        vm: StoppedVmView,
        snapshot: CompleteSnapshotView,
    ) -> bool {
        capture_context_relation(vm, snapshot)
            && partition_component_at(
                snapshot.components.units,
                snapshot.partition_component_index,
                snapshot.partition_component_blob,
                snapshot.partition_payload,
            )
            && partition_component_spec::save_relation(
                vm.partition,
                snapshot.partition_payload,
            )
    }

    pub open spec fn capture_context_relation(
        vm: StoppedVmView,
        snapshot: CompleteSnapshotView,
    ) -> bool {
        vm.execution_blocked
            && vm.component_ledger == snapshot.components.units
            && vm.inventory == snapshot.components.inventory
            && vm.memory == snapshot.memory_backing
            && vm.time == snapshot.captured_time
            && snapshot_clock_spec::vmtime_payload_bound(
                snapshot.vmtime_payload,
                snapshot.captured_time,
                snapshot.capture_wall_ns,
            )
            && snapshot.captured_time.tsc_frequency_hz
                == snapshot.tsc_frequency_hz
            && snapshot.captured_time.apic_frequency_hz
                == snapshot.apic_frequency_hz
    }

    pub open spec fn advance_time(time: TimeView, downtime: nat) -> TimeView {
        snapshot_clock_spec::advance_clocks(time, downtime)
    }

    pub open spec fn restore_relation(
        snapshot: CompleteSnapshotView,
        restored: StoppedVmView,
        downtime: nat,
        consumed_vp_indices: Seq<nat>,
    ) -> bool {
        restored.execution_blocked
            && restored.component_ledger == snapshot.components.units
            && restored.inventory == snapshot.components.inventory
            && partition_component_spec::restore_relation(
                snapshot.partition_payload,
                restored.partition,
                consumed_vp_indices,
            )
            && restored.memory == snapshot.memory_backing
            && snapshot_clock_spec::clock_adjustment_supported(
                snapshot.captured_time,
                downtime,
            )
            && restored.time == advance_time(snapshot.captured_time, downtime)
    }

    pub open spec fn actual_restore_body_witness(
        restore_input: &SavedState,
        selected_state: &SavedState,
        witness: RestoreBodyWitness,
        downtime: nat,
    ) -> bool {
        snapshot_restore_bridge_spec::restore_input_is(
            restore_input,
            selected_state,
        )
            && witness.selected_envelope == component_envelope_of(selected_state)
            && witness.selected_envelope.inventory.len() > 0
            && snapshot_restore_bridge_spec::inventory_validation_path(
                witness.selected_envelope.inventory.len() > 0,
                witness.selected_envelope.inventory == witness.destination_inventory,
            )
            && witness.supplied_payloads == witness.selected_envelope.units
            && witness.consumed_payloads == witness.supplied_payloads
            && witness.declared_downtime == downtime
            && snapshot_restore_bridge_spec::common_duration_applied(
                witness.state_unit_downtime == witness.declared_downtime,
                witness.vp_downtime == witness.declared_downtime,
                witness.backend_downtime == witness.declared_downtime,
            )
            && snapshot_restore_bridge_spec::stopped_return(
                witness.return_guard_installed,
            )
    }

    pub open spec fn duration_value(duration: Duration) -> nat {
        duration.nanos
    }

    pub open spec fn source_restore_value_ledger(
        restore_input: &SavedState,
        ledger: RestoreValueLedgerWitness,
    ) -> bool {
        ledger.destination_inventory == component_envelope_of(restore_input).inventory
            && ledger.supplied_payloads == component_envelope_of(restore_input).units
            && ledger.consumed_payloads == ledger.supplied_payloads
    }

    pub open spec fn unique_registrations(
        registrations: Seq<RegisteredStateUnit>,
    ) -> bool {
        forall|left: int, right: int|
            0 <= left < registrations.len()
                && 0 <= right < registrations.len()
                && left != right
                ==> registrations[left].id != registrations[right].id
                && registrations[left].inventory_name
                    != registrations[right].inventory_name
                && registrations[left].payload_name
                    != registrations[right].payload_name
    }

    pub open spec fn inventory_matches_registrations(
        inventory: Seq<String>,
        registrations: Seq<RegisteredStateUnit>,
    ) -> bool {
        inventory.len() == registrations.len()
            && forall|i: int| 0 <= i < registrations.len()
                ==> inventory[i] == registrations[i].inventory_name
    }

    pub open spec fn successful_name_resolution(
        supplied: Seq<state_unit::SavedStateUnit>,
        registrations: Seq<RegisteredStateUnit>,
        resolved: Seq<ResolvedRestoreEntry>,
    ) -> bool {
        resolved.len() == supplied.len()
            && forall|i: int| 0 <= i < supplied.len()
                ==> resolved[i].supplied_index == i
                    && resolved[i].unit == supplied[i]
                    && exists|registration_index: int|
                        0 <= registration_index < registrations.len()
                            && resolved[i].id == registrations[registration_index].id
                            && supplied[i].name
                                == registrations[registration_index].payload_name
            && forall|left: int, right: int|
                0 <= left < resolved.len()
                    && 0 <= right < resolved.len()
                    && left != right
                    ==> resolved[left].id != resolved[right].id
    }

    pub open spec fn callback_visits_registrations_once(
        registrations: Seq<RegisteredStateUnit>,
        callbacks: Seq<RestoreCallbackInput>,
    ) -> bool {
        callbacks.len() == registrations.len()
            && forall|i: int| 0 <= i < registrations.len()
                ==> callbacks[i].id == registrations[i].id
    }

    pub open spec fn callback_inputs_are_exact_removals(
        resolved: Seq<ResolvedRestoreEntry>,
        callbacks: Seq<RestoreCallbackInput>,
    ) -> bool {
        forall|callback_index: int| 0 <= callback_index < callbacks.len()
            ==> match #[trigger] callbacks[callback_index].input {
                Some(unit) => exists|resolved_index: int|
                    0 <= resolved_index < resolved.len()
                        && resolved[resolved_index].id == callbacks[callback_index].id
                        && resolved[resolved_index].unit == unit,
                None => forall|resolved_index: int| 0 <= resolved_index < resolved.len()
                    ==> resolved[resolved_index].id != callbacks[callback_index].id,
            }
    }

    pub open spec fn every_resolved_entry_consumed_once(
        resolved: Seq<ResolvedRestoreEntry>,
        callbacks: Seq<RestoreCallbackInput>,
    ) -> bool {
        forall|resolved_index: int| 0 <= resolved_index < resolved.len()
            ==> exists|callback_index: int|
                0 <= callback_index < callbacks.len()
                    && callbacks[callback_index].id
                        == #[trigger] resolved[resolved_index].id
                    && callbacks[callback_index].input
                        == Some(resolved[resolved_index].unit)
                    && forall|other_callback: int|
                        0 <= other_callback < callbacks.len()
                            && other_callback != callback_index
                            ==> callbacks[other_callback].id
                                != resolved[resolved_index].id
    }

    pub open spec fn successful_state_units_restore_boundary(
        restore_input: &SavedState,
        witness: StateUnitsRestoreBoundaryWitness,
    ) -> bool {
        unique_registrations(witness.registrations)
            && inventory_matches_registrations(
                component_envelope_of(restore_input).inventory,
                witness.registrations,
            )
            && successful_name_resolution(
                component_envelope_of(restore_input).units,
                witness.registrations,
                witness.resolved_entries,
            )
            && callback_visits_registrations_once(
                witness.registrations,
                witness.callback_inputs,
            )
            && callback_inputs_are_exact_removals(
                witness.resolved_entries,
                witness.callback_inputs,
            )
            && every_resolved_entry_consumed_once(
                witness.resolved_entries,
                witness.callback_inputs,
            )
            && witness.leftovers.len() == 0
    }

    pub open spec fn restore_value_ledger_from_boundary(
        restore_input: &SavedState,
    ) -> RestoreValueLedgerWitness {
        RestoreValueLedgerWitness {
            destination_inventory: component_envelope_of(restore_input).inventory,
            supplied_payloads: component_envelope_of(restore_input).units,
            consumed_payloads: component_envelope_of(restore_input).units,
        }
    }

    pub open spec fn successful_save_callback_collection(
        witness: StateUnitsSaveBoundaryWitness,
    ) -> bool {
        unique_registrations(witness.registrations)
            && witness.callback_results.len() == witness.registrations.len()
            && forall|i: int| 0 <= i < witness.registrations.len()
                ==> witness.callback_results[i].id == witness.registrations[i].id
                    && witness.callback_results[i].name
                        == witness.registrations[i].payload_name
            && witness.saved_units.len() == witness.saved_to_callback.len()
            && forall|saved_index: int| 0 <= saved_index < witness.saved_units.len()
                ==> witness.saved_to_callback[saved_index] < witness.callback_results.len()
                    && witness.callback_results[
                        witness.saved_to_callback[saved_index] as int
                    ].state == Some(witness.saved_units[saved_index])
                    && (saved_index == 0
                        || witness.saved_to_callback[saved_index - 1]
                            < witness.saved_to_callback[saved_index])
            && forall|callback_index: int| 0 <= callback_index
                < witness.callback_results.len()
                && witness.callback_results[callback_index].state.is_some()
                ==> exists|saved_index: int| 0 <= saved_index < witness.saved_units.len()
                    && witness.saved_to_callback[saved_index] == callback_index
    }

    pub open spec fn partition_save_callback_collected(
        witness: StateUnitsSaveBoundaryWitness,
        handler: PartitionSaveHandlerWitness,
        ledger: PartitionSaveLedgerWitness,
    ) -> bool {
        successful_save_callback_collection(witness)
            && ledger.component_index < witness.saved_units.len()
            && witness.saved_units[ledger.component_index as int].name
                == partition_component_name()
            && witness.saved_units[ledger.component_index as int].payload == handler.blob
            && witness.saved_units[ledger.component_index as int].partition_payload
                == Some(handler.payload)
            && forall|other: int| 0 <= other < witness.saved_units.len()
                && other != ledger.component_index
                ==> witness.saved_units[other].name != partition_component_name()
    }

    pub open spec fn partition_restore_callback_dispatched(
        restore_input: &SavedState,
        witness: StateUnitsRestoreBoundaryWitness,
        supplied_index: nat,
        dispatch: PartitionRestoreDispatchWitness,
    ) -> bool {
        successful_state_units_restore_boundary(restore_input, witness)
            && supplied_index < component_envelope_of(restore_input).units.len()
            && component_envelope_of(restore_input).units[supplied_index as int].name
                == partition_component_name()
            && dispatch.component_index == supplied_index
            && dispatch.delegate_input_blob
                == component_envelope_of(restore_input).units[supplied_index as int].payload
            && exists|resolved_index: int, callback_index: int|
                0 <= resolved_index < witness.resolved_entries.len()
                    && witness.resolved_entries[resolved_index].supplied_index
                        == supplied_index
                    && 0 <= callback_index < witness.callback_inputs.len()
                    && witness.callback_inputs[callback_index].id
                        == witness.resolved_entries[resolved_index].id
                    && witness.callback_inputs[callback_index].input
                        == Some(component_envelope_of(restore_input).units[
                            supplied_index as int
                        ])
    }

    pub open spec fn restore_body_witness_from_contract(
        restore_input: &SavedState,
        contract: snapshot_restore_bridge_spec::RestoreReturnContract,
        ledger: RestoreValueLedgerWitness,
    ) -> RestoreBodyWitness {
        RestoreBodyWitness {
            selected_envelope: component_envelope_of(restore_input),
            destination_inventory: ledger.destination_inventory,
            supplied_payloads: ledger.supplied_payloads,
            consumed_payloads: ledger.consumed_payloads,
            declared_downtime: duration_value(contract.duration.declared),
            state_unit_downtime: duration_value(contract.duration.state_unit),
            vp_downtime: duration_value(contract.duration.vp),
            backend_downtime: duration_value(contract.duration.backend),
            return_guard_installed: contract.return_guard_installed,
        }
    }

    pub open spec fn component_ledger_contract(
        witness: RestoreBodyWitness,
        restored: StoppedVmView,
    ) -> bool {
        restored.component_ledger == witness.consumed_payloads
            && restored.inventory == witness.destination_inventory
    }

    pub open spec fn checked_partition_save_handler(
        captured: partition_component_spec::PartitionView,
        handler: PartitionSaveHandlerWitness,
    ) -> bool {
        partition_component_spec::save_relation(captured, handler.payload)
    }

    pub open spec fn state_units_partition_save_entry(
        units: Seq<state_unit::SavedStateUnit>,
        handler: PartitionSaveHandlerWitness,
        ledger: PartitionSaveLedgerWitness,
    ) -> bool {
        partition_component_at(
            units,
            ledger.component_index,
            handler.blob,
            handler.payload,
        )
    }

    pub open spec fn snapshot_partition_projection_from_saved_state(
        snapshot: CompleteSnapshotView,
        handler: PartitionSaveHandlerWitness,
        ledger: PartitionSaveLedgerWitness,
    ) -> bool {
        snapshot.partition_component_index == ledger.component_index
            && snapshot.partition_component_blob == handler.blob
            && snapshot.partition_payload == handler.payload
    }

    pub open spec fn state_units_partition_restore_dispatch(
        witness: RestoreBodyWitness,
        component_index: nat,
        dispatch: PartitionRestoreDispatchWitness,
    ) -> bool {
        dispatch.component_index == component_index
            && component_index < witness.consumed_payloads.len()
            && witness.consumed_payloads[component_index as int].name
                == partition_component_name()
            && dispatch.delegate_input_blob
                == witness.consumed_payloads[component_index as int].payload
    }

    pub open spec fn checked_partition_restore_handler(
        unit: state_unit::SavedStateUnit,
        dispatch: PartitionRestoreDispatchWitness,
        handler: PartitionRestoreHandlerWitness,
        restored: partition_component_spec::PartitionView,
    ) -> bool {
        unit.payload == dispatch.delegate_input_blob
            && unit.partition_payload == Some(handler.decoded_payload)
            && handler.destination_full
            && partition_component_spec::compatible(
                handler.decoded_payload,
                restored.vps.len(),
            )
            && partition_component_spec::restore_relation(
                handler.decoded_payload,
                restored,
                handler.consumed_vp_indices,
            )
    }

    pub open spec fn partition_path_from_stages(
        ledger: PartitionSaveLedgerWitness,
        handler: PartitionSaveHandlerWitness,
        dispatch: PartitionRestoreDispatchWitness,
        restore_handler: PartitionRestoreHandlerWitness,
    ) -> PartitionPathWitness {
        PartitionPathWitness {
            component_index: ledger.component_index,
            saved_blob: handler.blob,
            supplied_blob: dispatch.delegate_input_blob,
            consumed_blob: dispatch.delegate_input_blob,
            payload: restore_handler.decoded_payload,
            destination_full: restore_handler.destination_full,
            consumed_vp_indices: restore_handler.consumed_vp_indices,
        }
    }

    pub open spec fn actual_partition_restore_path(
        snapshot: CompleteSnapshotView,
        witness: RestoreBodyWitness,
        partition: PartitionPathWitness,
        restored: StoppedVmView,
    ) -> bool {
        partition.component_index == snapshot.partition_component_index
            && partition.saved_blob == snapshot.partition_component_blob
            && partition.payload == snapshot.partition_payload
            && partition_component_at(
                snapshot.components.units,
                partition.component_index,
                partition.saved_blob,
                partition.payload,
            )
            && partition_component_at(
                witness.selected_envelope.units,
                partition.component_index,
                partition.saved_blob,
                partition.payload,
            )
            && partition_component_at(
                witness.supplied_payloads,
                partition.component_index,
                partition.supplied_blob,
                partition.payload,
            )
            && partition_component_at(
                witness.consumed_payloads,
                partition.component_index,
                partition.consumed_blob,
                partition.payload,
            )
            && partition.supplied_blob == partition.saved_blob
            && partition.consumed_blob == partition.supplied_blob
            && partition.destination_full
            && partition_component_spec::compatible(
                partition.payload,
                restored.partition.vps.len(),
            )
            && partition_component_spec::restore_relation(
                partition.payload,
                restored.partition,
                partition.consumed_vp_indices,
            )
    }

    pub open spec fn source_reviewed_exact_generation_path(
        snapshot: CompleteSnapshotView,
        opened_generation: nat,
        mapping: RestoredMemoryMappingView,
    ) -> bool {
        opened_generation == snapshot.memory_generation
            && mapping.generation == opened_generation
    }

    pub open spec fn memory_visibility_contract(
        snapshot: CompleteSnapshotView,
        opened_generation: nat,
        mapping: RestoredMemoryMappingView,
        restored: StoppedVmView,
    ) -> bool {
        opened_generation == snapshot.memory_generation
            && mapping.generation == opened_generation
            ==> mapping.visible_memory == snapshot.memory_backing
                && restored.memory == mapping.visible_memory
    }

    pub open spec fn time_handler_contract(
        saved_time: TimeView,
        witness: RestoreBodyWitness,
        restored: StoppedVmView,
    ) -> bool {
        witness.state_unit_downtime == witness.declared_downtime
            && witness.vp_downtime == witness.declared_downtime
            && witness.backend_downtime == witness.declared_downtime
            && snapshot_clock_spec::clock_adjustment_supported(
                saved_time,
                witness.declared_downtime,
            )
            && restored.time
                == advance_time(saved_time, witness.declared_downtime)
    }

    pub open spec fn stop_guard_contract(
        witness: RestoreBodyWitness,
        restored: StoppedVmView,
    ) -> bool {
        witness.return_guard_installed ==> restored.execution_blocked
    }

    pub open spec fn save_top(
        captured: StoppedVmView,
        domain: CaptureDomain,
        boundary: CaptureBoundary,
        saved_state: &SavedState,
        assembled_state: &SavedState,
        result: SaveResultView,
        external_impediment: bool,
    ) -> bool {
        match result {
            SaveResultView::Success(snapshot) => {
                capture_supported(domain)
                    && capture_boundary_established(boundary)
                    && saved_state_matches_snapshot_components(
                        saved_state,
                        assembled_state,
                        snapshot,
                    )
                    && capture_relation(captured, snapshot)
            }
            SaveResultView::Failure => !capture_supported(domain) || external_impediment,
        }
    }

    pub open spec fn restore_top(
        snapshot: CompleteSnapshotView,
        destination: DestinationView,
        domain: RestoreDomain,
        boundary: RestoreBoundary,
        downtime: nat,
        partition_consumed_vp_indices: Seq<nat>,
        result: RestoreResultView,
        external_impediment: bool,
    ) -> bool {
        match result {
            RestoreResultView::Success(restored) => {
                restore_supported(domain)
                    && restore_boundary_established(boundary)
                    && compatible(snapshot, destination)
                    && restore_relation(
                        snapshot,
                        restored,
                        downtime,
                        partition_consumed_vp_indices,
                    )
            }
            RestoreResultView::Failure => {
                !restore_supported(domain)
                    || !compatible(snapshot, destination)
                    || external_impediment
            }
        }
    }

    pub open spec fn same_logical_state_modulo_downtime(
        captured: StoppedVmView,
        restored: StoppedVmView,
        downtime: nat,
    ) -> bool {
        captured.component_ledger == restored.component_ledger
            && captured.inventory == restored.inventory
            && partition_component_spec::same_logical_view(
                captured.partition,
                restored.partition,
            )
            && captured.memory == restored.memory
            && snapshot_clock_spec::clock_adjustment_supported(
                captured.time,
                downtime,
            )
            && restored.time == advance_time(captured.time, downtime)
            && captured.execution_blocked
            && restored.execution_blocked
    }

    pub open spec fn implemented_restored_vm_time(
        saved_vm_time: int,
        boundary_to_stop: nat,
        post_stop_to_restore_sample: nat,
    ) -> int {
        saved_vm_time + boundary_to_stop + post_stop_to_restore_sample
    }

    pub open spec fn intended_restored_vm_time_at_sample(
        saved_vm_time: int,
        post_stop_to_restore_sample: nat,
    ) -> int {
        saved_vm_time + post_stop_to_restore_sample
    }

    pub open spec fn intended_restored_vm_time_at_return(
        saved_vm_time: int,
        post_stop_to_restore_sample: nat,
        restore_sample_to_return: nat,
    ) -> int {
        saved_vm_time + post_stop_to_restore_sample + restore_sample_to_return
    }
}

} // verus!
