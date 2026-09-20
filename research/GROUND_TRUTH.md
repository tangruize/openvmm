# Instantiated VP prefix selection

## Bitmap package readiness continuation

The continuation explicitly requires committing the three request files and
leaving the worktree clean while retaining all dirty work. The earlier
no-commit blocker is superseded by this requested delivery action, not by
package validation. The proposed normalization still must not be applied.

The delivery retains the accessor proof, metadata, existing research changes
and unrelated PIPELINE_STATE entries together. Since its working tip
contains the accessor work, `run.patch` is refreshed to contain only the proved bitmap
helper, its native contract and the unchanged call-site substitution.
`freeze.patch` and the candidate executable/proof construction are unchanged.
The prior patch is retained in `bitmap-run.patch` as historical evidence.
Candidate proof and Rust results match this resulting source and are reused;
the six declaration failures and known boundary rejection are not rerun.

Post-commit validation/readiness output belongs under the ignored
`.verus_agent/cache/bitmap-delivery/` directory, so recording the current
result will not dirty the newly committed tree. The shared CHECKPOINT will
record the observed result after the native check, not assume readiness.

The required package-scoped clippy and documentation commands passed
(`delivery-clippy.log`, `delivery-doc.log`). Full `cargo xtask fmt --fix`
ran in an isolated current-source archive to avoid unrelated changes to
frozen source. The first run found a missing copyright header in the patch
generator, which was repaired. The next comparison incorrectly included
the formatter's own changing log; that bookkeeping bug was corrected,
without changing any proof or executable. The final full formatter and
delivered-file comparison passed (`delivery-fmt.log` and
`.verus_agent/cache/bitmap-delivery/precommit-final.log`). No individual
formatting pass was substituted. The failed durable task records remain
explicitly superseded in PIPELINE_STATE rather than overwritten as successes.

`check-bitmap-delivery-ready.command` uses native `operator_ready_issues`,
which runs `validate_request` internally for an unapplied request. It first
checks that no application receipt exists, then reports VALID and READY only
if that native combined gate returns no issues. This obtains current
validation/readiness evidence once without repeating the same patch-result
comparison. The prior round's proof, Rust, make_verify and drift evidence
is unchanged and reused; this continuation does not repair the known
boundary debt or introduce trusted semantics.

## Bitmap-update request: unresolved bindings before experimentation

The proposed candidate replaces only `mem::replace(slot, true)` in the
production validator with a private `mark_restore_vp_present(&mut bool)`
operation. Its proposed unconditional postcondition is that the result equals
the initial slot and the final slot is true. The intended body reads the
Boolean, writes true, and returns the saved value. This is source-supported
but not yet proved at this point; both possible initial values, including
the duplicate path, must be covered without an assumed operation contract.
The helper is proposed executable source, not installed project proof.

The real caller needs the previous value to choose exactly the existing
duplicate-error branch. It gets the same exclusive slot from the unchanged
checked bitmap lookup. Index iteration, errors and messages, and the final
missing-entry scan must remain unchanged. Existing LSP evidence in
`restore-vp-index-coverage/callers.log` agrees with the current production
validator/selector/`VpSet::restore` call sites. The retained native VpIndex
View/accessor proof has no precondition and is not a coverage guarantee.

The candidate must still face `RestoreError`, the four anyhow declarations,
and slice-iterator `position`. None will be assumed or repaired in this
bounded request. The generic IntoIterator input has no proved binding to
the consumed contents, nor proved exhaustion/bitmap invariants. Exact
coverage, prefix selection, payload installation, and the frozen TOP theorem
remain unproved. The earlier intrinsic rejection is reused, not rerun.

Candidate source and tests will live only in isolated research work, with
the proposed executable and proof changes carried by independent request
patches. The live executable, frozen manifests, sanctioned declarations,
accessor proof and metadata, unrelated dirty work, and prior normalization
packages are preserved. Package validation is not permission to apply.

## Bitmap-update request: candidate proved, package unapplied

`research/freeze_requests/restore-vp-bitmap-native-update/` contains exactly
`freeze.patch`, `run.patch`, and `rationale.md`. The single native submission
reported both patches applicable to the current frozen/working tips, matching
frozen specifications and executable projections, and
`freeze_request: VALID` (`restore-vp-index-coverage/bitmap-submit.log`).
No authoritative executable, branch tip, manifest or sanctioned declaration
was changed. All prior packages and unrelated dirty work are preserved.

The actual candidate's `mark_restore_vp_present` body proves, without
preconditions, that its return is the initial slot and the final slot is
true: **1 verified, 0 errors** with lifetime checking enabled
(`bitmap-helper.log`). This covers both values, including writing true
before taking the duplicate-error branch. It uses primitive Boolean
copy/assignment, no assumed update contract and no library operation.
The request includes the same real helper called by the production validator,
not a standalone control or verification-only replacement.

The annotation-only production-validator experiment reports exactly the
remaining six failures: `RestoreError`, slice iterator `position`,
`anyhow::Error`, `Error::msg`, `__private::must_use`, and
`__private::format_err` (`bitmap-validator.log`). The old `mem::replace`
diagnostic is absent; no new trusted interfaces were introduced to hide it.
This is frontend progress, not discharged validator VCs. The original
intrinsic rejection was not rerun. An initial isolated setup failure lacked
the existing toolchain path (`bitmap-helper-setup.log`); linking that
already-installed dependency resolved it without changing Verus.

Ordinary `cargo check --locked -p vm_topology -p vmm_core --tests` passed on
the exact candidate. The agent-profile nextest selection passed **43 tests**
with 27 unselected tests. The three new tests exercise the real helper
and validator, covering both previous-value outcomes and other-slot
preservation, exact error variants/messages, duplicate/unknown short-circuit
order, empty and unsorted inventories, and first-missing selection.
`bitmap-rust.log`, `bitmap-tests.log`, and `bitmap-behavior.patch` retain
the evidence. Existing selector tests also passed; these finite tests are
not a proof of exact coverage for arbitrary iterators.

The exact candidate source is `bitmap-candidate-vp_set.rs`; the request
patches retain its supporting accessor metadata and proof unchanged.
`bitmap-validator-admission.patch` contains only the experimental validator
annotation and macro feature. Both experiment overlays were removed and
the 47 MB isolated archive was cleaned up. The production archive can be
recreated from the current tip and package by
`reproduce-bitmap-candidate.command`; its focused commands and durable Rust
receipt/status are retained in this evidence directory.

The required authoritative checks ran once on the preserved production
source, after candidate cleanup:

| Command | Result |
| --- | --- |
| `make_verify --crate-root .` | Exit 0; 0 verified / 0 errors; 2.497 s |
| `boundary --crate-root . --baseline-dir .verus_agent check` | Exit 1; unchanged provenance rejection and 7 temporary locations; 1.015 s |
| `spec_drift --crate-root . --baseline-dir .verus_agent` | Exit 0; 1.190 s |
| `exec_drift --crate-root . --baseline-dir .verus_agent` | Exit 0; 4.937 s |

Full outputs are `bitmap-{make_verify,boundary,spec_drift,exec_drift}.complete.log`.
The durable handoff task's exit 1 is solely the existing boundary limitation.
Its worker-only roots do not enumerate dependency declaration debt.
`make_verify` still selects the admitted, scaffolded restore TOP and is not
a proof of that theorem. The native accessor proof and associated metadata
were byte-compared with intake; their existing body-proof evidence is reused.
No temporary marker was added, removed, moved or discharged by this request.
The TOP body marker, four representation bridges and project declaration
cut remain. Iterator-to-contents binding, exhaustion and coverage, selector
prefix/payload preservation, installation and the TOP connection remain
unproved.

**Human-review readiness remains blocked**, despite package validity.
The native `operator_ready_issues` check requires all three request files to
be committed and the worktree clean (`bitmap-readiness.log`). This mission
forbids commits and discarding dirty work, so neither requirement was bypassed.
The resulting branch tips equal intake (`bitmap-intake-tips.log`); no
unchanged validation was repeated. Reviewer receives the valid, unapplied
package and this explicit readiness blocker, not an assertion that Human
approval or application is authorized.

## VP-index coverage: unresolved bindings before experimentation

The current validator in `vmm_core/src/partition_unit/vp_set.rs:888-911`
has no Verus body annotation or coverage contract. Its actual caller at
line 918 passes `states.iter().map(|(vp_index, _)| *vp_index)` before filtering.
Any coverage contract must describe this consumed `IntoIterator`, not an
unconnected ghost sequence. The checked bitmap lookup, replacement of an
already-set bit, and final missing-bit scan support the intended successful
return guarantee, but do not yet prove it. Generic iteration, iterator
exhaustion, mutable bitmap borrowing, error construction and final scan
contracts are unresolved. No input bounds, uniqueness, coverage, or fixed
iteration order may be assumed.

`VpIndex` is the production private-field `u32` wrapper, and `index()` returns
that field (`vm/vmcore/vm_topology/src/processor.rs:262-282`). Neither it nor
`RestoreError` currently has native declaration opt-in. Whether the pinned
frontend can translate the unchanged validator body remains to be measured.
Both existing freeze-request packages stay unapplied; the selector's known
tuple-closure diagnostic is not rerun for this objective. Any declarations
or library-operation contracts still missing after this experiment remain
proof debt, not sanctioned trust.

## VP-index coverage: current diagnostic result

Continuation: Reviewer correctly distinguishes the missing-interface
diagnostic from exhaustion of Engineer-owned work. The next native
construction annotates the actual `VpIndex` declaration and its constructor
and accessor, with a closed `u32` View mapped to the private field. Its source
supports those contracts; their bodies must still pass the owning-crate
check. No coverage assertion is inferred from representation admission.
Native verification of the actual sysroot implementations, rather than
trusted wrappers or copied replacements, is being investigated separately.
The prior completed delivery and integration evidence is not rerun unchanged.

The selected exact-coverage fact is **not proved**, including the empty-count
and arbitrary-order cases. No coverage contract or iterator correspondence
was assumed. The first focused run of the current production validator
exited 101 because its unannotated module was not in the verifier selection.
Adding only native verifier entry, the required prelude, and the existing
project pattern for `proc_macro_hygiene` reached real body translation.
The complete, subsequently removed overlay is
`restore-vp-index-coverage/native-feature-annotation.patch`.

That focused production check exited 101 with nine unsupported declarations
or calls: `RestoreError`, `VpIndex`, `VpIndex::index`, `core::mem::replace`,
`anyhow::Error`, `anyhow::Error::msg`, `anyhow::__private::must_use`,
`anyhow::__private::format_err`, and slice iterator `position`.
`native-feature-focused.log` preserves the diagnostics. The earlier macro
import and feature errors are retained separately, not treated as blockers.
No validator verification conditions were generated.

The two standard-library calls remain barriers independently of the project
representation closure. `replace-admission.rs` uses just `&mut bool` and the
real `std::mem::replace`; `position-admission.rs` uses just `&[bool]` and the
real slice-iterator `position`. Both reject the same missing interfaces under
pinned Verus `0.2026.09.18.8ed93e5`. The separate `swap-control.rs` verifies
its two-outcome mutation contract (1 verified, 0 errors) using the existing
vstd `mem::swap` specification. This is a diagnostic control, not a replacement
validator or an executable normalization proposal. Commands and full output
are in `restore-vp-index-coverage/probe-operations.command` and the probe logs.

The pinned `vstd/std_specs/core.rs` specifies `mem::swap`, but not
`mem::replace`; `vstd/std_specs/iter.rs` has no `position` interface. The
known reference-identity discharge addresses lost post-borrow facts, not
these missing call declarations before VCs. This evidence establishes a
missing library verification interface in the frozen integration, **not**
an inherent inability of Verus to prove bitmap coverage. Adding the suggested
`assume_specification` declarations would enlarge trust, while substituting
different executable operations would change frozen source. Neither was
done, and no new freeze package was prepared.

