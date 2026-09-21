# Duration observation for virtual-time advancement and exact restore elapsed time

## Exact requested boundary

The proposed external interface is in
`vm/vmcore/src/vmtime_duration/observation.spec.rs`, owned by
`vmcore::vmtime::duration_observation`. It has three trusted declarations:

| Declaration | Required concrete interpretation |
| --- | --- |
| `whole_seconds(d: &Duration) -> u64` (`uninterp`) | The actual `core::time::Duration::secs` field of this argument, also observed by the library's `as_secs`. |
| `subsecond_nanoseconds(d: &Duration) -> u32` (`uninterp`) | The actual `nanos.as_inner()` value of this argument, also observed by the library's `subsec_nanos`. |
| `duration_as_nanos` (`external_fn_specification` for `Duration::as_nanos`) | Its actual `u128` return equals `whole_seconds(d) * 1_000_000_000 + subsecond_nanoseconds(d)`, and the subsecond observation is below `1_000_000_000`. |

`elapsed_nanoseconds` is a defined natural-number sum, not another opaque
bridge. This same observation is needed both by the actual
`VmTime::wrapping_add` body and by the exact
`loaded_vm_representation(vm).state.virtual_time.elapsed_since_snapshot_ns`
connection to the retained restore request. Rounding keeper ticks cannot
recover that exact value. The two observations are proposed external representation semantics,
not project-owned functions claimed proved. Their intended concrete
interpretation is part of the trust being requested: merely choosing some
ghost values that make the arithmetic work would not satisfy this request.
No universal axiom, executable constructor/accessor contract, generic
`transmute` rule, or alternate Duration representation is introduced.
The subsecond bound is supplied at the one real operation being specified;
there is no separate global representation axiom.

The manifest proposal sanctions exactly these three symbols/markers.
The source scope adds only `vm/vmcore/src/vmtime_duration` so that the checker
actually sees this interface and its specification dependencies, through its
ghost-only `mod.rs`. It retains the same frozen
branch, TOP goal, and every existing sanction. Ghost-only module wiring and
vmcore's existing workspace vstd dependency/Verus opt-in admit the interface.
The existing vstd `ExDuration` declaration is unchanged.

This directory scope deliberately excludes unrelated vmcore implementations.
An earlier whole-vmcore scope attempt reached an unsupported `dyn` parse and
ambiguous same-named, platform-specific executable comparisons in existing
`interrupt.rs` and `vm_task.rs`. No checker or unrelated source is changed
to accommodate that failure. The separate authoritative executable comparator
checks the entire candidate `vmtime.rs` against its frozen source; its output
is `candidate-exec-drift.json`. The new directory contains every proposed
trusted declaration, not an unscanned trust dependency.

## Scope of semantic comparison

The frozen TOP file and the existing `InitializedVm::load` and
`ExRestoreReadyFile` declarations are unchanged. The only proposed external
trust is the three Duration declarations listed above.

The comparison must distinguish that trust from project-owned Views. The
working base already contains `VpIndex::view` in both
`vm/vmcore/vm_topology/src/processor.proof.rs` and the retained research
snapshot `research/restore-vp-index-coverage/bitmap-intake-processor.proof.rs`;
the frozen base does not. The working base also already contains the concrete `VmTime::view` needed
for its scalar proof; the frozen base does not.
These differences do not propose new external guarantees for either View.

The retained source-analysis diagnosis explains why the semantic comparator
can nevertheless attribute all three Views to `InitializedVm::load`: its
bare `view` dependency matching conservatively follows same-named methods,
and adding the Duration directory expands common-root source discovery.
That explanation is evidence for reviewing these particular findings, not
proof of semantic equivalence and not authority to change or sanction the
Views. Reviewer must inspect the exact two proposals and the preserved
advisory outputs. No checker repair or unrelated View trust is requested.

Executable comparison also needs to distinguish the existing
`ExtractTopologyConfig` implementations for `ProcessorTopology<X86Topology>`
and `ProcessorTopology<Aarch64Topology>` in `dispatch.rs`. Both bodies and
their intervening source are byte-identical between the current branch bases,
and neither patch touches that file. Comparing the X86 specialization against
the Aarch64 specialization is not evidence that this request changes either
implementation. This source comparison explains that particular advisory;
it does not substitute for review of the requested Duration boundary.

