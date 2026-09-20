# Named parameters for the production VP selector

The requested frozen-source change is limited to the two closure parameters
in `vmm_core/src/partition_unit/vp_set.rs::select_instantiated_vp_states`.
Replace each tuple-pattern parameter with `entry` and perform the identical
`let (vp_index, _) = entry;` destructuring at the start of that closure.
The selector's signature and its validator, map, filter, cast, collection,
and error propagation remain unchanged. This is a source normalization,
not a claim that the current executable computes an incorrect result.

## Why an annotation alone is insufficient

`research/restore-vp-prefix-proof/native-body.log` records the real selector's
rejection with only `verus_verify` and its macro import. The pinned frontend
requires a binding parameter: `rust_to_vir_expr.rs:4092-4107` invokes
`pat_to_mut_var`, whose `PatKind::Binding` check at lines 294-310 rejects the
existing tuple pattern before verification conditions are generated.
The retained standalone control establishes that local destructuring after a
named parameter is accepted, but is not the evidence for this proposal's
production behavior or admission.

No intermediate postcondition can change this Rust parameter's AST shape.
Trusting the selector body or changing verifier semantics would conceal,
rather than discharge, the obstacle. The smallest proposed frozen change
relocates the same irrefutable pattern inside each closure.

## Preservation for every payload type

For any `T`, `states.iter()` yields `&(VpIndex, T)`. In both spellings, Rust's
reference-pattern binding rules bind `vp_index` as `&VpIndex`; the wildcard
does not bind, inspect, clone, or move `T`. The map still copies only the
`Copy` index. Introducing a shared-reference local neither invokes user code
nor introduces an owning value with a destructor.

The owning `states.into_iter()` still yields the original `(VpIndex, T)`
pairs. `Iterator::filter` supplies a shared reference to each such pair to
its predicate. Its new local destructuring therefore creates exactly the
same `&VpIndex` as the old parameter pattern. The predicate still calls the
same `VpIndex::index` method and compares against exactly
`instantiated_vp_count as u32`. There is no new range assumption or count
restriction: counts above capacity and narrowing on 64-bit hosts retain
their previous behavior.

Thus the map supplies the same ordered index stream to the unchanged
full-capacity validator, before the owning iterator is created. Unknown,
duplicate, and missing entries are rejected by the same code in the same
order, including invalid dormant entries. On error, the same `?` returns
the same error and drops the owned input. On success, the filter makes the
same decision on each original pair; collection preserves input order,
keeps the same payload objects, and discards the same rejected objects.
Closure capture of the instantiated count, input borrowing, owning moves,
and the ordering of drops are unchanged. This argument establishes
equivalence to the old selector for arbitrary payloads, not an independently
verified selected-prefix theorem or a proof of the validator.

## Actual production candidate, not a replacement implementation

`freeze.patch` contains only the two normalizations. `run.patch` independently
contains those normalizations plus the prelude import and `verus_verify`
annotation needed to expose this existing function to the pinned verifier.
It adds no postcondition, precondition, assumed theorem, representation View,
temporary trust marker, rlimit annotation, or sanctioned declaration.

The commands in `research/restore-vp-prefix-proof/check-prepared-candidate.sh`
run the real generic function from an isolated archive of the working source.
The only production-source changes in that archive are exactly `run.patch`.
The script uses the installed pinned Verus and Z3, existing packaged protoc,
and the function-focused selector:

```text
cargo verus focus -p vmm_core -- --verify-only-module partition_unit::vp_set --verify-function select_instantiated_vp_states --no-lifetime --multiple-errors 20 --num-threads 1 --triggers-mode silent
cargo check --locked -p vmm_core --tests
```

The complete production diagnostics are `candidate-admission.log` and
`candidate-rust-prepared.log` in that directory. The normalized map and
filter no longer produce the tuple-parameter rejection. Translation reaches
the filter's `vp_index.index()` at candidate line 931 and reports these
next declaration failures: unsupported `VpIndex`, unsupported `RestoreError`,
ignored `validate_restore_vp_indices`, and unsupported `VpIndex::index`.
No selection verification condition or closure theorem is established.
The frontend's suggested external declarations are not included in either
patch; native declarations and actual body contracts remain project proof
work, not new BOTTOM-level trust.

`candidate-behavior.patch` is a separate test-only overlay, not part of the
request. `check-behavior.sh` exercises the actual normalized function and
existing validator tests, including non-Clone owning payload identity,
unsorted input order, drop order, full-capacity errors, count-above-capacity,
and the existing narrowing behavior. It does not replace the generic
equivalence argument or provide a Verus proof.

The initial isolated commands lacked the already-installed packaged protoc.
Those setup diagnostics are retained in `candidate-verus.log` and
`candidate-rust.log`; the prepared run connects that existing dependency
without modifying source or installing a replacement toolchain.

## Relation to the frozen restore contract

The actual `VpSet::restore` supplies `vp_capacity` and `vps.len()` to this
selector, then uses each selected index to find the VP and sends that pair's
payload to `StateEvent::Restore`. `PartitionUnitRunner::restore` supplies
the decoded pairs. This is why payload identity, not merely the selected
indices, matters to the TOP contract's `restore_vp_projection`.
These are direct source observations: the callgraph command reports that
the maintained `.verus_agent/proof_state.json` snapshot is absent.

No proof is installed on either authoritative branch. The selector theorem,
full validator coverage, caller count invariants, decoding, RPC installation,
time adjustment, and final TOP connection remain unproved. The TOP body,
four uninterpreted representation bridges, and existing declaration debt
are unchanged. Both frozen manifests, all sanctioned declarations, verifier
semantics, and the existing restore-guard-native-presence request are
untouched. The authoritative integration check still selects a scaffolded
restore body; boundary scanning is worker-scoped and has an existing
TCB-manifest provenance rejection, not a whole-program trust certificate.
