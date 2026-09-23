# Snapshot Restore TOP-Level Specification

## Target boundary

The focused verification TOP is `LoadedVm::restore_snapshot_state` in `openvmm/openvmm_core/src/worker/dispatch.rs`.

Its pre-state is a destination `LoadedVm` after `InitializedVm::load` has completed platform/device construction, installed the prepared memory and external resources, and instantiated the VPs. Its successful post-state is the point after state-unit restore, permitted time adjustment, backend-clock adjustment, and acquisition of the restore stop guard, but before `LoadedVm::resume` can permit guest execution.

`InitializedVm::load` retains a conditional snapshot wrapper contract. `LoadedVm::save` carries the anchoring contract described below. Neither is the focused verification target.

`prepare_snapshot_restore_for_config`, snapshot decoding, destination construction, caller failure publication, deferred-state activation, and readiness publication are outside the focused TOP.

## Design principle: anchored snapshot state

Every View that the success property constrains has a fixed meaning taken from real data. Nothing in the snapshot model is an invented placeholder, so no later proof-side definition can make the property vacuous.

- **The snapshot View is the decoded `SavedState`.** `VmSnapshotView` holds the complete ordered state-unit `inventory` and a map from unit name to that unit's `SavedStateBlob`. `VmSnapshotView::of_saved_state` is an open, field-by-field decoding of a real `SavedState`. `SavedState` and `SavedStateUnit` are transparent to Verus; `SavedStateBlob` is an opaque protobuf payload compared only as a value (see the TCB section).
- **The restore input is decoded the same way.** `RestoreRequestView::decode` is open: it decodes the real `SavedState` and `restore_time` arguments (downtime through the sanctioned `elapsed_nanoseconds` Duration observation). There is no proof-side request bridge.
- **The live VM's View is anchored by `save`.** `LoadedVm@` is the snapshot-owned state of the VM: what `LoadedVm::save` would return. The `LoadedVm::save` contract states `of_saved_state(result) == old(self)@.snapshot` and that saving does not change it. Because `save` returns real data, a degenerate definition of `LoadedVm@` (for example a constant) cannot verify against a verified `save` body.

This mirrors an abstract data type whose View is pinned by operations that return concrete values: `save` plays the role of the observer, and `restore_snapshot_state` is specified in terms of the same View. Combined, the two contracts imply the concrete round trip: after a successful restore of `s`, the next `save` returns `s` overlaid on the unsaved units (and time-compensated when requested).

Which component fields are snapshot state is decided by what the component's own `save` serializes. Configuration and host-bound runtime objects that `save` does not serialize are not snapshot state. This is the per-component snapshot/non-snapshot split, taken from the code instead of being modeled by hand. It covers every kind of snapshot state uniformly: VP and partition state (saved by the partition state unit), virtual time (the `vmtime` unit), and deferred virtio state (a transport whose restore is still pending saves that pending state back verbatim).

## Vocabulary

All vocabulary is expressed as methods:

- `VmSnapshotView::of_saved_state` — decoding of a real `SavedState`.
- `VmSnapshotView::overlay` — saved units replace the corresponding live unit state; other units keep theirs; the live inventory is unchanged.
- `VmSnapshotView::after_downtime` — every unit advances its own guest-visible clocks by the downtime, as observed by its next save; zero downtime is the identity by construction.
- `VmSnapshotView::restore_from` — the single definition of the post-restore snapshot state: `request.compensate(self.overlay(request.snapshot))`.
- `VmSnapshotView::installs` — every saved unit holds exactly its compensated saved state; other units are not constrained (used by the `load` wrapper, whose pre-state has no save observer).
- `RestoreRequestView::decode` / `compensate` / `is_accepted_by`.
- `LoadedVmView::snapshot_restore_success`.

## Successful postcondition

The helper has no snapshot precondition: restore itself validates the snapshot and fails otherwise. On `Ok(())`, `old(self)@.snapshot_restore_success(RestoreRequestView::decode(&saved_state, &restore_time), final(self)@)` requires:

- the snapshot was acceptable (`is_accepted_by`): saved unit names are unique and registered, and the saved inventory is empty (a legacy snapshot) or equals the live inventory;
- the final snapshot state equals `old(self)@.snapshot.restore_from(request)`: saved units hold their saved blobs, unsaved units keep their state, the inventory is unchanged, and with a downtime adjustment every unit is compensated;

and in addition `pre_execution_representation(final(self), true)`: the VM is marked restored, holds the restore stop guard, and is not running.

`InitializedVm::load` ensures, on `Ok(loaded)` with a snapshot, that the request was accepted, that `loaded@.snapshot.installs(request)`, and the same pre-execution representation.

The `Err` arm has no rollback guarantee. Failure non-publication and no-resume properties belong to separate caller contracts.

### Downtime compensation

With a downtime adjustment, restore advances each state unit's time (`StateUnits::advance_time`), the VP TSC and APIC timers (`PartitionUnit::advance_tsc`, x86_64), and the backend snapshot clock, so the restored state is not the saved state alone. The TOP states this uniformly: each unit's saved state is replaced by `restore_proof::unit_after_downtime(name, state, downtime)`, a temporary `uninterp` proof debt whose meaning is the unit's own compensation. It is anchored like the rest of the View: the restored VM's next `save` must observe exactly this state. Without a downtime adjustment the property involves no uninterpreted function at all.

## What this TOP deliberately does not claim

- Destination-frame preservation (prepared RAM, external-resource bindings, destination compatibility, component configuration). Stating these soundly requires their own anchoring observations; unanchored placeholder Views would make such clauses vacuous. They are future work.
- Per-VP selection. `load` on current main instantiates every destination VP, and VP identity checks are performed by the partition unit's restore, which fails otherwise.
- Host-bound runtime state (tasks, timers, handles, host queues). It is not serialized by `save` and is therefore not snapshot state.
- Guest readiness, deferred-state activation, caller publication, or rollback.

## Residual trust and soundness limits

- **Round trip is necessary, not sufficient.** The anchoring pins restore to what `save` observes. A hypothetical implementation that merely stashed the blob and returned it from `save` would also satisfy it. With the real, frozen executable code this cannot happen, because each unit's `save` serializes and each unit's `restore` installs real device state; connecting the View to guest-visible device behavior is a per-device refinement beyond this TOP.
- **Blob equality is byte-level value equality.** It requires each unit's re-save of restored state to reproduce its saved payload. A unit whose restore normalizes its state would make the property unprovable (never vacuous) and would need a reviewed per-unit equivalence.

## Open and closed layers, TCB, and proof debt

The human-owned open layer in `worker/dispatch.spec.rs` defines the vocabulary and success property.

The TCB manifest sanctions three new BOTTOM-level entries for the snapshot wire form: `ExSavedState` and `ExSavedStateUnit` (transparent `external_type_specification`, exposing only declared public fields) and `ExSavedStateBlob` (`external_type_specification` + `external_body`: an opaque payload compared as a value). These give the anchor its fixed meaning.

The engineer-owned closed layer in `worker/dispatch.proof.rs` holds two temporary `uninterp` symbols recorded in `UNINTERP.json` — `loaded_vm_representation` (fixed by the `save` contract) and `unit_after_downtime` — plus the concrete `pre_execution_representation`. `LoadedVm::save` and `LoadedVm::restore_snapshot_state` are temporary `external_body`. The boundary check therefore reports this state as `INCOMPLETE`, not complete.

`verification/tools/verify.sh` selects the extracted helper as the focused TOP.