## Source justification and the existing obstruction

Reuse `research/vmtime-duration-binding/source.log`, `production-body.log`,
`pattern-type.log`, and `transmute.log`. These diagnostics concern the selected
library and the same real production body, not a copied clock implementation.

The verifier-selected Rust 1.98.1 library reexports `core::time::Duration` at
`library/std/src/time.rs:35`. In `library/core/src/time.rs:81-84`, Duration
stores `secs: u64` and `nanos: Nanoseconds`. At lines 631-633, `as_nanos`
computes exactly `secs as u128 * 1_000_000_000 + nanos.as_inner() as u128`.
The selected library's public observations at lines 506-508 and 575-577
return those same components. In `library/core/src/num/niche_types.rs`,
`Nanoseconds` wraps the range `0..=999_999_999`; its `as_inner` invokes the
bodyless `mem::transmute` intrinsic.

The exact maximum total is
`(2^64 - 1) * 1_000_000_000 + 999_999_999`, below `2^128`, so the library
sum represents elapsed nanoseconds without overflow. The proposed observation
semantics are valid for every actual Duration, not merely selected constructors
or restored request values.

The existing native diagnostic rejects `Duration::as_nanos` as unsupported.
The deeper, separately retained probes reject the actual pattern type and
the same `transmute` intrinsic even at the simpler identity instantiation.
The frozen opaque Duration declaration also prevents private-field projection;
another transparent declaration would conflict with it. A missing
project-owned postcondition alone is not the reason for this request.
The smallest operation reached by production `wrapping_add` is `as_nanos`;
trusting its bounded observation is narrower than granting arbitrary
intrinsic semantics or trusting any project-owned clock operation.

## Conditional production-body construction

`run.patch` extends the working tip's existing scalar View and
constructor/accessor annotations with the proof of the actual
`VmTime::wrapping_add` implementation. It updates, rather than recreates,
`vmtime.proof.rs` and does not duplicate vmcore's existing Verus manifest
entries. `freeze.patch` independently supplies the interface and its wiring
against the frozen tip. Neither patch changes the executable body of any
production operation.

The unconditional postcondition is

```text
result@ = (self@ + floor(elapsed_nanoseconds(&d) / 100)) modulo 2^64
```

Here `VmTime@` is the existing closed View of the real private `u64` field.
There is no new precondition on any public caller. The retained caller report,
`research/vmtime-scalar-interface/callers.log`, identifies the real
`VmTimeKeeper::advance -> VmTime::wrapping_add` edge, together with device
timer callers. Current source in `vmm_core/src/vmtime_unit.rs` and
`vm/vmcore/src/vmtime.rs` confirms that edge and the preceding
`KeeperUnit::advance_time -> VmTimeKeeper::advance` call. The maintained
callgraph cannot currently be read because `.verus_agent/proof_state.json`
is absent; no graph rebuild or complete caller-coverage claim is made.
No async caller is claimed proved by this arithmetic.

The proof lemma establishes that dividing any `u128` nanoseconds by 100 and
adding a `u64` cannot overflow `u128`. A bit-vector step establishes that
narrowing the sum to `u64` equals reduction modulo `2^64`. The real production
body consumes that lemma and the proposed `as_nanos` contract. The lemma is
not a substitute executable; the original expression still invokes the
actual `d.as_nanos()`, `u128::wrapping_add`, and cast.

The earlier contract-only attempt in
`research/vmtime-duration-interface-candidate/production-body.log` leaves a
real arithmetic postcondition unproved; the named truncation lemma supplies
the missing proof without strengthening the trusted interface.
Current source-matched candidate inputs and commands are under
`/home/ruize/.argus-skill/projects/9b3370b0cf02/research/refresh-duration-interface-freeze-request/workspace/`.
`research/GROUND_TRUTH.md` records unresolved bindings before experiments;
`research/selected-library.log` records the currently selected interface;
`research/verify-production.sh` invokes module verification of the actual
vmcore source, with output in `research/production-module.log`.
Lifetime and trait-conflict checking remain enabled, the command-line
rlimit is 50, and no `assume`, `admit`, or project-body cut is added.
`freeze.patch` and `run.patch` are independent proposals for their respective
branch tips; they are not instructions to modify the live checkout.