The source caller tool reports rust-analyzer LSP evidence for
`VpSet::restore` -> `select_instantiated_vp_states` ->
`validate_restore_vp_indices` (`callers.log`). The selector passes the actual
mapped slice iterator at authoritative line 918, before its prefix filter.
Its use of a future coverage guarantee is still unproved. The maintained
call-graph snapshot could not be read because `.verus_agent/proof_state.json`
is absent; this setup failure is separate from the successful caller query.
The generic `IntoIterator::into_iter` interface has no input/output contents
postcondition. Any future iterator abstraction must supply a proved binding
for the actual input and justify its iteration laws for this caller; an
independent ghost sequence or assumed correspondence is insufficient.

All experimental production annotations were removed. Before the single
handoff run, a source comparison confirmed the validator, crate root,
representation owners, TOP files, manifests, and both existing freeze
packages were unchanged. The required commands then reported:

| Command | Result |
| --- | --- |
| `make_verify --crate-root .` | Exit 0; 0 verified / 0 errors; 2.520 s |
| `boundary --crate-root . --baseline-dir .verus_agent check` | Exit 1; existing provenance rejection and 7 temporary locations; 1.119 s |
| `spec_drift --crate-root . --baseline-dir .verus_agent` | Exit 0; 1.262 s |
| `exec_drift --crate-root . --baseline-dir .verus_agent` | Exit 0; 5.002 s |

The integration pass is only the existing scaffolded restore selector, not
the validator or full TOP proof. No temporary marker was added, removed, or
discharged. The TOP body marker and four bridges
`decoded_restore_request_view`, `decoded_load_restore_request_view`,
`initialized_vm_representation`, and `loaded_vm_representation` remain.
The project-owned opaque declaration cut described in the Wiki also remains;
it is not sanctioned trust. The boundary scanner's worker-only `src_roots`
does not count declaration debt in other crates or this validator.
Its rejection of TCB-manifest commit provenance is pre-existing and was not
repaired. No production Rust change remains requiring a new Rust regression
run; completed formatting, delivery validation, and review were not repeated.

## VP-index coverage continuation: native proof and intrinsic boundary

The real `VpIndex::index` body now proves `result == self@` with no
preconditions. The closed View is exactly the private `u32` field; it
introduces no representation assumption. `vp-index-accessor.log` records
1 verified / 0 errors on the owning crate. The first attempt also annotated
the const constructor, which the attribute frontend rejected with
`cannot find function new in this scope`. That unnecessary constructor
annotation was removed; the accessor contract applies to every representable
value without any constructor requirement.

The subsequent native validator experiment imported the owning-crate
declaration and accessor specification. `native-accessor-consumer.log` no
longer reports either `VpIndex` error, but retains seven error/type/library
translation failures. Its annotation-only validator overlay was removed.
The coverage body proof and input-iterator correspondence remain absent;
the accessor theorem is not coverage, selector filtering, or payload
preservation.

The continuation followed the real standard-library implementation rather
than treating an undeclared method as proof of impossibility.
`native-core-source.log` records the current Rust 1.98.1 sysroot
`core/src/mem/mod.rs:953-970`: `mem::replace` directly invokes
`intrinsics::read_via_copy` and `intrinsics::write_via_move`.
`probe-intrinsics.log` records their declarations at
`core/src/intrinsics/mod.rs:2214,2225`: compiler intrinsics with no Rust body.
The reference-argument probes first exposed an implicit raw-pointer coercion
rejection. The smaller raw-pointer probe eliminates that coercion and still
rejects both intrinsic calls (`probe-raw-intrinsics.log`). No unchecked
intrinsic is invoked at runtime by these compilation-only diagnostics.

Native source annotation therefore does not by itself discharge this
operation: after annotating `replace`, there are no Rust bodies to prove
for its compiler-intrinsic leaves. The existing verifier has neither
operation's interface. Providing such semantics would require new trusted
intrinsic declarations or a verifier change; changing the executed operation
would instead cross the frozen executable boundary. None was done. This is
stronger evidence than the original missing-method diagnostic and does not
claim all other project proof work is exhausted.

The read-only tooling investigation also found Verus's specialized native
core modes (`--is-core` and `--is-stdlib-outside-of-core`). These are not
ordinary Cargo dependency opt-ins and do not themselves provide intrinsic
semantics or import verified sysroot operation bodies into this workspace.
The current accessor proof needs neither mode nor any change to the pinned
verifier.

Remaining library/type failures on the real validator are `RestoreError`,
`mem::replace`, slice iterator `position`, `anyhow::Error`, `Error::msg`,
`__private::must_use`, and `__private::format_err`. The real `position`
implementation is separately recorded in `native-core-source.log`; it calls
`next`, a predicate, and `assert_unchecked` while counting. Its native
representation/operation proof is still owed, not assumed.

The current source retains only the native scalar View/accessor proof and
owning-crate metadata/dependency changes. The validator and selector bodies,
TOP contracts, sanctioned declarations, manifests, and both freeze packages
remain unchanged. This is limited representation-proof progress, **not**
completion of the assigned validator proof. No temporary marker was added
or removed.

The changed-source handoff run passed `cargo check --locked -p vm_topology
-p vmm_core --tests` and the agent-profile nextest selection for all
`vm_topology` tests plus `restore_vp_index_tests`. Logs are
`continuation-rust.log` and `continuation-tests.log`.
Required checks ran once for this source:

| Command | Result |
| --- | --- |
| `make_verify --crate-root .` | Exit 0; 0 verified / 0 errors; 8.268 s |
| `boundary --crate-root . --baseline-dir .verus_agent check` | Exit 1; existing provenance rejection and 7 temporary locations; 1.006 s |
| `spec_drift --crate-root . --baseline-dir .verus_agent` | Exit 0; 1.180 s |
| `exec_drift --crate-root . --baseline-dir .verus_agent` | Exit 0; 4.871 s |

The durable task `vp-index-native-proof-checks` therefore exits 1 solely for
the existing boundary failure. Its receipt, status and complete check outputs
are preserved under `restore-vp-index-coverage/continuation-*`. The restored
TOP selector remains scaffolded and its zero-error admission run is not
evidence of coverage. Existing four bridges and dependency-declaration debt
remain as previously recorded.

## Named-closure candidate: unresolved bindings before experimentation

This follow-on task proposes, but does not install, a source normalization:
each selector closure receives a named shared-reference parameter and
destructures that reference locally. Both closures must be exercised in the
real production selector, not inferred admitted from the retained standalone
integer control. `VpIndex` and its `index` method currently have no native
Verus annotations; declaration and iterator-contract failures after syntax
translation are possible and must be reported separately.

For arbitrary payload `T`, both original parameters match `&(VpIndex, T)`.
The proposed local `let (vp_index, _) = entry` must retain the same borrowed
`&VpIndex`, never move or clone `T`, and leave the owning `into_iter` and
`collect` unchanged. The map must still be consumed by full-capacity
validation before filtering. The filter must retain the exact existing
`instantiated_vp_count as u32` comparison, including narrowing behavior for
large counts; no new count restriction is justified by this normalization.
Validation error order, payload drop behavior, and selection order remain
part of semantic preservation, not consequences of frontend admission.

No selected-prefix, validator-coverage, or payload-preservation contract is
assumed. These remain body-proof obligations, as do the caller's count
invariants, decoding, RPC installation, time adjustment and final TOP link.
The existing restore-guard-native-presence request and both authoritative
branches are outside the candidate's writable surface. Current source
observations, not the earlier mission's summary, determine this experiment.

## Named-closure request: current diagnostic result

The bounded package is submitted and unapplied at
`research/freeze_requests/restore-vp-selector-named-closures/`, containing
exactly `freeze.patch`, `run.patch`, and `rationale.md`. The one `submit`
invocation reported both patches applicable to the current branch tips,
matching frozen specifications and executable projections, and
`freeze_request: VALID` (`restore-vp-prefix-proof/freeze-submit.log`).
The request is only a two-closure normalization plus verifier entry
annotations; it adds no contract, assumption, trust marker, or rlimit.
This is a diagnostic/package delivery, not an installed selector proof.

The actual generic production body was checked in an isolated source
archive, not replaced by a model or a cfg-selected body. Both named
parameters and local destructuring statements clear the original
tuple-parameter frontend rejection. The next actual result is four
declaration errors: `VpIndex`, `RestoreError`, the unannotated
`validate_restore_vp_indices`, and `VpIndex::index` in the filter at candidate
line 931. No selection VC was reached. The suggested external declarations
were not adopted. The exact commands and complete diagnostics are in
`check-prepared-candidate.sh` and `candidate-admission.log`; the focused run
exited 101 after approximately 31 seconds.

Ordinary Rust checking of that exact candidate,
`cargo check --locked -p vmm_core --tests` with Rust 1.95.0, exited 0 in
11.71 seconds (`candidate-rust-prepared.log`). It emitted an unused
proof-prelude import warning and dependency cfg warnings. The initial
archive lacked its configured packaged protoc; the first two setup failures
are preserved, and the prepared check linked the already-installed package
directory without installing tools or changing source.

The separate `candidate-behavior.patch` tests the real function with a
non-Clone owning payload. It checks allocation identity, unsorted retained
order, rejection/drop order, dormant-entry errors, count above capacity, and
the existing 64-bit-to-u32 narrowing. Six of seven tests initially passed;
the error test incorrectly expected `RestoreError::Display` to include its
source. Production's `InvalidSavedState` deliberately displays only "saved
state is invalid". Correcting that test to inspect the typed inner error
made the focused remaining test pass. Both logs are retained as
`candidate-behavior.log` and `candidate-error-behavior.log`; the selector and
request patches did not change during this test correction. These tests
support, but do not replace, the generic reference-binding equivalence
argument in the rationale.

After removing the test-only overlay, `finish-candidate.sh` established that
the tested source exactly matched the submitted `run.patch`, all three
package files matched their validated inputs, both authoritative branch tips
were unchanged, and the authoritative selector was byte-identical to HEAD.
Only the isolated archive was removed. The existing
restore-guard-native-presence request, manifests, sanctioned declarations
and verifier were not modified.

Current authoritative checks ran once after isolated work completed:

| Check | Result |
| --- | --- |
| `make_verify --crate-root .` | Exit 0, 0 verified / 0 errors, 2.503 s |
| `spec_drift --crate-root . --baseline-dir .verus_agent` | Exit 0, no frozen specification drift, 1.183 s |
| `exec_drift --crate-root . --baseline-dir .verus_agent` | Exit 0, no executable drift, 4.894 s |
| `boundary --crate-root . --baseline-dir .verus_agent check` | Exit 1, 7 existing temporary locations / 0 assumptions, 1.000 s; TCB provenance rejected |

Receipts are `current-*.log`; complete wrapper output remains under
`.verus_agent/cache/checks/*/latest.log`. The unchanged declaration markers
on `InitializedVm` and `LoadedVmInner`, TOP body marker, and four uninterpreted
representation bridges remain debt. The rejected provenance still says the
TCB manifest was last modified by an autonomous agent commit. Worker-scoped
boundary scanning and patch drift checks are not proofs or whole-program
trust inventories. The absent maintained callgraph was re-probed; source
caller observations are not a generated graph or caller proofs.

There is no installed proof of selection, validator coverage, count
invariants, decoding, RPC installation, time adjustment, or the final TOP
connection. No temporary marker was added, removed, or moved. At initial
delivery, `delivery-readiness.log` reported the separate Human-presentation
gate: the three package files had to be committed and the working tree clean.
The initial no-commit instruction was honored and the gate was not bypassed.

The continuation authorizes committing the exact request and its in-scope
evidence, without applying either patch. `commit-delivery.sh` limits staging
to this research evidence and request. No Cargo package is changed, so
package-scoped clippy and doc have no modified-package targets.
`precommit-delivery.sh` runs the mandatory full formatter in an isolated
archive, preserving frozen production source and checking that the submitted
artifacts are unchanged. The initial formatter required only blank lines
after the shebangs in two evidence Python scripts; those header fixes do not
change the request or experimental commands. The first formatter log and
the failed strict artifact-identity check are retained separately from the
final run. `validate-committed-delivery.sh` then checks the
same request against the resulting branch tips, committed-byte identity,
clean worktree, unchanged frozen branch and selector, and the authoritative
Human-presentation gate. Its post-commit output belongs in session artifacts
rather than a new uncommitted evidence file. This is delivery validation,
not authorization to apply the request or a new selector proof.

## Binding obligations before the selection proof

