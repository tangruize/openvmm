# Duration nanosecond observation for virtual-time advancement

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
bridge. The two observations are proposed external representation semantics,
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
the frozen base does not. The run proposal additionally contains the concrete
`VmTime::view` needed for its scalar proof; the freeze proposal does not.
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

`run.patch` contains the same proposed boundary plus the accepted scalar View,
constructor/accessor annotations, and the proof of the actual
`VmTime::wrapping_add` implementation. Those accepted scalar edits are needed
in this independent patch because they are currently dirty working-tree input,
not part of the working branch tip. Neither patch changes the executable body
of any production operation.

The unconditional postcondition is

```text
result@ = (self@ + floor(elapsed_nanoseconds(&d) / 100)) modulo 2^64
```

Here `VmTime@` is the existing closed View of the real private `u64` field.
There is no new precondition on any public caller. The retained caller report,
`research/vmtime-scalar-interface/callers.log`, identifies the real
`VmTimeKeeper::advance -> VmTime::wrapping_add` edge, together with device
timer callers. Direct source also connects `KeeperUnit::advance_time` to
`VmTimeKeeper::advance`. No async caller is claimed proved by this arithmetic.

The proof lemma establishes that dividing any `u128` nanoseconds by 100 and
adding a `u64` cannot overflow `u128`. A bit-vector step establishes that
narrowing the sum to `u64` equals reduction modulo `2^64`. The real production
body consumes that lemma and the proposed `as_nanos` contract. The lemma is
not a substitute executable; the original expression still invokes the
actual `d.as_nanos()`, `u128::wrapping_add`, and cast.

Candidate reproduction and native outputs are under
`research/vmtime-duration-interface-candidate/`: `verify.command`,
`verify-module.command`, `production-body.log`, `production-module.log`, and
the final-layout `production-final.log`.
The initial contract-only attempt leaves a real arithmetic postcondition
unproved; the named truncation lemma supplies the missing proof, without
strengthening the trusted interface. Lifetime checking is enabled and no
rlimit annotation, `assume`, `admit`, or project-body cut is added.
`freeze.patch` and `run.patch` are independent proposals for their respective
branch tips; they are not instructions to modify the current dirty checkout.

## Explicitly excluded conclusions

This conditional construction does not prove the selected library implementation
of Duration, install its observation semantics, or establish the frozen
`snapshot_restore_success` contract. It does not connect the Duration argument
to decoded `request.downtime_ns`, prove asynchronous stopped-clock installation,
define any of the four existing restore representation bridges, or discharge
the TOP body's `external_body`. Those remain proof obligations.

The existing boundary rejection of TCB-manifest provenance is independent.
The restore wrapper's function-selected 0/0 result, with its existing
`--no-lifetime` flag, is not full-crate verification and is not evidence for
this candidate. The candidate proof does not use that wrapper or flag.