## Exact elapsed-time use and its remaining binding obligations

The accepted elapsed-time investigation is recorded in
`/home/ruize/.argus-skill/projects/9b3370b0cf02/handoffs/a535358753b0/round-0001.json`
and its `research/restore-elapsed-time-representation/workspace/research/`
sources. Its `duration_observer.probe.log` rejects the real `as_nanos`
observer under the present boundary. Its history helpers retain the actual
opaque `Option<Duration>`, not a natural number inferred from a keeper clock.

`run.patch` carries those helpers, extended by a partial numeric observation,
under `research/restore-duration-nanoseconds-interface/`. The observation is
undefined for Unbased, zero for Completed(None), and
`elapsed_nanoseconds(&duration)` for Completed(Some(duration)). It does not
interpret invalid history as physically zero elapsed time. The executable
interface probe calls the actual `duration.as_nanos()` and relates its return
to the history containing that same Duration. It is not an alternative
implementation of a production restore operation.

The conditional projection lemma consumes the frozen
`restored_virtual_time` definition. It requires decoded
`has_time_adjustment` to match the actual optional request and, when present,
decoded `downtime_ns` to equal that Duration observation. These are explicit,
source-supported but unproved production-decoding obligations, not new TOP
preconditions or trusted guarantees. Under those bindings, the completed
receipt supplies exactly the frozen `elapsed_since_snapshot_ns`, without
the division by 100 used for keeper ticks. The existing examples distinguish
0, 1 and 99 ns and a full keeper wrap despite coinciding scalar ticks.

The source attachment remains unproved: invalidate old history before the
first mutating TOP operation; retain the actual request; record completion
only after the final stop-guard await and before successful return. Errors,
cancellation, generic restore, reset and subsequent starts must invalidate
or frame history correctly. The task-owned keeper, its asynchronous updates,
the other fields of `loaded_vm_representation`, and a total representation
across lifecycle phases remain separate obligations. The retained remote-TPM
counterexample is unchanged: the receipt cannot imply RPC success or equality
of every configured component clock.

For isolated reproduction, use the candidate's selected Verus executable:

```text
verus --crate-type lib --crate-name restore_duration_history --edition 2024 \
  --rlimit 50 --num-threads 1 --triggers-mode silent \
  research/restore-duration-nanoseconds-interface/restore_history.proof.rs
```

The production command is `cargo verus focus --offline --locked -p vmcore --`
with `--verify-only-module vmtime --rlimit 50 --multiple-errors 4
--num-threads 1 --triggers-mode silent`. Neither command disables lifetime
or trait-conflict checks. The helper's output is `research/retained-duration.log`
in the isolated workspace; it is not evidence that TOP itself verifies.

## Explicitly excluded conclusions

This conditional construction does not prove the selected library implementation
of Duration, install its observation semantics, or establish the frozen
`snapshot_restore_success` contract. It does not discharge the real request
decoding, asynchronous stopped-clock installation, or history-attachment
obligations. All four existing bridges remain uninterpreted:
`decoded_restore_request_view`, `decoded_load_restore_request_view`,
`initialized_vm_representation`, and `loaded_vm_representation`. The TOP body's
`external_body` also remains. No temporary marker is removed or added in
production, and the existing BOTTOM declarations are unchanged.

The existing boundary rejection of TCB-manifest provenance is independent.
The restore wrapper's function-selected 0/0 result, with its existing
`--no-lifetime` flag, is not full-crate verification and is not evidence for
this candidate. The candidate proof does not use that wrapper or flag.
This is an update of the existing request identity, not a second submission.
The installed command rejects `submit` for an existing ID; current-base
package checking uses `freeze_request validate
restore-duration-nanoseconds-interface`. Historical validity does not
authorize this boundary or substitute for current validation.