The bounded target is the real `select_instantiated_vp_states` body in
`vmm_core/src/partition_unit/vp_set.rs`, not installation or complete restore.
On success, when the instantiated count fits `u32` and does not exceed
capacity, selection must retain exactly one unchanged input `(VpIndex, T)`
pair for each instantiated index, and no other pair. This is a conditional
postcondition, not a new precondition on currently unproved callers.

The validator's `present` bitmap, checked lookup, duplicate rejection, and
missing-entry rejection support exact input-index coverage on success.
If its body is temporarily admitted, that exact-coverage interface remains
unproved project-code debt; it is not sanctioned TCB. The iterator's relation
to the actual input pairs must also be established, not assumed.

`VpSet::restore` passes `self.vp_capacity` and `self.vps.len()`, then uses
each selected index to look up the destination and sends that pair's payload.
The capacity/count representation invariant, decoded partition inventory,
RPC payload installation, partition guard, stopped state, time adjustment,
and final frozen TOP equality remain separate unproved obligations.
Coverage inside a validated payload says nothing about paths that omit that
payload. The four TOP representation bridges and TOP `external_body` are
unchanged debt.

The call-structure tool was invoked at intake but its maintained snapshot
`.verus_agent/proof_state.json` is absent. Caller observations are therefore
direct production-source evidence, not a generated graph or a caller proof.
The frozen manifests are present and parsed; their scanner roots cover
`openvmm/openvmm_core/src/worker`, not the entire production dependency graph.
The existing restore-guard-native-presence package is outside this experiment.

## Selector admission diagnostic

No selection guarantee was proved. The unchanged selector has no Verus
declaration, so the first function-focused check reports that the containing
`partition_unit::vp_set` module is not available for selection. Adding only
`verus_verify` and its required macro import to the real function exposes:

```text
only variables are supported here, not general patterns
states.iter().map(|(vp_index, _)| *vp_index)
```

This is a frontend rejection of the closure parameter, before validation,
iterator, payload, or selection VCs. `production-admission.patch` records the
two annotation-only additions; they were removed from production afterward.
`native-body.log` records the real-function rejection. The initial missing
`verus_spec` import was corrected before that decisive run.

`research/restore-vp-prefix-proof/probe-closures.sh` reproduces the same error
using only a borrowed pair of integers. Both the bare tuple-pattern closure
and the closure with an explicit result specification are rejected. A named
closure parameter followed by an ordinary local destructuring statement
verifies its exact projection (`1 verified, 0 errors`). This is only a syntax
control, not a substitute selector or a selection theorem. The existing
known-proof-pattern catalog concerns mutable-reference identity and does not
apply to this translation error.

The pinned frontend directly calls `pat_to_mut_var` on closure parameters
(`rust_to_vir_expr.rs:4092-4107`); that helper accepts only
`PatKind::Binding` (`rust_to_vir_expr.rs:294-310`). Specification attributes
do not turn a tuple parameter into a variable. The control's normalization
would change frozen executable source, so it was not applied to production.
No verifier semantics, trust declaration, contract, or existing freeze
package was changed. No new freeze package was submitted.

The standalone control initially exposed a separate environment problem:
the default Z3 was 4.12.5 while this Verus expects 4.16.0. Selecting the
already installed `toolchain/verus-src/source/z3` resolved that problem;
the repro script names it explicitly. The negative frontend probes do not
reach Z3. The annotated standalone probes enable `proc_macro_hygiene`
locally, avoiding the unrelated expression-attribute feature gate; no
production feature gate was changed.

The desired conditional contract and validator coverage proof remain
unimplemented. No selector or validator postcondition was assumed, and no
temporary proof marker was added. This diagnostic does not satisfy the
bounded proof acceptance criterion; the next task needs an authorized path
past native closure-pattern admission before body verification can proceed.

## Restored integration and boundary evidence

After removing the two experimental annotations, `vp_set.rs` is byte-equal
to its intake version (`HEAD`). The four required wrapper commands were run
once against that restored production state:

| Check | Result |
| --- | --- |
| `make_verify --crate-root .` | Exit 0, 0 verified / 0 errors, 2.476 s |
| `spec_drift --crate-root . --baseline-dir .verus_agent` | Exit 0, no frozen specification drift, 1.222 s |
| `exec_drift --crate-root . --baseline-dir .verus_agent` | Exit 0, no executable drift, 4.924 s |
| `boundary --crate-root . --baseline-dir .verus_agent check` | Exit 1, 7 temporary locations, 0 assumptions, 1.049 s; also rejects TCB-manifest provenance |

The make wrapper still selects the scaffolded TOP restore function; its
zero-body result preserves integration but proves neither selection nor
restore. The boundary report additionally says
`tcb_manifest.json was last modified by an autonomous agent commit`.
That is a harness authority/setup issue, not permission to rewrite the
Human-frozen manifest; JSON parsing alone did not establish accepted
provenance. The manifest and sanctioned declarations were left untouched.
The reported temporary locations are the existing TOP `external_body`,
`InitializedVm`/`LoadedVmInner` declaration debt, and four representation
bridges. Its scoped scan is not a whole-program trust inventory.

# Restore TSC interpretation

## Unresolved binding facts recorded before the experiment

Scope: the TSC of one selected, saved x86 VP, not the complete restore theorem.
The frozen specification and sanctioned declarations are not edited by this
diagnostic. At intake, the manifest sanctions `InitializedVm::load` and the
two `ExRestoreReadyFile` markers. This diagnostic did not apply or modify any
frozen-boundary package.

- `dispatch.spec.rs::restore_vp_projection` copies the selected saved VP View
  unchanged. `restore_snapshot_projection` separately adjusts virtual time.
  `VpStateView.value` is an integer, not yet a defined encoding of registers.
- `dispatch.proof.rs::decoded_restore_request_view` receives both the saved
  state and the optional `(downtime, TSC frequency, APIC frequency)` policy.
  Its uninterpreted result is not currently bound to raw serialized TSC.
  Whether its saved VP image may mean the policy-adjusted restore image,
  rather than the policy-independent wire image, is the binding question.
- The alternative candidate is an ordinary live VP View that retains the
  actual TSC, paired with a request View that interprets saved TSC under the
  supplied restore policy. Its arithmetic is source-supported but unproved:
  on success, `effective_tsc = serialized_tsc + floor(downtime_ns * hz / 1e9)`;
  without adjustment, `effective_tsc = serialized_tsc`. Production uses checked
  multiplication, narrowing, and addition. Error cases are not successful
  restored states, and must not become new preconditions.
- This interpretation must not erase TSC, equate all VP states, assume the
  theorem, or silently make the load wrapper use a different request image.
  The concrete backend getter/setter/commit relationship, serialization,
  selection, and stopped-state invariant remain proof obligations.
- A raw-wire request View and an actual-counter VM View would disagree after
  a successful nonzero adjustment. That would refute that pair of intermediate
  definitions, not the current theorem with four uninterpreted bridges.
- The source path is `InitializedVm::load` -> `restore_snapshot_state` ->
  `PartitionUnit::advance_tsc` -> `PartitionRequest::AdvanceTsc` handler ->
  `VpSet::advance_tsc` -> `StateEvent::AdvanceTsc` handler ->
  `BoundVp::advance_tsc`. `BoundVp` invokes the production `AccessVpState` TSC
  getter, setter and commit. The generated `VpSavedState` schema includes TSC
  as field 11; `restore_all` restores it. The callgraph command was attempted
  and cannot read the absent `.verus_agent/proof_state.json`; these are source
  call/handler observations, not a generated call graph or verified contracts.

## Experiment boundary

Exercise the production VP save/restore and stopped-VP RPC path with a
deterministic register backend, once without an adjustment and once with
nonzero downtime. The backend is test instrumentation, not a trusted backend
specification. Do not replace any production restore or adjustment method.
Use the real generated VP serialization schema. Observe restored TSC, committed
TSC, and reserialized TSC. Compare both raw-wire and policy-adjusted request
interpretations. No conclusion about successful whole-VM restore or all
hardware backends follows from this bounded experiment alone.

## Observed production-path result

`research/restore-tsc-consistency/production_path.rs` is test instrumentation
loaded temporarily into `vp_set.rs`; no production method is copied or replaced.
The source register backend first saves through production `save_all` and
`SavedStateBlob::new`, retaining every register present under its capabilities,
not constructing a TSC-only restore fixture. The request interpretation is
computed from that saved blob **before** the destination restore begins.
It then sends VP 0 through `VpSet::restore`, the real stopped `VpRunner`, the
`StateEvent::Restore` handler, `BoundVp::restore`, and the generated
`AccessVpState::restore_all`. Save/readback traverses `VpSet::save`, the real
save handler, `save_all`, and the real codec. With adjustment, it also traverses
`VpSet::advance_tsc`, the real `StateEvent::AdvanceTsc` handler,
`BoundVp::advance_tsc`, `advance_tsc_state`, setter, commit, and getter.
It supplies `Some(APIC frequency)`, like the x86 TOP; the APIC timer is disabled
and TSC-deadline capability is absent in this bounded fixture.

The backend stores TSC/APIC and models all other registers at reset. Their
setters assert that production restores exactly those reset values. Guest
execution and unexpected non-reset register writes panic in test code so
that unintended paths do not pass silently. It has no restore algorithm:
its restore adapter invokes production `restore_all`. The TOP's frequency
checks, partition RPC, backend-clock
adjustment, stop guard, and whole-VM success are source evidence, not executed
by this unit probe. The TOP and partition RPC call/handler source identifies
the tested lower path as the production one.

| Policy | Wire TSC | After production restore | Committed TSC sequence | Final reserialized TSC | Policy-indexed request TSC |
| --- | ---: | ---: | --- | ---: | ---: |
| None | 1000 | 1000 | 1000 | 1000 | 1000 |
| None | 1001 | 1001 | 1001 | 1001 | 1001 |
| 250 ms, 4003 Hz | 1000 | 1000 | 1000, 2000 | 2000 | 2000 |
| 250 ms, 4003 Hz | 1001 | 1001 | 1001, 2001 | 2001 | 2001 |

Both nextest tests passed, covering these four production-path executions.
The fractional 1000.75-cycle request exercises truncation as well as nonzero
adjustment. Both raw-wire/request inequality and **complete generated
`VpSavedState` equality** are asserted: before adjustment against the original
saved image, afterward against the precomputed policy image. The latter is
constructed by saving an independent backend with the desired TSC and the
same reset registers. No final state is fed back into the request image.
The two distinct saved counters remain distinct afterward; equality is not
obtained by dropping TSC or comparing only a common constant. Full output:
`research/restore-tsc-consistency/policy-image.log`. The earlier
`production-path.log` is retained as historical output, not current evidence.

Reproduce from the repository root:

```bash
bash research/restore-tsc-consistency/run.sh
```

The runner adds only a test module include, invokes Rust 1.95.0 and the existing
nextest agent profile, and removes the include on exit, including test failure.
The retained source is the instrumentation and its four-line test-only overlay;
the actual production source is restored after the diagnostic.
The strengthened probe compiled in 1.58 s and its two tests completed in
0.006 s on this run. These are bounded experiment timings, not whole-VM
restore or verification performance claims.

## Semantic decision

For this selected-and-saved TSC coordinate with an exact stopped-counter
backend, the frozen selector is compatible with a **policy-indexed requested
VP image and an actual-state VM image**. This construction has now been
executed against production save/restore/adjustment, not just proposed.
Refining real hardware backends remains open; the evidence does not justify
a freeze request or a claim that the whole TOP is proved.

Let `s` be decoded TSC, `d` downtime in nanoseconds, and `f` the supplied TSC
frequency. Define the desired counter for successful adjustment as
`s + floor(d*f/1_000_000_000)`; for no adjustment it is `s`. The executable
candidate `requested_tsc` retains this natural-number value without wrapping.
It evaluates cycles as `seconds*f + floor(subsecond_ns*f/1_000_000_000)`.
For every Rust `Duration`, `duration < 2^64` seconds, so with `f,s <= 2^64-1`
the final value is at most `2^128-1`; this coordinate fits `u128` even when
production's intermediate multiplication or final `u64` addition fails.
This is a total arithmetic interpretation, not a replacement restore method.
In Verus it can use `nat`, embedded in the integer-valued VP image. A desired
counter above `u64::MAX` need not have a successful live-state realization:
production still returns its existing overflow errors, and the TOP's Err arm
promises no state equality. No precondition is added to exclude such requests.
The bound and agreement with successful checked arithmetic remain proof
obligations; the production-path experiment covers the table, not all inputs.

