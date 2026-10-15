# Unified snapshot save/restore contract

## Status

This is an initial specification candidate for the latest `nanvix/openvmm` task checkout. It is intentionally incomplete and must be judged against the real callers and component implementations before proof work treats it as authoritative.

The contract reuses the earlier restore abstraction: a decoded snapshot plus prepared RAM and external resources denotes a logical VM state; the returned `LoadedVm` independently denotes a logical VM state; successful restore relates those states at the protected return before guest execution. The new part is that save and restore now share exactly the same logical state and compose through one snapshot interpretation.

## Common logical state

`LogicalVmState` contains the retained developer-visible state:

- Machine/profile identity needed to interpret the snapshot.
- Guest RAM as GPA-to-byte content, not a host pointer or file descriptor.
- Stable ordered VP identity and retained architectural/optional state.
- Stable component identity, serialized payload kind/content, and deferred private state that must survive until later application.
- Clock/timer state at an identified observation boundary.
- Generation ID and entropy when they are part of the selected snapshot mode.

Host virtual addresses, file-descriptor numbers, locks, task identities, channels and handles are not compared by identity. Their rebuilt instances must nevertheless implement the projected RAM, CPU, component and resource state.

## Complete snapshot view

`SavedState` is only the component envelope. `SnapshotArtifactsView` also includes the exact RAM generation/backing, manifest interpretation, capture-time clock metadata and other artifacts required by the selected mode.

The active interpretation is:

```text
snapshot_view(artifacts): SnapshotArtifactsView -> LogicalVmState
project_vm(resources): RuntimeVmView -> LogicalVmState
```

Neither function may return an independently populated witness that merely repeats expected values. `snapshot_view` must be connected to decoded artifacts and memory-file ranges. `project_vm` must be connected to actual runtime resources and observers.

## Save relation

There must be one capture cut `t_cut` at which the selected logical state is stable:

```text
snapshot_view(published_snapshot) = project_vm(source_vm_at_t_cut)
```

The capture boundary must establish, from the real path:

- State units and VPs have reached the required stopped/quiesced state.
- Host input and every retained-state mutation path are gated or otherwise excluded through the relevant cut and publication interval.
- Registration/inventory used by save is stable.
- Component payloads and the clock anchor describe the same cut.
- The RAM bytes published for the snapshot are exactly the bytes of the selected generation at the cut.
- Filesystem publication commits one complete generation or reports failure; a staging directory or callback success is not yet a successful snapshot.

The proof route starts at `VmController::handle_guest_snapshot_request`, follows `VmRpc::QuiesceForSnapshot`, `LoadedVm::quiesce_for_snapshot`, `LoadedVm::save`, RAM flush, snapshot publication and the resulting manifest/state/RAM artifacts.

## Restore relation

The restored VM boundary is successful return from `InitializedVm::load(Some(_))`, after `LoadedVm::restore_from_snapshot` has applied component state and restore-time clock policy and acquired the temporary VP stop guard, but before guest execution or external-ready publication.

For compatible artifacts, prepared resources and explicit environment inputs:

```text
project_vm(returned_vm) = allowed_restore_transform(snapshot_view(snapshot), environment)
```

`allowed_restore_transform` may:

- Rebuild host-only resources without preserving identity.
- Apply the declared downtime transformation to clocks and timers.
- Replace generation ID or entropy only when an explicitly selected policy permits it.
- Select a supported stable VP prefix without changing the meaning of preserved VP identities.

It may not silently drop component payloads, RAM bytes, VP state, deferred device state, or other retained state. The prepared RAM backing, snapshot generation guards, destination capabilities, attachments and authenticated clock samples are explicit inputs, not facts chosen from the returned VM.

The real code bridge is:

```text
InitializedVm::load(Some(saved_state))
    -> LoadedVm::restore_from_snapshot(saved_state)
       -> begin_snapshot_restore(&saved_state)
       -> LoadedVm::restore(saved_state)
       -> finish_snapshot_restore(profile_span)
```

Proving only the helper without proving this call and returned value connection is insufficient.

## Composition

For a supported capture, a compatible destination and successful operations:

```text
project_vm(restored_vm)
    = allowed_restore_transform(project_vm(source_vm_at_t_cut), environment)
```

With the identity restore policy and zero downtime, the selected logical state is preserved exactly. With a non-identity policy, every permitted difference must be named in the policy and all unaffected fields remain equal.

## Compatibility

Compatibility must come from actual caller validation and supported configurations, not from whatever the implementation happens to accept. It includes at least:

- Machine/profile and snapshot format compatibility.
- RAM layout and size compatibility.
- Stable inventory and component identity compatibility.
- VP capacity, stable identity and optional-field capability compatibility.
- Destination feature/capacity contracts for deferred devices.
- Clock frequency and time-ABI requirements.
- Typed external resource reconnection policy.

The exact backend matrix remains a Ground decision. Unsupported combinations must be reported explicitly rather than erased from the projection.

## Failure

A failed save does not publish a successful snapshot generation. A failed restore does not return or externally publish a successfully restored VM and does not permit guest execution.

Full rollback is not required. State-unit restore, multi-VP work and clock adjustment may partially mutate unpublished destination resources. The result must preserve ownership and cleanup information needed by the caller to discard them safely.

Permitted failure categories must name a real unsupported input or external impediment. An internal callback error, panic, missing required state or coordination defect is not automatically an allowed external failure.

## Refinement obligations

- Replace abstract payload bytes with the relevant decoded component relations where semantic content matters.
- Reuse the detailed earlier restore treatment of prepared RAM, deferred virtio state, VP identity, reconnect policy and clock samples when it still matches current source.
- Decide exact overflow/wrapping and timer-expiration semantics from current production code.
- Identify the exact capture cut and publication commit point.
- Establish concrete `snapshot_view` and `project_vm` bridges rather than supplying proof-only observations.
- Attach the final top-level contract to the real save/capture route and `InitializedVm::load(Some(_))`.
