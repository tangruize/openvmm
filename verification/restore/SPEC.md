# Snapshot Restore TOP-Level Specification

## Target boundary

The focused verification TOP is `LoadedVm::restore_snapshot_state` in `openvmm/openvmm_core/src/worker/dispatch.rs`.

Its pre-state is a destination `LoadedVm` after `InitializedVm::load` has completed platform/device construction, installed the prepared memory and external resources, and instantiated the selected VPs. Its successful post-state is the point after state-unit restore, permitted time adjustment, backend-clock adjustment, and acquisition of the restore stop guard, but before `LoadedVm::resume` can permit guest execution.

`InitializedVm::load` retains its conditional snapshot wrapper contract. That contract states the externally visible composition from the initialized VM and restore request to the returned `LoadedVm`, while delegating the extracted restore transition to the helper specification. It is not the focused verification target.

`prepare_snapshot_restore_for_config`, snapshot decoding, destination construction proofs, caller failure publication, deferred-state activation, and readiness publication are outside the focused TOP. They require separate contracts for an end-to-end proof, but this task does not add their proofs.

## Logical state boundary

The human-owned open model separates runtime state from snapshot state at every level where they differ:

- `VmStateView`: the full logical runtime state represented at this boundary, including RAM, guest-visible state, destination configuration/resources, live components, and `HostOperationalStateView`.
- `ComponentStateView`: the full runtime state of one live component. Most components mix serialized and non-serialized fields, so each component View splits them into `snapshot: ComponentSnapshotStateView` (exactly what the component's saved-state blob carries and its restore installs), `config: ComponentConfigView` (destination-constructed configuration such as bindings, sizes, and wiring), and `host: ComponentHostStateView` (host-bound runtime objects such as tasks, timers, queue workers, and handles).
- `ComponentSnapshotStateView`: serialized component state only. Deferred (pending) components hold only this form before activation.
- `VmSnapshotView`: the VM state carried by decoded `SavedState`, and equally the snapshot-owned part of a live VM: partition state, stable-identity VP state, component inventory, active and pending component snapshot state, and virtual time. It does not contain RAM (carried by the separately prepared memory file), external resources, destination compatibility, VP capacity, or snapshot-generation identity. One type serves both roles, so the saved state and the live snapshot state are directly comparable.
- `RestoreRequestView`: the saved `VmSnapshotView`, selected VP count, and the optional downtime policy.
- `LoadedVmView`: the concrete helper pre/post-state together with active VP count and lifecycle phase.
- `ExternalResourcesView`: bindings from a destination attachment point (`ResourceSlotId`) to the logical identity of the host backing object bound there (`BackingResourceId`). Handle or descriptor values are not identities.

`VmStateView::snapshot()` does not select fields of the VM-level View. It composes the partition, VP, inventory, pending, and virtual-time snapshot state with each active component's own `ComponentStateView::snapshot` part (`ComponentStates::snapshot()`). The split between snapshot and non-snapshot fields is therefore owned by each component View, and the future component refinements must place every serialized field in `snapshot` and every other field in `config` or `host`.

Restore overlays saved state only on snapshot state: `ComponentSnapshotStates::overlay` operates on `ComponentSnapshotStateView`, never on full component runtime state. Component `config` must be preserved. Component `host` state may be rebuilt by restore and is not constrained by this TOP; host-side activity may likewise change `host_operational_state` while restore runs.

The vocabulary is expressed as methods on the Views (`VirtualTimeView::after_downtime`, `VpStates::restore_selected`/`after_downtime`, `ComponentStates::snapshot`/`configs`, `ComponentSnapshotStates::overlay`/`after_downtime`, `VmStateView::snapshot`/`has_stable_vp_identities`/`preserves_destination_of`/`restores_to`, `VmSnapshotView::restore_from`/`after_downtime`, `RestoreRequestView::is_compatible_with`/`valid_for_*`, and the `LoadedVmView`/`InitializedVmView` success adapters) rather than free functions.

## Snapshot data interpretation

The generic serialized state is:

```text
SavedState {
    units: Vec<SavedStateUnit>,
    inventory: Vec<String>,
}

SavedStateUnit {
    name: String,
    state: SavedStateBlob,
}
```

`inventory` is the complete ordered state-unit identity set, including stateless units. `units` contains only units with mutable serialized state. Each `name` selects one registered component, and its opaque blob is interpreted by that component's concrete saved-state schema.

The TOP currently uses `decoded_restore_request_view` as an explicit proof-debt bridge from `SavedState` and `restore_time` to the logical request. Replacing that bridge with component Views is proof work and is not part of this top-level-spec task. Protobuf codec correctness may eventually be a narrow trusted boundary, but repository-owned name-to-component interpretation must not be hidden by the final proof.

## Preconditions

`RestoreRequestView::valid_for_loaded_vm(old(self)@)` requires:

- the helper pre-state is `PreparingRestore`;
- the active VP count does not exceed destination VP capacity;
- the request's selected VP count equals the active VP count already instantiated in the helper pre-state;
- the destination VP map has stable identities for the full destination capacity (`VmStateView::has_stable_vp_identities`);
- saved VP identities are a subset of destination VP identities;
- the complete saved component inventory equals the destination component inventory;
- active and pending saved-state domains are subsets of that inventory and of the corresponding destination component domains;
- saved virtual time has not already incorporated restore downtime.

Preparation establishes additional end-to-end facts before this TOP: exact snapshot memory generation, manifest/state association, machine compatibility, approved external resources, decoded state provenance, and selected-VP construction. Those obligations are documented in `PREPARATION.md` and are not asserted by `SavedState` alone.

## Successful postcondition

On `Ok(())`, `old(self)@.snapshot_restore_success(request, final(self)@)` requires:

- the final snapshot state equals `initial.state.snapshot().restore_from(request, initial.active_vp_count)`;
- saved partition state is restored;
- VP state is restored by stable VP identity for selected VPs present in the saved state, while other destination VP state remains initial/default (`VpStates::restore_selected`);
- complete component inventory is preserved and saved active/pending component snapshot state overlays the initial/default component snapshot state (`ComponentSnapshotStates::overlay`);
- without a downtime adjustment, virtual time is the saved time with zero elapsed downtime; with one, the whole restored snapshot state is compensated by `VmSnapshotView::after_downtime` (see below);
- prepared RAM, destination compatibility, VP capacity, external resources, and every active component's configuration are preserved (`VmStateView::preserves_destination_of`);
- the active VP count is preserved and the final lifecycle phase is `PreExecutionRestored`.

### Downtime compensation

When the request carries a downtime adjustment, the restored VM must appear to have kept running for that downtime, so restore is not "install the saved state" alone. `VmSnapshotView::after_downtime` applies the compensation after the overlay: virtual time advances by the truncated 100ns downtime (`VirtualTimeView::after_downtime`, concrete), and partition, every VP, and every live component snapshot state advance their own clocks (`PartitionStateView`, `VpStateView`, and `ComponentSnapshotStateView::after_downtime`). Component inventory is unchanged, and deferred (pending) component state is held rather than live, so it is not advanced by this TOP.

The three leaf operations are temporary `uninterp` proof debt declared in `worker/dispatch.proof.rs` with `TODO(uninterp)` notes and recorded in `UNINTERP.json`; they are not part of the sanctioned TCB. The open spec fixes only their role: each leaf advances by the same downtime and nothing else changes. What "advance" means for a particular leaf (TSC and APIC timer for a VP, the backend snapshot clock for the partition, an RTC for a component, identity for clock-free components) is supplied when that leaf's View is refined. Frequencies and other mechanism details stay out of the TOP.

The postcondition deliberately does not constrain component `host` state or `host_operational_state`. It also does not claim guest readiness, deferred-state activation, caller publication, or rollback after failure.

The `Err` arm has no state rollback guarantee. Failure non-publication and no-resume properties belong to separate caller contracts.

`VmStateView::restores_to` contains the single definition of the successful restore result: snapshot equality, preserved destination state, selected active VP count, and `PreExecutionRestored` lifecycle phase. `LoadedVmView::snapshot_restore_success` adapts the helper's `LoadedVm` pre-state to it, while `InitializedVmView::snapshot_load_success` adapts the wrapper's `InitializedVm` pre-state. Neither adapter duplicates the restore semantics.

The retained `InitializedVm::load` contract uses `valid_for_initialized_vm` and `snapshot_load_success`. It keeps the original conditional behavior for `saved_state.is_some()`, including boot-online and selected-VP bounds. On current main, `load` instantiates every destination VP, so the wrapper selects the full destination VP capacity.

## Open and closed layers

The human-owned open layer in `worker/dispatch.spec.rs` defines the reviewable state vocabulary, runtime/snapshot split, validity relation, and success property.

The closed layer in `worker/dispatch.proof.rs` currently provides only representation bridges needed to attach the open contract to production types, plus the three uninterpreted leaf downtime operations. These are recorded in `UNINTERP.json` as proof debt. No proof, `assume`, `admit`, copied restore implementation, or predicate that directly asserts the final theorem is added by this task.

`#[verus_spec]` contracts are attached to both the retained `InitializedVm::load` wrapper and `LoadedVm::restore_snapshot_state`. `verification/tools/verify.sh` selects the extracted helper as the focused TOP. The `#[verus_verify]` marker remains disabled because proving either body is outside the requested scope.