Use the same faithful encoding of VP state on each side. For the bounded TSC
coordinate it distinguishes every `u64` counter. In a full implementation it
must encode the remaining guest-relevant VP state as well, not replace the VP
with a scalar TSC or discard other registers. The request encodes the *desired
restored* counter; the VM encodes the actual stopped counter. For selected,
saved VP 0, the frozen `restore_vp_projection` returns exactly that request
entry, so 1000 equals 1000 in the control and 2000 equals 2000 after adjustment.
Different final counters remain different Views. No subtraction of downtime
from the VM View, collapsed constant View, or hidden ghost history is needed.

The request image is policy-indexed, not a bare policy-independent `SavedState`
View. Keep the raw decoder separate from interpretation of the requested
restore. Two requests containing identical wire bytes but different downtime
may have different requested VP images; two identical live VP states have the
same actual-state View regardless of history. This captures an observable
restore policy rather than reclassifying the TSC as host-operational state.
Conversely, **raw-wire request + actual-counter VM** is invalidated by the
production adjustment trace. Since the current bridges are uninterpreted,
that rejected intermediate construction is not a counterexample to the frozen
theorem.

### Compatibility with the load wrapper

For the selected VP 0, let `I(s,p)` be the desired TSC coordinate implemented
by `requested_tsc` from decoded snapshot `s` and restore policy `p`. The
proposed bridge relationship is:

```text
decoded_load_restore_request_view(Some(s), p, selection)
    .saved_state.vp_states[0].TSC
  = I(s,p)
  = decoded_restore_request_view(s, p, 1)
      .saved_state.vp_states[0].TSC
```

Here `.TSC` is explanatory projection of a future faithful VP encoding, not
an existing field or a newly assumed Verus equality. Both bridges must use
the same complete encoding and the same interpretation, rather than making
the wrapper retain raw TSC while the helper sees adjusted TSC.
The source supports this relationship: `InitializedVm::load` computes the
instantiated count from `restore_vp_count.unwrap_or(vp_capacity)` and truncates
the binders (dispatch.rs:1590-1600), then passes the very same saved state and
`restore_time` to the helper (3328-3330). For the one-VP experiment, both
`Some(1)` and `None` at capacity 1 select VP 0. The frozen helper success
predicate and wrapper success predicate both invoke `snapshot_restore_result`;
their selected-count arguments agree in this case. That function copies
`I(s,p)` unchanged through `restore_vp_projection`, exactly the equality
exercised by the probe.

This does not execute `InitializedVm::load` or prove its sanctioned contract.
It establishes no global rule for recovering destination capacity from the
wrapper bridge's inputs, and no general selected-count agreement. Those
remain binding obligations. Neither the initialized-VM nor live-VM View may
depend on the supplied restore policy: in the fixture the destination starts
at 77, while the requested image is derived only from saved state and policy.
The `None`-snapshot boot path has no snapshot success obligation in the
existing wrapper contract.

### Exact future bridge use and still-open obligations

- `decoded_restore_request_view`: decode the selected saved VP's TSC through
  the real component schema, then interpret its desired VP image with the
  supplied `restore_time`. Its `saved_state.virtual_time` must remain raw and
  have zero elapsed downtime: the frozen virtual-time projection already
  applies the policy once. Prove the checked arithmetic and all successful
  return paths of `BoundVp::advance_tsc`; tests are not that proof.
- `loaded_vm_representation`: expose guest-observable VP state, including the
  actual TSC at the logical observation epoch. Establish that production
  restore installed the decoded TSC, that it is the value read for adjustment,
  and that set/commit/readback and subsequent backend-clock operations preserve
  the desired counter at that epoch. The runner's stopped state and object
  ownership must connect the RPC response to this View. These facts are not
  presently contracted or proved.
- Backend refinement is substantive, not assumed. `Tsc::is_present` is always
  true, but `Tsc::can_compare` depends on `caps.can_freeze_time`. WHP sets that
  capability true and its TSC access delegates to register get/set; KVM sets
  it false and accesses TSC through MSRs. Therefore the deterministic backend
  demonstrates consistency of the production algorithm with an exact
  stopped-counter interface, **not** that every hardware read remains equal
  to the last programmed value. KVM clock/epoch behavior and the TOP's later
  `advance_snapshot_time` still need refinement evidence; one must not assume
  literal equality of arbitrary wall-clock-separated KVM reads or exclude
  KVM from the existing precondition. No KVM hardware behavior is claimed from
  the test.
- `decoded_load_restore_request_view`: on `Some(saved_state)`, use the same
  policy-indexed VP interpretation as the helper request. `InitializedVm::load`
  passes the identical `restore_time` to the helper at dispatch.rs:3329.
  Prove agreement of the wrapper's VP-selection policy and the helper's
  instantiated count. `initialized_vm_representation` remains the actual
  destination/default image; it must not acquire restore-policy-dependent TSC.
  The sanctioned wrapper contract is unchanged.
- No conclusion is made for unsaved or unselected VP frames, APIC/deadline
  state, other components, or final readiness. The production advance operation
  visits all instantiated VPs; those other frame obligations must not be
  inferred from this selected-and-saved VP diagnostic.

This is a source-supported representation direction with executed
implementation evidence, not a completed body proof or a new trusted
assumption. No `requires`, `ensures`, View definition, marker, or rlimit
annotation was changed.

## Required checks on restored production source

All four commands used `${ARGUS_SKILL_PYTHON}` from the repository root, once
after removing the test overlay. Logs are in
`.verus_agent/cache/checks/{make_verify,boundary,spec_drift,exec_drift}/latest.log`.

| Command module (`python -m argus_verus.tools.checks.<module>`) | Result |
| --- | --- |
| `make_verify --crate-root .` | Exit 2, 2.320 s; no verification-result counts emitted. Admission fails on 13 existing unsupported/undeclared production types or traits, beginning with `HvlitePartition` at dispatch.rs:548 and `SavedState` at dispatch.proof.rs:29. |
| `boundary --crate-root . --baseline-dir .verus_agent check` | Exit 1, 1.039 s; zero permanent marker violations, five retained temporary locations, zero assumptions. Separately rejects TCB provenance: `tcb_manifest.json was last modified by an autonomous agent commit`. This checker/setup issue was not bypassed or repaired here. |
| `spec_drift --crate-root . --baseline-dir .verus_agent` | Exit 0, 1.213 s; frozen specifications match. |
| `exec_drift --crate-root . --baseline-dir .verus_agent` | Exit 0, 4.852 s; 20 executable files checked, no body edits or unknown functions. |

The five unchanged debts are the TOP's `external_body` (whole successful
restore postcondition), the helper request bridge (decoded component images
and policy), the load request bridge (optional input and selected count), the
initialized-VM bridge (prepared/default destination image), and the loaded-VM
bridge (actual restored component image and lifecycle). Their four
`uninterp` definitions remain in `dispatch.proof.rs`; the TOP marker remains
in `dispatch.rs`. The experiment adds/removes **no proof-debt marker** and
discharges none of those complete facts.

The replay shell passed syntax checking, the overlay passed `git apply --check`,
and the retained Rust instrumentation passed rustfmt checking. A scoped
`git diff --exit-code` confirms all four writable production/proof files have
no diagnostic edits. The strengthened two-test/four-execution experiment was
validated once; the required checks were run once afterward. No assumptions,
trust declarations, contract edits, or rlimit annotations were introduced.
The call-structure command was also retried once for this resumed diagnostic:
the maintained `.verus_agent/proof_state.json` is still absent, so no generated
callgraph evidence is claimed. Source calls/handlers are cited above.
The wiki was read but not edited: it is outside the mission's authoritative
writable paths.

Handoff classification: **diagnostic**. The chosen policy-indexed TSC
interpretation has a production-path witness and preserves counter
distinctions; the rejected raw-wire interpretation has a concrete mismatch.
There is no demonstrated frozen contradiction and no freeze request.
Full production proof/progress handoff remains unavailable because verifier
admission fails independently of this result. Backend observation refinement
and the separate TCB-provenance checker issue remain explicit.

## Unsaved VP frame: omitted partition payload

**Decision (diagnostic):** completeness of a *present* partition payload does
not exclude omission of that payload. With the real state-unit inventory still
containing `"partition"`, production accepts the omission, skips partition/VP
restore, and can successfully adjust the instantiated VP's existing TSC.
This invalidates extending the accepted policy-indexed saved-VP interpretation
to the preservation branch by claiming that every adjusted VP must have a
saved entry. It is not a machine-executed counterexample to the whole TOP:
the four bridges are still uninterpreted, and the fixture does not construct
`LoadedVm` or run a hardware hypervisor.

The accepted selected-and-saved diagnostic above is reused unchanged, not
re-reviewed or rerun. Its register backend is included by the new fixture;
its two tests are excluded by the new nextest selector.

### Reproduction and executed boundary

```bash
bash research/restore-unsaved-vp-frame/run.sh
bash research/restore-unsaved-vp-frame/check-spec.sh
```

`production_frame.rs` constructs a real `PartitionUnit` with one instantiated
VP and registers it in real `StateUnits`. It runs the production partition
runner and VP runner, reusing the accepted deterministic register backend.
The partition backend is test instrumentation: its save/restore methods only
encode a fixed register and count restore calls. It supplies neither a restore
algorithm nor a trusted contract.

The fixture first saves through `StateUnits::save`, `PartitionUnitRunner::save`,
`VpSet::save`, and the generated VP schema. It retains the exact inventory,
then either removes the entire partition payload or clears the VP vector
*inside* it. The small test-only `partition_payload.rs` helper decodes and
re-encodes the actual production `state::Partition`; no replacement partition
schema or restore body is used.

It invokes production inventory validation and state-unit restore, then, only
after success, the same state-unit time advance, partition TSC RPC, and stop
guard operation called by the TOP. The real partition request handler,
`VpSet::advance_tsc`, VP event handler, `BoundVp::advance_tsc`, checked
arithmetic, register setters, commit, and save/serialization path all execute.
The fixture checks both serialized counters and the exact commit sequence.

| Payload | Downtime / TSC frequency | Initial TSC | Final serialized TSC | VP commits | Result |
| --- | --- | ---: | ---: | --- | --- |
| Partition omitted | None | 77 | 77 | none | Restore and stop guard succeed |
| Partition omitted | None | 78 | 78 | none | Restore and stop guard succeed |
| Partition omitted | 250 ms / 4003 Hz | 77 | 1077 | 1077 | Restore, adjustment, and stop guard succeed |
| Partition omitted | 250 ms / 4003 Hz | 78 | 1078 | 1078 | Restore, adjustment, and stop guard succeed |
| Partition present, VP vector empty | Requested, not reached | 77 | 77 | none | Restore rejects missing VP 0 |

All rows pass exact, nonempty inventory validation. In the omitted rows the
partition backend restore counter remains zero. In the present-but-incomplete
row it becomes one: partition restore precedes VP inventory validation, but no
VP restore or time adjustment follows the error. No rollback claim is made.
The TOP's Err arm permits this partial mutation.

The final run compiled in 1.79 s; three tests covering five executions passed
in 0.009 s. Complete output is `restore-unsaved-vp-frame/production-frame.log`.
The fixture leaves APIC timers disabled and TSC-deadline capability absent,
as in the accepted backend. This is not a measurement of whole-VM restore.
The test-only includes are removed on exit; production and proof files match
their intake contents.

### Connection to the real TOP and its caller

These are source observations, not new callee contracts or assumptions:

1. `openvmm_defs/src/worker.rs:44-52` declares `SavedState.units` and
   `SavedState.inventory` as separate vectors. Inventory includes units with
   no mutable payload; equality of inventories is not equality of payload
   domains. The snapshot frontend at `openvmm_entry/src/lib.rs:4630-4646`
   checks the saved inventory against manifest names, not presence of a
   partition payload. Artifact integrity does not impose that missing check.
