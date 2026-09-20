# Require partition state for x86 time-adjusted snapshot restore

## Proposed boundary change

Reject a snapshot at the start of `LoadedVm::restore_snapshot_state` when
`guest_arch = "x86_64"`, `restore_time` is `Some`, and `SavedState.units`
contains no entry whose name is exactly `"partition"`. Return the explicit
error `time-adjusted snapshot restore requires partition state`.

The guard precedes frequency queries/setters, state-unit restore, time
advance, VP TSC adjustment, backend clock adjustment, and stop-guard
acquisition. It examines payload names, not `SavedState.inventory`.
The private helper contains the actual production guard and is called first
by the real TOP body; it allows isolated execution of the guard without
constructing a hardware-backed `LoadedVm`. No alternate restore body is added.

`freeze.patch` and `run.patch` propose exactly the same executable change.
There is no TOP specification, TCB declaration, manifest, intermediate
specification, View, invariant, or proof change. Only Human may authorize
application. The generic sparse `StateUnits::restore` semantics are unchanged.

## Conflict and why bridge work alone is insufficient

The accepted omission diagnostic is recorded in
`research/GROUND_TRUTH.md`, under "Unsaved VP frame: omitted partition
payload", and in `research/restore-unsaved-vp-frame/`.
Its real `StateUnits` inventory contains `"partition"`, but its saved payload
vector omits that unit. Production restore returns success without dispatch,
and the production partition/VP adjustment path changes the two destination
counters 77 and 78 to 1077 and 1078 for the same omitted payload and time
policy. No VP restore occurs. These existing executions are reused, not
repeated as part of this proposal.

The source connection is:

- `InitializedVm::load` registers `"partition"` and passes the decoded saved
  state and time policy directly to the TOP
  (`dispatch.rs`, partition construction and call to `restore_snapshot_state`).
- The TOP calls `LoadedVm::restore`, then, for `Some(restore_time)`, calls
  `StateUnits::advance_time` and the x86 `PartitionUnit::advance_tsc`.
- `LoadedVm::restore` checks the optional nonempty inventory and calls
  `StateUnits::restore`. `StateUnits::state_change` returns `Ok(None)` for
  an absent payload without dispatching a restore RPC
  (`vmm_core/state_unit/src/lib.rs`).
- The partition TSC RPC reaches every instantiated VP independently of the
  saved payload domain (`vmm_core/src/partition_unit.rs` and
  `vmm_core/src/partition_unit/vp_set.rs`).

The existing `frozen_precondition.rs` includes the real `dispatch.spec.rs`.
It witnesses an admitted request with an empty saved VP map and a frozen
projection that preserves initial VP 0, rather than returning its incremented
counter image. Its retained log and production execution logs are evidence
for this bounded diagnostic, not a whole-TOP execution or a bridge refinement
proof.

The accepted selected-and-saved diagnostic
(`research/restore-tsc-consistency/production_path.rs` and `policy-image.log`)
supports a policy-indexed requested VP image for a **present saved entry**:
the desired TSC is saved TSC plus the checked adjustment's mathematical
delta. That construction cannot transform a nonexistent saved entry.
`restore_vp_projection` preserves the initial VP image when that entry is
absent. Moreover, `decoded_restore_request_view` receives only saved state,
restore policy, and selected count, not the destination's initial counter.
Identical request inputs cannot recover the two different destination-dependent
post-counters. Inventing saved entries, discarding TSC, introducing restore
history into an actual-state View, or using an artificial compatibility
precondition would hide rather than discharge this obligation.

## Intentional accepted-input change

Every x86 `Some(restore_time)` request must now contain a partition payload.
This includes `Duration::ZERO` and nonzero durations whose computed TSC delta
rounds down to zero. Those cases need not exhibit the positive-delta witness;
rejecting their omission is an explicit, conservative policy-level input
restriction, not a claim that every omitted payload changes a counter.
The guard does not calculate a delta or depend on frequency or destination
state. Presence of the restore policy consistently requires the partition
state to which that policy applies.