2. `VmWorker::new` at `dispatch.rs:422-434` parses the saved message and
   passes it to `InitializedVm::load`. Load instantiates the selected VP
   prefix, registers `"partition"` (`3185-3206`), and forwards the same
   `saved_state` and `restore_time` to the TOP (`3328-3330`). There is no
   intervening payload-completeness predicate. Normal production save emits
   partition state, but provenance from that producer is neither a TOP
   precondition nor an enforced property of this decoding path.
3. The TOP calls `LoadedVm::restore` (`3839-3842`), whose entire body at
   `4743-4749` validates inventory when nonempty and calls
   `StateUnits::restore`. This is precisely the pair exercised by the probe.
   `StateUnits::restore` (`state_unit/src/lib.rs:981-1042`) rejects unknown
   or duplicate supplied names, not absent payloads. `run_op` passes `None`
   for an omitted name, and `state_change` returns `Ok(None)` without sending
   a restore request (`1245-1254`).
4. For a present partition payload, the real path is
   `PartitionUnitRunner::restore` (`partition_unit.rs:678-683,732-742`) ->
   `VpSet::restore` (`vp_set.rs:1182-1191`) ->
   `select_instantiated_vp_states` -> `validate_restore_vp_indices`
   (`888-925`). It requires exactly one saved entry for **every capacity VP**,
   including dormant ones, before selecting the instantiated prefix.
   This path is not entered at all for omission.
5. After state-unit restore succeeds, the TOP's `Some(restore_time)` branch
   advances state-unit time and calls `PartitionUnit::advance_tsc`
   (`dispatch.rs:3845-3868`). The request and handler
   (`partition_unit.rs:310-324,442-448`) do not test whether partition restore
   ran. `VpSet::advance_tsc` visits every instantiated VP, not a saved-entry
   set (`vp_set.rs:1221-1247`). Thus the tested mutation is reached by the real
   TOP control flow, not an arbitrary lower-level call disconnected from it.

The TOP's other gates must also succeed for `Ok(())`: matching destination
frequencies, backend operations, other units' restore/time advance, backend
snapshot-clock advance, and final stop guard. They were **not** all executed
by this fixture. None of the inspected gates enforces partition-payload
presence. A source-supported success continuation exists on x86 WHP:
frequency equality succeeds by choosing its reported frequencies
(`virt_whp/src/lib.rs:618-644`), and backend `advance_snapshot_time` uses
the default `Ok(())` (`virt/src/generic.rs:417-419`), with no prior-restore
flag. The TOP then acquires its stop guard and returns `Ok(())`
(`dispatch.rs:3872-3883`). KVM's clock advance likewise reads/adds/writes the
current clock without inspecting payload presence
(`virt_kvm/src/arch/x86_64/mod.rs:668-678`), but backend success and observation
refinement remain unproved and were not hardware-tested.

Accordingly, the production condition that rules out missing VP entries
*inside a present partition payload* is identified. No such enforcing path
was found for entire-payload omission. Successful whole-TOP reachability here
is a source-level execution argument conditional on ordinary successful
backend/device operations, not an executed whole-TOP result or a discharged
representation theorem. Proving the former completeness property would
require connecting the real state-unit dispatch, successful partition restore,
full-inventory validator, and all error propagations; it cannot be promoted
to an unconditional successful-TOP invariant.

The prescribed callgraph command was attempted once. It cannot read the
missing `.verus_agent/proof_state.json`; generated callgraph evidence is
unavailable. No graph was invented or rebuilt around this setup gap.

### Frozen precondition and representation consequence

`frozen_precondition.rs` includes the actual, unchanged `dispatch.spec.rs`.
With capacity/active/selected counts all one, initial VP 0 present, empty saved
VP map, matching nonempty component inventories, zero saved elapsed time,
and nonzero downtime, its proof establishes:

```text
request.valid_for_loaded_vm(initial)
restore_snapshot_projection(...).vp_states[0].value == 77
restore_snapshot_projection(...).vp_states[0].value != 1077
```

Pinned Verus with pinned Z3 reports **1 verified, 0 errors** in
`restore-unsaved-vp-frame/frozen-precondition.log`. The `value` constants
illustrate a TSC-distinguishing encoding; this witness does not define the
four production bridges, identify its abstract VM with the test backend, or
assert that the complete VP View consists only of TSC. It proves directly
that the frozen predicate does not demand saved VP coverage and that its
preservation branch cannot produce a changed image. The same issue holds
for any faithful full-state encoding that distinguishes the changed counter.

For an absent saved VP, policy-indexing the *saved* images has nothing to
transform. The frozen projection must return the initial image, whereas the
real adjustment reads the current TSC and adds the positive cycle delta.
Nonzero downtime alone need not give a positive delta; the witness uses a
delta of exactly 1000, so truncation or overflow cannot explain it away.
There is no faithful AI-owned definition satisfying both observations while
retaining TSC and the saved-domain meaning. Erasing TSC, shifting the live
View by restore history, fabricating saved entries, or falsifying compatibility
to reject this otherwise admitted input would conceal the obligation.
Even fabricated entries cannot generally recover the desired destination
counter: `decoded_restore_request_view` receives saved state, policy, and
selected count, **not initial VM state**. The two omitted-input executions
have identical request data but require distinct final counters.

This is concrete evidence for a frozen-boundary decision, not authority to
change it. One narrow behavior-change candidate is to reject an absent
`"partition"` payload on the TOP's time-adjusted restore path before
adjustment; that leaves the current success contract intact and uses its
unrestricted Err arm. Such rejection would change accepted runtime behavior
and must not be smuggled into a bridge or local invariant. It has not been
implemented or submitted: the current writable scope does not authorize
changing the TOP body or creating the submission tool's
`research/freeze_requests/...` output. There is no `freeze_request: VALID`
claim. Reviewer has the production witness and exact missing guard for a
separately authorized request; backend refinement is still required before
calling this a formal counterexample to the currently uninterpreted TOP.

### Current checks, retained obligations, and diagnostic cleanup

All four required wrappers were run once with the authoritative Argus Python
after the final fixture run removed its overlays:

| Check | Current result |
| --- | --- |
| `make_verify --crate-root .` | Exit 2, 2.304 s; no verification counts. Thirteen existing admission errors, beginning with undeclared `HvlitePartition` at `dispatch.rs:548` and unsupported `SavedState` at `dispatch.proof.rs:29`. |
| `boundary --crate-root . --baseline-dir .verus_agent check` | Exit 1, 1.065 s; zero permanent violations, five temporary locations, zero assumptions; also rejects the existing TCB-manifest autonomous-commit provenance. |
| `spec_drift --crate-root . --baseline-dir .verus_agent` | Exit 0, 1.237 s; frozen specifications unchanged. |
| `exec_drift --crate-root . --baseline-dir .verus_agent` | Exit 0, 4.867 s; 20 executable files, no changed bodies or unknown functions. |

Full logs remain in the prescribed `.verus_agent/cache/checks/*/latest.log`
locations. The manifests were read but not changed. Verifier admission and
TCB provenance are separate, pre-existing setup/proof blockers; this bounded
diagnostic neither repairs nor scaffolds them. Native proof/progress handoff
remains unavailable.

No production contract, invariant, trust declaration, or rlimit annotation was
changed. No proof-debt marker was added, removed, or moved. The retained TOP
`external_body` still owes its entire successful outcome. The four
`uninterp` bridges still owe, respectively: decoded request/policy and saved
domains; load/helper request and selection agreement; initialized/default
state; and actual live state/lifecycle including backend observation. The
new witness specifically prevents discharging their unsaved-VP frame
obligation by the refuted universal saved-coverage claim.

Fixture development exposed an ambiguous test-macro import, then a teardown
wait caused by retaining `VpRunner` after its run future completed; the test
now explicitly drops the runner so production teardown can finish. Error
inspection was corrected to display the full source chain. The initial
standalone Verus invocation found ambient Z3 4.12.5 instead of required
4.16.0; `check-spec.sh` selects the existing pinned solver rather than
disabling its version check. These were test/setup corrections, not production
changes or contradictory semantic results. Final tests and the standalone
proof passed once; unchanged successful checks were not repeated.

The wiki was read but remains outside writable scope. The reproducible probe,
its logs, and this appended decision are the only retained changes for this
diagnostic. All experimental production-source includes were removed.

## Proposed x86 time-adjusted partition-presence requirement

**Decision (diagnostic and freeze-request preparation, not a TOP proof):**
the candidate's real production guard rejects an omitted partition payload
before any partition restore or VP commit in the bounded fixture. Its call
is the first operation in the candidate TOP, before frequency setup or any
restore/time adjustment. The proposal remains unapplied to both authoritative
branches and production sources.

The maintained request command accepted the exact three-file package at
`research/freeze_requests/restore-time-partition-presence/`:
`freeze.patch`, `run.patch`, and `rationale.md`. Its complete output is
`research/restore-time-partition-presence-validation/submit.log`:

```text
freeze.patch: APPLIES
run.patch: APPLIES
spec_drift: PASS - both patch results have the same frozen TOP and BOTTOM specifications
exec_drift: PASS - both patch results have the same executable behavior
freeze_request: VALID
```

This is the observed native result, not an inference from the earlier
TCB-provenance failure. Pair validation succeeded without changing or
bypassing any checker. The separate authoritative `boundary` check still
rejects TCB provenance, as recorded below. A valid proposal is not a completed
proof or permission to apply it. No commit, approval, or branch advance was
performed. The maintained Human-review readiness path requires a committed
package and a clean working tree; this mission forbids commits and hands the
uncommitted package to Reviewer, not directly to Human.

### Scope and preserved behavior

The proposed guard is x86-guest-only and tests
`restore_time.is_some()` together with absence of a `"partition"` entry in
`SavedState.units`. It does not substitute inventory presence for payload
presence, parse a new schema, or alter generic sparse state-unit restore.
The private production helper permits an isolated call to the same guard
used by the real TOP; there is no copied or alternate restore body.

The intentional accepted-input change includes zero downtime and nonzero
downtime whose computed TSC delta is zero. These are explicit policy-level
rejections, not claims that their TSC necessarily changes. Without a time
policy, omission remains accepted by the original sparse restore path.
Present payloads and non-x86 guest behavior retain their original paths.
Errors that already occurred later may now occur earlier for inputs meeting
the new rejection predicate.

The frozen `Err(_) => true` arm permits this rejection without weakening
the success contract or adding a precondition. The accepted omitted-VP
diagnostic and frozen-precondition witness above were reused, not rerun.
The selected-and-saved policy-indexed interpretation is likewise reused:
it cannot create a saved entry where none exists, and its request bridge
does not receive the destination-dependent initial counter. Two identical
requests with destination TSCs 77 and 78 still require different adjusted
counters. No invented entry, erased counter, history-dependent live View,
or new trusted fact is used to hide that distinction.

### Isolated candidate execution

The durable runner launched `validate_candidate.py`, which uses a disposable
archive of the real project, applies `run.patch`, and adds only the separate
`instrumentation.patch` for testing. It checks the TOP's guard-call ordering
and uses the actual guard compiled in `openvmm_core`, then the real state-unit
restore, partition/VP runners, adjustment, save/codec and stop-guard path.
The accepted deterministic register backend is reused unchanged. The actual
production partition schema is accessed through test-only helpers; production
restore and adjustment methods are not copied or replaced.

Two nextest tests passed, including **17 partition/VP fixture executions**.
For each complete payload, the entire generated `VpSavedState` equals the
expected production-saved image. Exact TSC and commit observations are:

| Payload | Time policy | Initial TSC | Final TSC | VP commits | Outcome |
| --- | --- | --- | --- | --- | --- |
| Omitted | None | 77 or 78 | unchanged | none | Original restore and stop path succeeds |
| Omitted | 0 ns | 77 or 78 | unchanged | none | New guard rejects; partition restore count 0 |
| Omitted | 1 ns, 4003 Hz; delta 0 | 77 or 78 | unchanged | none | New guard rejects; partition restore count 0 |
| Omitted | 250 ms, 4003 Hz; delta 1000 | 77 or 78 | unchanged | none | New guard rejects; partition restore count 0 |
| Complete, saved TSC 1000 | None | 77 or 78 | 1000 | 1000 | Original restore and stop path succeeds |
| Complete, saved TSC 1000 | 0 ns or 1 ns | 77 or 78 | 1000 | 1000, 1000 | Original restore/adjust/stop path succeeds |
| Complete, saved TSC 1000 | 250 ms, 4003 Hz | 77 or 78 | 2000 | 1000, 2000 | Original restore/adjust/stop path succeeds |
| Present, VP vector empty | 250 ms, 4003 Hz | 77 | 77 | none | Guard passes; existing downstream missing-VP error |

The present-but-incomplete row has one partition restore call before the
existing `snapshot is missing state for vp0` error; no VP restore or
adjustment follows. There is no rollback claim. A second test makes six
guard calls covering empty/nonempty inventories, no payload, a differently
named payload, and the no-policy control.

Complete output: `restore-time-partition-presence-validation/candidate.log`.
The successful invocation took 5.455 s including nextest/build setup;
the reported build duration was 4.78 s and the two tests took 0.026 s.
These are bounded fixture timings, not whole-TOP or verifier performance.
The fixture does **not** construct `LoadedVm`, execute its frequency/backend
clock gates, run a hardware hypervisor, or prove any representation bridge.
The connection to the unexecuted TOP is its inspected real call and control
flow, not an assumed whole-VM execution.

The durable runner's initial executable preflight rejected an unexpanded
shell variable; a literal `bash` launcher corrected it. A malformed patch
hunk count was corrected before patch application, and the test fixture's
macro-import ambiguity was corrected before tests executed. Their separate
logs remain in the validation directory. No semantic construction was
refuted by those setup errors, and successful candidate tests and native
request validation were not repeated.

### Authoritative checks and retained obligations

All required wrappers ran once on unchanged authoritative sources using the
configured Argus Python. Full logs and byte-faithful command sidecars are in
`research/restore-time-partition-presence-validation/`; native logs also use
the prescribed `.verus_agent/cache/checks/<check>/latest.log` paths.

| Required command | Result |
| --- | --- |
| `make_verify --crate-root .` | Exit 2, 2.242 s; no verification counts emitted. Thirteen pre-existing frontend admission errors, starting with undeclared `HvlitePartition` at `dispatch.rs:548` and unsupported `SavedState` at `dispatch.proof.rs:29`. |
| `boundary --crate-root . --baseline-dir .verus_agent check` | Exit 1, 1.070 s; zero permanent marker violations, five retained temporary locations, zero assumptions. Still rejects `tcb_manifest.json was last modified by an autonomous agent commit`. |
| `spec_drift --crate-root . --baseline-dir .verus_agent` | Exit 0, 1.187 s; frozen TOP and BOTTOM specifications unchanged. |
| `exec_drift --crate-root . --baseline-dir .verus_agent` | Exit 0, 4.785 s; 20 executable files, no edited bodies or unknown functions. |

The maintained call-structure command was attempted once and still cannot
read the absent `.verus_agent/proof_state.json` (`intake.log`). Real caller
and callee connections were inspected in source; no generated call graph or
verified callee contract is claimed. No graph rebuild, verifier-admission
repair, backend refinement, or provenance-checker repair was bundled.

No TOP or BOTTOM declaration, manifest, proof contract, invariant, or rlimit
annotation changed. The proposal preserves all four `uninterp` bridges and
the TOP `external_body` as the same proof debt described above. The helper's
body, successful dispatch/coverage relationship, arithmetic, backend
observations, and complete TOP success/lifecycle relation remain proof
obligations, not sanctioned trust. No marker was added, removed, or moved.
Full verification and boundary failures prevent a bounded proof/progress
handoff; this is a validated proposal with a bounded runtime diagnostic.

The candidate runner compared authoritative TOP/spec/proof/manifests and
both branch refs before and after execution, and removed its disposable
source copy. Production sources stayed unchanged throughout. The wiki
remains outside this mission's writable scope.

## Native partition admission: unresolved binding facts before experimentation

- The production `HvlitePartition` declaration is in
  `openvmm/openvmm_core/src/partition.rs:57`. Its supertraits are `Inspect`,
  `Send`, `Sync`, and `RequestYield`; none may be omitted from an admission
  construction. Its methods and blanket implementation must retain their
  production signatures and behavior.
- The first construction will mark that exact trait for native Verus
  translation with `#[verus_verify]`, leaving its supertraits and all method
  declarations intact. Whether translation accepts the supertrait graph and
  method-signature types is unresolved. No method postcondition, implementation
  proof, representation bridge, or backend observation is established by
  admitting a declaration.
- `Inspect::inspect` takes the real `inspect::Request<'_>`
  (`support/inspect/src/lib.rs:2040`); `RequestYield::request_yield` takes
  `VpIndex` (`vmm_core/src/partition_unit/vp_set.rs:1932`). These project-owned
  declarations are potential next admission obligations, not sanctioned
  external leaves. The experiment must not substitute opaque or replacement
  definitions for them.
- The existing selector translates the real `worker::dispatch` module.
  `InitializedVm.partition` and `LoadedVmInner.partition` carry
  `Arc<dyn HvlitePartition>`; the TOP calls its frequency methods and
  `advance_snapshot_time` (dispatch.rs:3818-3874). Whether the native trait
  annotation removes that declaration barrier, or exposes the next one, is
  the bounded question. The TOP body remains scaffolded and is not proved by
  this experiment.
- The pinned verifier contains native `Arc<dyn T>` tests
  (`toolchain/verus-src/source/rust_verify_test/tests/traits_dyn.rs:57`).
  This supports trying native translation, not omitting the production
  supertraits or claiming all dynamic traits are supported.
- The maintained call-structure command was attempted in
  `research/native-partition-admission/intake.sh`; it exits 1 because
  `.verus_agent/proof_state.json` is absent. The above connections are inspected
  source, not generated graph evidence. The independent boundary check's
  existing TCB-provenance rejection is not a frontend-admission diagnostic.

### Observed native declaration rejection

**Diagnostic result, not an admission repair or TOP proof.** One construction
was executed: import `vstd::prelude::verus_verify` and attach
`#[verus_verify]` to the real production `HvlitePartition` trait. Every
supertrait, method signature, and implementation remained unchanged. There
was no replacement trait, external trait specification, opacity annotation,
assumed method behavior, or removal of the TOP's scaffold.

The existing `make verify MODULE=restore` selector exited 2 in 2.26 seconds,
before emitting verification counts. The pinned frontend reported:

```text
thread 'rustc' (...) panicked at vir/src/traits.rs:1615:9:
compute_dyn_compatibility: missing trait Path(inspect, ["Inspect"])
```

The complete, unedited output is
`research/native-partition-admission/verify.log`. `verify.command` records the
exact timed command; `run.command` records its production-overlay runner.
`trait-annotation.patch` is replayable; `attempted-source.patch` is the
source diff captured from the actual run. `run.log` records both the failure
and successful restoration. Reproduce from the repository root with:

```bash
bash research/native-partition-admission/run.sh
```

The runner applies only the two annotation/import lines, executes the existing
production selector once, reverses the patch even on verifier failure, and
returns the verifier command's status. Exit 2 is the observed negative result,
not a successful proof. No experimental Rust changes remain in production.

The panic has a specific source explanation. In the pinned
`toolchain/verus-src/source/vir/src/traits.rs`, `set_krate_dyn_compatibility`
(1721-1763) builds local and imported trait-compatibility maps.
`compute_dyn_compatibility` (1606-1648) recursively follows each actual
`Self` supertrait bound; line 1615 panics if the requested trait is absent.
The native annotation therefore reaches analysis of the real trait's
`inspect::Inspect` bound, but that project-owned supertrait has no available
Verus declaration in this run. This is distinct from both the baseline
undeclared-`HvlitePartition` error and a normal failed verification condition.
It is also distinct from a demonstrated incompatibility of an admitted
`Inspect` declaration: this experiment did not annotate or prove `Inspect`.

The rejected construction is **annotating only `HvlitePartition` while keeping
its actual, currently untranslated supertraits**. The next concrete admission
obligation is the real `Inspect::inspect(&self, Request<'_>)` declaration
and its reachable types; `RequestYield::request_yield(&self, VpIndex)` and the
remaining method-signature types remain untested admission obligations.
This does not prove that those declarations cannot be admitted, that native
dynamic dispatch is unsupported, or that permanent trust is necessary.
No attempt was made to hide or remove a supertrait to obtain a passing result.

The production connection is source-backed. `InitializedVm::load` invokes
`restore_snapshot_state` at dispatch.rs:3329 with the actual saved state and
time policy. The helper invokes the partition frequency/time methods on its
`Arc<dyn HvlitePartition>`. The real blanket implementation at
partition.rs:202-231 forwards those operations to `virt::Partition`;
neither these bodies nor their backend observations are proved by the
declaration experiment. The absent maintained call graph remains a setup
limitation; no graph or verified callee contract is claimed.

### Unchanged-source checks and remaining obligations

No integration change is retained, so the matching existing required-check
evidence was reused rather than rerun. `preserve-evidence.log` confirms no
staged or unstaged production/configuration/manifest changes after restoration
and records the check timestamps. Its initial history query also included
research Rust fixtures; `source-provenance.command` corrects that scope by
excluding `research/`. The last production/configuration change was at
11:18:32 on 2026-09-20, preceding the reused verification log at 12:26:56
and integrity logs at 12:49:35-39. The intake production diff was also empty.
The exact reused logs are preserved as
`research/native-partition-admission/reused-<check>.log`.

Each command below uses `"${ARGUS_SKILL_PYTHON:-python3}" -m
argus_verus.tools.checks.<check>`; **none was rerun in this diagnostic**.

| Required check and arguments | Reused result |
| --- | --- |
| `make_verify --crate-root .` | Exit 2, 2.242 s; 13 existing frontend admission errors, no verification counts. First error: undeclared `HvlitePartition` at dispatch.rs:548. |
| `boundary --crate-root . --baseline-dir .verus_agent check` | Exit 1, 1.051 s; existing TCB-provenance rejection: `tcb_manifest.json was last modified by an autonomous agent commit`. Five temporary marker locations remain. |
| `spec_drift --crate-root . --baseline-dir .verus_agent` | Exit 0, 1.215 s; frozen specifications match. |
| `exec_drift --crate-root . --baseline-dir .verus_agent` | Exit 0, 4.843 s; 20 executable files, no body edits or unknown functions. |

No temporary marker was added, removed, or moved. The retained TOP
`external_body` still defers its successful restore/pre-execution guarantee.
The four retained `uninterp` bridges in dispatch.proof.rs still defer the
decoded helper request, decoded load request, initialized-VM representation,
and loaded-VM representation; their connection to actual state and policy is
not established here. No contract, invariant, sanctioned BOTTOM declaration,
manifest, verification selector, or rlimit changed. The unapplied semantic
partition-presence freeze proposal and the other baseline admission errors
remain outstanding. The frontend panic must not be confused with the separate
boundary provenance rejection. This bounded result is ready for diagnostic
review, not a passing full-crate, proof, progress, or final-completion claim.

## Inspect cross-crate admission: unresolved binding facts before experimentation

- The actual `Inspect::inspect(&self, Request<'_>)` declaration is in
  `support/inspect/src/lib.rs`. `Request` contains `RequestParams` and
  `&mut InternalNode`; the latter's production feature-dependent representation
  includes a mesh oneshot receiver. Whether native translation accepts these
  real declarations, without opacity or a replacement interface, is unresolved.
- `inspect` does not currently opt into cargo-verus or depend on `vstd`.
  The pinned cargo-verus implementation selects opted-in dependencies using
  `package.metadata.verus.verify` and imports an artifact-adjacent `.vir`, not
  ordinary Rust metadata alone. The experiment must determine whether that
  path produces usable declarations before attributing failure to import.
- The workspace disables inspect's defaults and enables `derive`; production
  dependencies may unify additional features. The experiment must record the
  actual feature selection needed by `openvmm_core`, not infer it from inspect's
  standalone defaults. No feature may be disabled to hide a production field.
- The first bounded construction will annotate the actual `Inspect` trait and
  `Request` datatype in their owning crate and opt that crate into the native
  dependency build. Any further declaration annotation must follow the actual
  signature/field closure and verifier feedback. Method implementations remain
  unproved; declaration translation supplies no method behavior.