An omitted partition payload with `restore_time = None` follows the original
sparse restore path. Non-x86 guest builds have neither the helper nor its call.
A present partition payload passes this guard unchanged, including malformed
payloads that existing decoding or restore code subsequently rejects.
Inputs satisfying the rejection predicate that already failed elsewhere
may now fail earlier with the new error. No preservation of their former
error text or partial side effects is claimed.

Presence is distinct from VP inventory completeness. Within a present
partition payload, `PartitionUnitRunner::restore` invokes `VpSet::restore`;
`select_instantiated_vp_states` calls `validate_restore_vp_indices` and
requires exactly one entry for every capacity VP before selecting the active
prefix. Clearing the VP vector is therefore already rejected downstream.
The new guard is not a replacement for that validation and does not parse
or reinterpret the partition schema. Partition backend restore may already
have run before this downstream error; the proposal adds no rollback promise.

## Exact frozen success contract and remaining proof obligations

The TOP's precondition and success postcondition remain byte-for-byte intact.
Its existing `Err(_) => true` arm permits the new early rejection without a
new precondition, a weaker success relation, or a rollback obligation.
For requests that pass the guard, the remaining executable body and
arguments are unchanged. The proposal removes the demonstrated omission
success continuation; it does not establish the success theorem.

The TOP `external_body` and all four `uninterp` bridges remain outstanding:
`decoded_restore_request_view` owes faithful saved-state decoding and
policy interpretation; `decoded_load_restore_request_view` additionally
owes coherence with the real load caller and its selection policy;
`initialized_vm_representation` owes construction/configuration/resource
representation; `loaded_vm_representation` owes actual partition/VP,
component, memory, time, resource, and lifecycle representation. The complete
TOP body, its new guard, successful partition dispatch and VP coverage,
checked adjustment arithmetic, real backend refinement, and pre-execution
guard/lifecycle facts still require proof. No temporary marker is removed,
added, or promoted to permanent trust, and no rlimit annotation is changed.
`InitializedVm::load` and the two `ExRestoreReadyFile` declarations remain
exactly the sanctioned BOTTOM declarations.

## Candidate execution boundary

`research/restore-time-partition-presence-validation/validate_candidate.py`
applies the proposal only in a disposable archive of the working tree's
committed sources. It checks equality of the two branch inputs for the edited
file, checks the two patch contents, and checks that the real TOP calls the
guard before any affected operation. A separate, non-proposed instrumentation
patch exposes the existing partition schema helpers and includes unit tests
in `openvmm_core`; it changes no production restore or adjustment method.

`candidate_tests.rs` calls the candidate's real guard, then the real
`StateUnits::restore`, partition and VP runners, time/TSC adjustment,
serialization, and stop-guard path when the guard permits it. It reuses the
unchanged deterministic register-backend portion of the accepted
selected-and-saved fixture, not a replacement restore implementation.
Saved VP state comes from production `save_all`, and successful complete
restores compare the entire generated `VpSavedState`, not only TSC.

The matrix covers destination counters 77 and 78, complete saved TSC 1000,
omitted and complete payloads under no policy, zero downtime, a 1 ns policy
with zero computed delta, and 250 ms at 4003 Hz with delta 1000. It also
covers the existing present-but-missing-VP error and verifies that inventory
presence or a differently named payload does not satisfy the guard.
The fixture checks exact VP commits and partition restore counts.
It does not construct `LoadedVm`, execute backend frequency setup or backend
clock advance, run a hardware hypervisor, or discharge any representation
bridge. Whole-TOP execution and verification are not claimed.

Commands and their full outputs are kept outside the three-file package in
`research/restore-time-partition-presence-validation/`. The maintained
`freeze_request submit` command is the authority for request validation.
Candidate unit tests do not substitute for its TOP/TCB and executable
comparison, nor for the required authoritative-source drift checks.