- If dependency translation succeeds, the unchanged restore selector, with
  only the real `HvlitePartition` declaration annotated, must demonstrate that
  its `Inspect` supertrait is available. A dependency-only run cannot establish
  that fact. A rejection before export is not an import failure or a proof of
  general impossibility.
- The TOP scaffold, four representation bridges, remaining frontend barriers,
  separate TCB-provenance rejection, and unapplied semantic freeze proposal
  remain outside this diagnostic's result. No trust addition, executable
  change, weakened interface, frozen-contract edit, or rlimit increase is
  authorized by successful declaration admission.

### Observed native dependency result

**Diagnostic, not an integrated admission repair or TOP proof.** The native
path was exercised with pinned Verus `0.2026.09.18.8ed93e5`. The real
`Inspect` trait and request representation were translated to intermediate
VIR, but the dependency failed declaration well-formedness before Cargo could
analyze `openvmm_core`. The precise remaining declaration is
`mesh_channel_core::oneshot::OneshotReceiver`, carried by
`InternalNode::Deferred` under the production `defer` feature.

The source-backed native path is:

- `cargo-verus/src/metadata.rs:9-31,105-123` reads
  `package.metadata.verus.verify` and plans imports for opted-in dependencies.
  `rust_verify/src/cargo_verus.rs:116-180` finds the `.vir` beside the
  dependency artifact selected by rustc. Ordinary `.rmeta` is insufficient.
- The native `rust_verify_test/tests/cargo-tests/verified/` example
  `basic_verified_lib` opts into verification and exports `double` and a
  trait; `basic_verified_lib_consumer_verified` opts in, depends on it, and
  calls `double`. Its two manifests and source files demonstrate the supported
  dependency construction, not the success of this production experiment.
- `rust_verify/src/verifier.rs:2805-2811` exports VIR before the declaration
  checks at `2837-2849,2879-2881`. `import_export.rs:35-77` serializes the
  supplied crate without first checking well-formedness. A `.vir` artifact can
  therefore exist after a failed dependency check. `--no-verify`, used for
  dependencies by `focus`, does not skip that check.

All those paths are relative to `toolchain/verus-src/source/`; the pinned
revision and executable were checked in `intake.log`. The production feature
graph is retained in `production-features.log`: inspect resolves
`defer,derive,inspect_derive,std`, without `initiate`. The focused command uses
`--no-default-features --features derive,defer,std`, which enables precisely
that set (`inspect_derive` follows from `derive`). The integration command
retains the original production selector and its normal dependency resolution.

All experiment files below are under `research/inspect-cross-crate-admission/`.
Every result-bearing command was recorded before execution in a `.command`
sidecar and/or its traced `.sh` script; complete stderr/stdout and actual
nonzero statuses are retained.

| Construction and selection | Observed result |
| --- | --- |
| `dependency-source.patch`: owning-crate opt-in and actual `Inspect`/`Request` annotations; `cargo verus focus -p inspect --no-default-features --features derive,defer,std` | Exit 101, 4.42 s. `RequestParams` and `InternalNode` are undeclared. See `dependency-verifier.log`. |
| `representation-source.patch`: native annotations for the local request datatype closure; same focused selection | Exit 101, 1.53 s. `InternalNode` contains undeclared `mesh_channel_core::oneshot::OneshotReceiver`. See `representation-verifier.log`. |
| `integration-source.patch`: preceding construction plus the actual `HvlitePartition` annotation; unchanged `make verify MODULE=restore` | Exit 2, 1.57 s. Cargo stops while checking `inspect` on the same receiver declaration, before `openvmm_core` analysis. See `integration-verifier.log`. |

No run emitted verification-result counts. Each verifier command was bounded
at 110 seconds and finished normally with its reported error, not a timeout.
No rlimit annotation or option was increased.

`dependency.vir` and `representation.vir` are the complete native textual
translation logs. The latter contains the real
`inspect::Inspect::inspect` method with its `inspect::Request` parameter
at lines 10012-10037, and the `InternalNode::Deferred` receiver field at
272-300. An artifact-adjacent binary `.vir` was also emitted, as recorded in
`dependency-artifacts.log` and `preserve-evidence.log`. These are
**pre-validation translation artifacts**, not valid imported declarations or
proved methods. Neither the artifact's existence nor the absence of the old
`missing Inspect` panic establishes consumer admission: the consumer was not
reached.

The diagnostic's source explanation is narrower than a representation
incompatibility claim. `vir/src/well_formed.rs:174-232` reports this error when
the datatype path is absent from its declaration map. The actual type is
defined at `support/mesh/mesh_channel_core/src/oneshot.rs:270-273`, with
`ManuallyDrop<OneshotReceiverCore>` and `PhantomData<Arc<Mutex<T>>>` fields.
It is reexported through `mesh_channel` and `mesh`; its owning crate does not
opt into cargo-verus. `Request`'s reference field and this conditional enum
variant are real production representation, not an invented model.
The compiler's suggested external type specification was **not** adopted.
The next bounded admission work would concern this actual dependency and its
representation closure. This experiment does not show that it cannot be
translated, that `Inspect` is intrinsically incompatible with Verus, or that
trust/opacity is necessary.

The two existing non-Copy derived-Clone warnings for `Value` and `ValueKind`
are retained in the logs, not suppressed or counted as proved Clone behavior.
No `Inspect` implementation, partition method implementation, or TOP body was
proved. No shared contract, precondition, postcondition, or invariant changed.

### Restoration, current evidence, and remaining obligations

All experimental production source, manifests, and lockfile changes were
restored. `preserve-evidence.log` records clean staged and unstaged
production/configuration/manifest diffs, applicability of all three exact
overlays, and shell syntax checking. `reproduce.sh` applies the integration
overlay to clean production files, executes the unchanged selector, restores
the overlay even on failure, and returns the verifier's status. Reproduce with
`bash research/inspect-cross-crate-admission/reproduce.sh`; exit 2 with the
receiver declaration diagnostic is the observed result. The packaged runner
was syntax-checked, not used to repeat the unchanged integration run.

The intake script's comparison against older copied boundary evidence exited
1 because the current boundary log was newer, not because source restoration
or a new verifier check failed. The current logs were inspected and copied
as `reused-<check>.log`; their timestamps follow the last production change
at 11:18:32 on 2026-09-20. No production changes remain, so these matching
required-check results are reused rather than rerun. Commands use
`"${ARGUS_SKILL_PYTHON:-python3}" -m argus_verus.tools.checks.<check>`.

| Required command arguments | Reused current result |
| --- | --- |
| `make_verify --crate-root .` | Exit 2, 2.242 s; 13 existing frontend admission errors, no verification counts; first is undeclared `HvlitePartition`. |
| `boundary --crate-root . --baseline-dir .verus_agent check` | Exit 1, 1.009 s; separate existing rejection: `tcb_manifest.json was last modified by an autonomous agent commit`; five temporary locations remain. |
| `spec_drift --crate-root . --baseline-dir .verus_agent` | Exit 0, 1.170 s; frozen specifications match. |
| `exec_drift --crate-root . --baseline-dir .verus_agent` | Exit 0, 4.781 s; 20 executable files, no body edits or unknown functions. |

The call-structure command was reprobed and still exits 1 because
`.verus_agent/proof_state.json` is absent. Direct source shows the load caller
at dispatch.rs:3329 and the helper's partition calls at 3819-3874; this is not
generated call-graph or verified-callee evidence.

No temporary marker was added, removed, or moved. The retained TOP
`external_body` at dispatch.rs:3785 still defers
`snapshot_restore_success` and `pre_execution_representation` for successful
real restore. Its production body performs restore, time adjustment, and stop
guard acquisition; their contracts and concrete state refinements remain
unproved. The four `uninterp` bridges in dispatch.proof.rs remain:
`decoded_restore_request_view` (saved state/time policy/selected count for the
helper), `decoded_load_restore_request_view` (optional snapshot/time policy/VP
selection for the load wrapper), `initialized_vm_representation` (prepared VM
state), and `loaded_vm_representation` (live VM state). The real saved inputs,
load policy, and VM fields support their intended domains, not a proved
encoding. Component Views and body/refinement proofs must replace them.

The unapplied partition-presence semantic freeze proposal at
`research/freeze_requests/restore-time-partition-presence/` remains unapplied;
this diagnostic does not resolve the recorded omitted-payload counterexample
for the candidate state interpretation. Remaining baseline frontend barriers,
the `RequestYield` supertrait, method/body semantics, and the separate
TCB-provenance rejection are outstanding. No passing full-crate, bounded
proof/progress handoff, or objective-completion claim follows from this result.

## Restore declaration activation: unresolved bindings before experimentation

The current mission is declaration admission, not the restore body proof.
The current source includes the authorized partition-presence check; the
historical statement above that its proposal is unapplied is not current.
Both frozen manifests retain the load/File TCB and the dispatch TOP.

The proposed cut keeps `LoadedVm` transparent so the existing
`pre_execution_representation` can read its lifecycle fields. It temporarily
marks only project-owned carried representations opaque in their actual
owning crates, with native cargo-verus dependency imports. No method
postcondition, bridge equality, or restore-success assumption is introduced.
Whether these declarations and their automatically included associated types
are sufficient is unproved until owner and consumer validation complete.

Each proposed `external_body` datatype has this explicit representation debt:

| Declaration | Source evidence and consumer | Deferred representation and removal obligation |
| --- | --- | --- |
| `InitializedVm` | dispatch.rs:548; sanctioned load signature and existing initialized View | Prepared partition, VP binders, memory/topology/resources; component Views and their real initialization relationships. |
| `LoadedVmInner` | dispatch.rs:856; transparent `LoadedVm.inner` | Runtime partition, devices, memory, topology, deferred state; component Views and every restore/adjustment operation used by the real TOP. |
| `SavedState` | openvmm_defs/src/worker.rs:46; both decoded request bridges | Serialized units and inventory; real codec/refinement and policy-indexed interpretation, not arbitrary bridge equalities. |
| `StateUnits` | state_unit/src/lib.rs:250; `LoadedVm.state_units` | Unit map, names, dependencies, running state and RPCs; actual restore/advance-time semantics and ordering. |
| `StopGuard` | partition_unit.rs:639 and its Drop; both LoadedVm guard fields | Stop-token sender and resume-on-drop protocol; actual stop acquisition/lifetime semantics. |
| `mpsc::Sender<T>` | mesh_channel_core/src/mpsc.rs:76; LoadedVm notification fields and sanctioned load parameter | SenderCore, shared queue and Send/Sync phantom; ownership, message transport and operations remain unproved. |
| `mpsc::Receiver<T>` | mesh_channel_core/src/mpsc.rs:336; LoadedVm boundary receiver | ReceiverCore, shared queue and Send/Sync phantom; ownership, message transport and stream operations remain unproved. |
| `Rpc<I,R>` | mesh_channel/src/rpc.rs:30; LoadedVm transaction completion | Input and oneshot response sender; actual RPC completion/ownership and transport semantics. |
| `MicrovmSnapshotBoundaryRequest` | chipset_resources/src/lib.rs:168; LoadedVm boundary receiver | Scratch policy, oneshot release/completion and timeout; actual device/worker boundary protocol. |

These are temporary project proof obligations, not sanctioned BOTTOM leaves.
The three simple carried representations (`Instant`, `Timestamp`, scratch
policy) can instead be translated transparently from their scalar fields.
Opaque generics will reject recursive type arguments conservatively; no
recursive representation or positivity fact is assumed.

The proposed opaque `HaltReason` was refuted by its derived non-Copy `Clone`:
the generated body matches variants. Its replacement keeps the real enum and
hardware breakpoint/scalar enums transparent. Derived Copy/Clone also visits
the carried register image's segment/table types; those types and the complete
register image can all be scalar-transparent instead. No generated operation
is excluded.
The initially annotated `TimestampOutOfRange` was unnecessary: its generated
Display method introduced an unsupported formatter call; removing that
annotation leaves the actual timestamp declaration valid.

The original TOP `external_body` and all four uninterpreted bridges remain
separate debt. The TOP still owes the real body on every relevant outcome;
the bridges still owe concrete saved-request, load-policy, initialized-VM and
live-VM definitions. This mission does not remove them or use opacity to
claim those obligations proved. The prior transparent function-pointer
rejection is accepted as existing evidence and will not be rerun.

## Partition-presence proof: unresolved bindings before experimentation

The accepted activation Rust check is recorded in `research/PIPELINE_STATE.json`
from its completed durable receipt and stdout/stderr; it is not rerun.
Activation's final integration log establishes declaration admission only
(zero verified); its pinned configuration is Verus 0.2026.09.18.8ed93e5
and source Z3 4.16.0. The current environment does not set `VERUS_Z3_PATH`,
so experiments must explicitly select that solver.

The selected, initially unproved intermediate fact is:
`validate_snapshot_restore_partition_presence(saved_state, restore_time)`
returns `Ok` iff `restore_time` is absent or some actual
`saved_state.units[i].name` is `"partition"`. No input restriction is needed.
The unchanged production body at dispatch.rs:3784-3794 directly supports
this hypothesis: its only rejection is the `anyhow::ensure!` over the
option and the name search, followed by `Ok(())`. Neither a function result
nor an uninterpreted request bridge may define presence.

Source establishes the TOP call at dispatch.rs:3832 (using `?`), before
frequency checks, `self.restore(saved_state)`, state-unit time advancement,
VP TSC advancement, backend clock advancement, and stop acquisition.
The load caller is dispatch.rs:3332. The prior call-structure command failed
because `.verus_agent/proof_state.json` was absent; intake confirms that
absence. This is a tool setup defect, not call-graph evidence.

`SavedState` is currently opaque. Its real `units: Vec<SavedStateUnit>` and
`inventory: Vec<String>` (openvmm_defs/src/worker.rs:48-54) must become
available without assuming a relationship. `SavedStateUnit` has the real
`name: String` and `state: SavedStateBlob` (state_unit/src/lib.rs:398-405).
The blob is a protobuf wrapper, not evidence of successful partition decoding.
Declaration refinements needed to expose names must preserve those fields
and leave payload semantics unproved. The pinned vstd has an `Iterator::any`
contract with both true/false outcomes; whether the concrete iterator,
closure, string comparison and error macro translate and establish the
desired equivalence is a body-proof obligation, not an admission result.

Partition presence only excludes time adjustment with an omitted partition
payload. It does not establish decoding, completeness of saved VP entries,
installed VP state, or the link from adjustment to `restore_vp_projection`'s
preservation of unsaved entries (dispatch.spec.rs:147-162). The TOP
`external_body`, four uninterpreted bridges, and activation's remaining
representation/operation obligations remain unproved. In particular no
equality between presence and decoded VP coverage is assumed.

The native guard experiment reached a new dependency-interface blocker, not
a failed SMT obligation. Transparent native declarations for `SavedState`,
`SavedStateUnit`, `SavedStateBlob`, `ProtobufAny` and `ProtobufMessage`, plus
vmcore's cargo-verus opt-in, allowed the real fields to appear in a
no-precondition equivalence contract. A method-only `verus_spec` first failed
because its generated return-type constraint treated this receiverless
associated function as free. Giving the existing guard its own
`#[verus_verify] impl LoadedVm` fixed that syntax issue without changing its
body. The subsequent focused run rejected `anyhow::__private::not`, called
by the unchanged `anyhow::ensure!`, before verification conditions.

`research/restore-partition-presence-proof/guard-impl-context.log` records
that production failure (2.27 s); `experimental-source.log` preserves the
attempted overlay. `anyhow-not-probe.rs` isolates the exact dependency call
with only a boolean parameter/result and reproduces the rejection (0.49 s).
`anyhow-ensure-probe.rs` independently reproduces it through the real macro
(0.49 s); without the project's compatibility declarations that standalone
probe also rejects `anyhow::Error` and `anyhow::__private::format_err`.
Those additional standalone errors are not new production blockers.

The resolved production dependency is anyhow 1.0.99. Its
`src/ensure.rs:919-924` fallback calls `__private::not`; `src/lib.rs:710-732`
implements this through a private `Bool` trait, with ordinary negation for
bool and &bool. Source supports its meaning but supplies no verified native
import or sanctioned new contract. The project's existing
`openvmm_core/src/verus_compat.rs` declares the error type and formatting
operations, not `not`. The pinned vstd's `Iterator::any` specification exists;
this result does not reject iterator support, establish the closure's
postcondition, or show that boolean negation itself is unsupported.
The known reference-identity proof pattern does not apply to this
pre-VC external-call rejection. No suggested `assume_specification`, error
type declaration, or temporary assumption was added to bypass it.

All experimental production annotations, the attempted contract, and vmcore
Cargo/lock changes were restored. `restored-source-integrity.log` compares
the entire relevant production overlay byte-for-byte with activation's
`final-source.log`, checks vmcore is unchanged, and checks both manifests,
the frozen spec/proof files and selector against the frozen branch.
The only retained result is diagnostic; no guard guarantee is exposed or
claimed proved. No guard marker was present at intake or removed by a proof.
The `SavedState` opacity temporarily removed in the experiment is restored;
all fourteen previously recorded temporary representation/body/bridge
locations remain. Seven dependency locations are outside the worker scanner.
The unchanged `verus_compat.rs` external error declaration and four
`assume_specification` declarations are also outside that scanner and are
not entries in the manifest's three-item list; this diagnostic neither
changes them nor treats the scanner's zero assumptions as a whole-program
trust inventory.

Final required checks, each run once after restoration: `make_verify`
exit 0 (0 verified, 0 errors, function-selected admission; 10.656 s),
`spec_drift` exit 0 (1.201 s), `exec_drift` exit 0 (5.034 s), and `boundary`
exit 1 (1.025 s; seven in-scope temporary locations and the existing
`tcb_manifest.json was last modified by an autonomous agent commit`
rejection). Full outputs and exact command sidecars are retained in the
same research directory. These are not all-functions/full-crate timing
measurements or a bounded proof/progress handoff.

## Partition-presence continuation: native dependency proof hypothesis

Reviewer requires the actual guard equivalence, not another admission result.
The missing `anyhow::__private::not` interface is source-supported: its body
delegates to private `Bool::not`, whose bool and &bool implementations compute
negation. The next construction annotates the exact pinned dependency sources
in a local production Cargo patch, preserving both implementations and the
unchanged guard macro invocation. It does not supply an assumed external
specification, verification-only replacement, or copied guard.

Before experimentation, the obligations are to prove the generic helper from
a Bool contract, prove both real Bool implementations, import those native
contracts on the production dependency edge, and prove the guard iff from
actual saved names and restore-time option without new requirements.
Any local dependency source must match the registry source modulo erasing
proof annotations; runtime-only support and all licenses must be retained.
This construction is a hypothesis until owner/consumer and executable
integrity checks succeed. Frozen TOP, BOTTOM, manifests and verifier semantics
remain unchanged. The earlier possible macro-expansion direction is not
being applied or used as a proof.

The native helper construction proved the original generic helper and both
private Bool implementations (three verified, zero errors,
`native-anyhow-owner-proof.log`, 1.47 s). The initial package-only focus
found no opted-in workspace crate; adding workspace membership still reused
an ordinary Rust artifact with no VIR. A scoped `cargo clean -p anyhow`
made the owner actually enter verification. Its generated mutable temporaries
then encountered anyhow's `deny(unused_mut)`; a verification-only lint allowance
fixed that without changing any operation. `native-dependency-integrity.log`
checks all 53 other imported files byte-for-byte and checks lib.rs against
the registry source after removing only enumerated ghost annotations.
No new external declaration, operation assumption or temporary marker was
introduced for negation. This addresses the earlier blocker constructively.

The unchanged real guard then reached VCs (`guard-native-anyhow.log`), but
its cross-type `String == &str` comparison had no content-equality contract
in pinned vstd. An explicit closure contract failed on that operation.
`string-equality-probe.rs` gives an independent minimal comparison:
the production String/&str spelling fails while `name.as_str() == "partition"`
proves the same postcondition (one verified, one error; 0.38 s).
The matching sysroot `alloc/src/string.rs:2688-2717` defines the former
comparison through `PartialEq::eq(&self[..], &other[..])`; vstd/string.rs
specifies as_str and str equality but only String/String equality.
This is a missing external primitive interface, not a boolean-negation
limitation or evidence of a runtime bug.

The requested next step is packaged as the single unapplied
`research/freeze_requests/restore-guard-native-presence/` request.
It normalizes only the guard source: evaluate the same short-circuit
condition into a local binding, compare the same full name slice using
as_str, and give the closure explicit types and a block body. This neither
changes success/error behavior nor expands trust. The source boundary
requires approval even though the runtime behavior is preserved.
The accompanying run patch carries native anyhow annotations and the real
saved-field declaration closure. Its guard has no precondition and ensures
Ok iff restore_time is None or some actual units[i].name equals "partition".
No predicate is defined from the result or from an uninterpreted bridge.

The proposal's actual production function verifies with one verified and
zero errors (`proposal-native-syntax-check.log`, 17.49 s including isolated
worktree/build overhead). A proof instantiation of Seq::as_ref transports the
iterator's false-case contract to the actual indexed unit names; no assumption
is used. The earlier absence branch failed without this instantiation.
The declaration-only TOP selector remains separate integration evidence.

There were two package-construction corrections, not new semantic premises.
The first generated new-file patch headers were invalid. The first
body-verified version then failed the executable-drift parser's treatment
of closure attributes. Native Verus closure syntax plus matching explicit
executable types/block shape resolves that parsing mismatch.
`submit-native-syntax-request.log` reports both independent patches apply,
spec_drift PASS, exec_drift PASS and `freeze_request: VALID` (7.53 s).
No manifest, TOP/BOTTOM contract, verifier or actual frozen source was changed.

All experimental working-tree source and the imported dependency copy were
restored; `restore-native-source-integrity.log` checks exact equality to
activation. The frozen guard is therefore still unproved in the working
tree, rather than being silently replaced by the proposed source.
Current checks after restoration: make_verify exit 0, zero verified/zero
errors (11.129 s); spec_drift exit 0 (1.257 s); exec_drift exit 0 (5.104 s);
boundary exit 1 (1.067 s), with its unchanged seven in-scope temporary
locations and provenance rejection. These are function-selected admission
and focused measurements, not all-functions/full-crate timings.

The final submitted run patch also passes the unchanged production restore
selector (`proposal-integration-full.log`: zero verified, zero errors;
make_verify 16.091 s) and ordinary Rust 1.95.0 compilation with
`cargo check -p openvmm_core --locked` (10.87 s in
`proposal-integration-check.log`). These were checks of the proposed source
in an isolated worktree, not reruns of the completed activation Rust check.
The durable integration receipt records exit 0 and 28.7 s total.
The proposed guard theorem is established for that source, but remains
unapplied pending the frozen-source decision. All original decoding,
coverage, adjustment and TOP proof obligations remain.

## Freeze-request submission readiness

The continuation explicitly requests committing the request and preserved
supporting work while keeping the frozen change unapplied. Delayed durable
notifications were reconciled with their receipts and stdout/stderr in
`reconcile-durable-results.log`; the two earlier failed constructions are
already superseded by the recorded body/reference and native-syntax proofs,
so they were not rerun.

Mandatory pre-commit commands completed under Rust 1.95.0:
`cargo clippy --all-targets` for the eleven modified packages (14.40 s),
`cargo doc --no-deps` for the same packages (7.66 s), followed by the complete
`cargo xtask fmt --fix` (38.38 s). All commands exited zero; the formatting
log also records completion of rustfmt, lints, verify-fuzzers and verify-flowey
without a hidden pass error. Evidence is in `submission-{clippy,doc,fmt}.log`.
The existing production overlay is preserved; formatting changes only the
line layout of the guard's existing expression.

Before retargeting, the validated run patch's complete source result was
captured with the authoritative independent-patch helper. `run.patch` is
now incremental over the preserved source being committed, rather than
reapplying that existing declaration overlay. `freeze.patch` is byte-identical,
and the intended candidate source result is unchanged. Post-commit validation
must use the resulting branch tips; its evidence and clean-tree result are
recorded in the shared CHECKPOINT rather than creating an uncommitted log.
