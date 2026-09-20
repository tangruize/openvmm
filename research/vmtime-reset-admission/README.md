# Narrow Error/source declaration candidate

**Diagnostic, not reset admission or a body proof.** The new isolated
comparison prepares a request for only `core::error::Error`, its existing
Debug/Display supertraits, and the unconstrained `source` method signature.
No active production source or frozen manifest was edited.

`package-error.py --output <fresh-directory-under-this-directory>` extracts
the current run tip, generates independent frozen/run patches, and compares
the actual generated RpcError implementation. The final control adds only
transparent `RpcError` opt-in; the candidate adds the Error interface.
Both derives and all executable bodies are retained byte-for-byte after
erasing that annotation and ghost-only include.

| Evidence | Result | Scoped conclusion |
| --- | --- | --- |
| `error-final/baseline.log` | Exit 101 | Real derive reports undeclared Error and RecvError. |
| `error-final/candidate.log` | Exit 101, 1.517 s | Undeclared Error/source obstruction removed; generated body still reports unsupported thiserror `as_dyn_error` and undeclared RecvError. |
| `error-production-candidate.log` | Exit 101, 1.567 s | A trait header alone additionally leaves Error::source unsupported. This is why the proposed interface includes the real method signature. |
| `error-interface-probe.log` | Exit 1, 0.365 s | Supporting probe reaches a separate Debug/Display dynamic-supertrait rejection in native trait-conflict checking. It is not a passing interface test. |

All native comparisons enable lifetime and trait-conflict checks, use rlimit
50 and one verification thread, and preserve the generated implementations.
No additional thiserror, task, or project-type external declaration is
proposed. The newly exposed helper call is a separate external interface
obligation; the proposed Error declaration alone does not solve it.
The minimal probe's later checker rejection remains unresolved and was not
hidden by disabling a checker or changing Error's supertraits.

The final comparison adds **no temporary marker**. Earlier exploratory runs
used the existing transparent-RPC overlay's five external_body cuts:
`RpcSend::{call,call_failable}`, sender `send_rpc`, `PendingRpc`, and
`PendingFailableRpc`. They defer the real constructor/send/wrapper bodies
and receiver representations, without success postconditions; they remain
only in the earlier isolated workspace, not the final run patch or active
source. Existing Rpc/channel opacity is unchanged. Supporting probes reuse
the existing opaque DerivedError type only. No assume, admit, uninterpreted
function, new requires/ensures, or termination waiver was introduced.

The accepted `dab043bb0bad` review and the unchanged active-source check
evidence below are reused. All 30 current scoped source, manifest, and
verification-script inputs predate the completed checks; no active source
edit occurred in this mission. The existing full wrapper remains a 0/0
partial scaffold check, boundary fails provenance/temporary debt, spec_drift
passes, and exec_drift fails the previously identified generic-implementation
pairing. These are not newly passing gates; no checker maintenance or
unchanged validation rerun was performed.

The one submission produced exactly `freeze.patch`, `run.patch`, and
`rationale.md` under
`research/freeze_requests/restore-error-trait-admission`. `error-submit.log`
records **freeze_request: VALID**, both patches applicable, in 11.387 s.
This is mechanical validity, not semantic-check success or Human approval.
The submitted files are byte-identical to the measured/generated inputs;
neither patch has been applied to active inputs.

Submission's semantic checks remain advisory and did not pass:

- `spec_drift` cannot parse `cell.rs:272:26` (`UpdateMessage::<T>`) after the
  added source root exposes that file. The file is unchanged between current
  frozen/run tips. This newly encountered scope-expansion diagnostic is
  separate from the earlier active-scope spec_drift pass.
- `exec_drift` repeats the existing ProcessorTopology generic-implementation
  pairing mismatch; it also reports recovery from the unchanged cell parse
  error. No checker, topology implementation, or cell implementation was
  modified.

The supporting probe was also checked with the production command's explicit
`RUSTUP_TOOLCHAIN=1.95.0` environment to discriminate a toolchain-environment
explanation, not to repeat unchanged validation. It reports the same
Debug/Display trait-conflict rejection (`error-interface-pinned-probe.log`,
exit 1, 0.415 s). The selected pinned Verus launcher still reports its
1.98.1 compiler toolchain; no compiler replacement was made.

`error-rationale.md` records the exact proposed boundary and reproduction
command. Reviewer must independently validate the submitted package before
any Human approval. The proposal is only for review of the narrow declaration
benefit, not a proof/progress handoff or a promise of complete Error admission.

## Request transaction readiness

Independent Reviewer validation accepted the narrow diagnostic and reported
the package mechanically valid, but the Human-decision transaction is not
ready. `error-transaction-readiness.log` records a single direct
`operator_ready_issues` check: all three exact package files need committing,
and the repository-wide working tree must be clean. Their bytes still match
the measured/generated inputs. No unchanged verification or patch validation
was repeated, and neither patch was applied.

The Engineer's explicit no-commit rule and owned-path restriction conflict
with delivering that clean transaction while preserving pre-existing dirty
work outside the mission. This is an authority/transaction blocker, not a
new proof failure. It requires Reviewer/Planner to resolve ownership or route
finalization to an authorized owner. No unrelated change was committed,
hidden, removed, or relocated.

# Normalized reset: previous production admission diagnostic

**Diagnostic, not a body proof.** The approved constructor normalization is
now in the frozen and working tips. The previous waiting condition is cleared.
The real normalized `VmTimeKeeper::reset_to` still does **not** reach native
verification conditions with the tested keeper/RPC declaration interface.
All experimental production annotations were removed; both source files were
compared byte-for-byte with their intake contents, preserving dirty scalar
and stopped-state proof work. No executable or frozen specification changed.

## Current native evidence

`current-native.command` selects the actual root-workspace function, with
lifetime checking enabled, rlimit **50**, and one verification thread.
It does not copy the body, alter dependencies, or use the TOP wrapper's
`--no-lifetime` flag. The exact deltas are `keeper-declarations.patch`,
`rpc-interface.patch`, and the alternative `rpc-transparent-error.patch`.
Neither Cargo manifest needed an edit: both packages already opted into Verus.

| Measurement | Result | Meaning |
| --- | --- | --- |
| `current-declarations.log` | Exit 101, 1.567 s | The RPC dependency fails declaration validation: undeclared `core::error::Error`, opaque-error pattern access, and undeclared `RecvError`. The consumer is not reached. |
| `current-transparent-error.log` | Exit 101, 1.617 s | Making the real `RpcError` transparent eliminates the opacity error, but **not** the undeclared Error trait or `RecvError` errors. |
| `current-keeper-declarations.log` | Exit 101, 2.770 s | With the RPC overlay removed, the actual keeper body reaches declaration checking. Remaining RPC interfaces are missing, and newly exposed fields require `async_task::Task`, `pal_async::task::TaskMetadata`, and `inspect::Deferred`. |
| `derived-error.log` | Exit 0, 0.515 s; 0 verified / 0 errors | Empty automatically-derived Error impl control. This invalidates the overly broad hypothesis that merely mentioning that trait's impl header always fails. It proves no body. |
| `derived-error-source.log` | Exit 1, 0.465 s | Adding the `source` method reproduces `trait core::error::Error not declared to Verus` at its `Option<&dyn Error>` return type, with **no missing project datatype**. |

No production experiment reports body verification results. The first attempt
(`current-native.log`, exit 101, 1.617 s) placed the RpcError annotation before
`derive`; moving it after `derive` produced `current-declarations.log` with
the same three errors. Attribute ordering is not the repair.

### Precise obstruction and its limits

This is not the historical constructor-value rejection or a demonstrated
async/RPC language limitation. The admitted production error type pulls in
its generated `Error::source` implementation. Primary pinned frontend evidence:

- `toolchain/verus-src/source/rust_verify/src/external.rs:649-739` inherits
  the annotated datatype's opt-in for automatically-derived impls.
- `rust_verify/src/automatic_derive.rs:27-43` selects `VerifyAsIs` for an
  unrecognized automatically-derived trait; the special Debug handling does
  not extend to Error.
- `vir/src/well_formed.rs:100-125` rejects a `Dyn` signature whose trait is
  absent from the declaration map. The source-method probe reproduces exactly
  this case, without a channel, future, task, or closure.
- `rust_verify/src/rust_to_vir_adts.rs:64-95` translates every transparent
  field. `vir/src/well_formed.rs:325-337` rejects accesses to an opaque
  datatype. Making the keeper opaque cannot justify its actual `time` and
  `req_send` accesses; making `KeeperRequest` opaque cannot justify the
  actual `Reset(rpc)` constructor.

`RecvError`, `TaskMetadata`, and `Deferred` are project-owned declaration
work, but their owners are outside this mission's writable paths.
`pal_async::task::Task<T>` is an alias for external
`async_task::Task<T, TaskMetadata>` (`support/pal/pal_async/src/task.rs:25`).
The native diagnostic's suggested external-type declaration, and an external
declaration for `core::error::Error`, would add unsanctioned external
interfaces; they were **not** introduced. The frontend's
`external_auto_derives` escape hatch explicitly marks impls external
(`external.rs:696-721`); it is not a temporary body proof and was not used.
The known reborrow/reference-identity proof pattern does not fix declaration
well-formedness or an undeclared `dyn` signature.

Thus the tested interface is insufficient within the current ownership and
trust boundary. This does **not** establish that all other proof-owned
interfaces are impossible or that Human must approve a particular TCB
expansion. Wider declaration/proof work or a separately justified interface
design remains unresolved. No freeze request was submitted.

## Exact outstanding semantic obligations

Even after declaration admission, these facts are still owed:

1. **Stopped entry and local frame.** `TimeState::is_started` is already
   proved, but its observation does not prove the assertion at entry to
   `reset_to`. The real `restore`, `advance`, and `reset` callers must establish
   stopped state. The local assignment supports the intended stopped-value
   postcondition, with task, builder, and sender preserved; it has not been
   verified here.
2. **Request/response correspondence.** `RpcSend::call` constructs one
   `Rpc(input, result_send)`, invokes the supplied constructor once, sends it,
   and returns the matching receiver (`rpc.rs:164-173`). Its real body,
   generic closure callability, transport, and receiver relation still need
   contracts and proofs. The annotation-only signature cut supplies none
   of those semantic postconditions.
3. **No channel error for this particular lifecycle.**
   `PendingRpc::poll` maps receiver errors to `RpcError::Channel`
   (`rpc.rs:266-268`); a generic `call` cannot honestly promise `Ok`.
   The installed vstd `Result::unwrap` contract requires `result is Ok`
   (`vstd/std_specs/result.rs:177-185`). A future interface would need the
   conditional fact that this pending request, if awaited to completion,
   returns `Ok(())`, established from the primary task/response-channel
   lifecycle. Retaining `_task` and sending the request do not themselves
   prove that fact. Eventual scheduling/completion is a separate liveness
   obligation, not a consequence of a conditional future-result specification.

The source supports where to seek that conditional fact:
`VmTimeKeeper::new` retains the spawned primary task; the primary reset arm
(`vmtime.rs:723-734`) calls `Rpc::handle`, whose body sends the response only
after its handler returns (`rpc.rs:76-82`). Proving that this particular
handler remains stopped, completes normally, and does not lose its response
channel is still required. Child-RPC results are discarded by `join_all`;
no universal remote-clock installation fact is claimed.

### Temporary markers

The experimental opaque-error overlay adds six `external_body` locations:
`RpcSend::{call,call_failable}`, the sender implementation's `send_rpc`,
and `RpcError`, `PendingRpc`, `PendingFailableRpc`. The first three defer
the unchanged request-construction/wrapper/send bodies; the other three
defer the real error and receiver representations. Their supporting source
is the real RPC implementation, not an assumed successful response. The
transparent-error comparison removes only the RpcError opacity marker.
**All six were removed from production on restoration.** No `requires`,
`ensures`, invariant, `assume`, `admit`, or `uninterp` was introduced.

Both minimal probe files retain one diagnostic-only `DerivedError`
`external_body`, solely to match opaque datatype admission. They are not
runtime replacements or accepted declarations. The patch files describe
unapplied experimental debt, not a validated reusable interface.

Existing production `Rpc<I,R>` and channel Sender/Receiver opacity remain
unproved transport/representation debt. The worker boundary still reports
the `InitializedVm`/`LoadedVmInner` declaration cuts, the TOP `external_body`,
and the four uninterpreted representation bridges. None was discharged.
Remote installation, all-source lifecycle, Duration/saved-state bindings,
and the entire TOP body remain outside this result.

## Current checks and reproduction

The four required commands ran **once** on restored production, via
`check-current.py`; complete outputs are `current-*.complete.log`.

| Check | Recorded result |
| --- | --- |
| `make_verify --crate-root .` | Exit 0, 9.784 s; **0 verified, 0 errors (partial verification)**. Existing wrapper still selects scaffolded TOP with `--no-lifetime`; not a reset or full-crate body proof. |
| `boundary --crate-root . --baseline-dir .verus_agent check` | Exit 1, 1.167 s; existing TCB-manifest provenance rejection and seven worker temporary locations. |
| `spec_drift --crate-root . --baseline-dir .verus_agent` | Exit 0, 1.316 s; frozen specifications match. |
| `exec_drift --crate-root . --baseline-dir .verus_agent` | **Exit 1**, 4.923 s; checker pairs x86 and Aarch64 `ProcessorTopology::to_config` under the same name. This required gate did not pass. |

`current-topology-equality.log` independently compares the **entire** actual
x86 and AArch64 impl blocks with their corresponding frozen blocks: both
are byte-identical. The whole dispatch file is not byte-identical because of
pre-existing proof annotations/formatting (`current-dispatch-equality.log`).
No unrelated code or checker was changed, no check was retried, and this
failure was not routed to maintenance. It remains a separate check problem,
not a claimed passing bounded proof/progress handoff.

Because the manifest's executable scope is only the worker, the same
authoritative comparator also checked the two experiment-owned production
files against the current frozen branch: vmtime **59 functions / 13 structs**
match; RPC **19 functions / 3 structs** match; no unmatched executable items.
`current-preservation.log` additionally records byte-exact intake restoration.
These checks do not replace or relabel the required gate's exit 1.

From the repository root, use a **new** output filename:

```bash
"${ARGUS_SKILL_PYTHON:-python3}" research/vmtime-reset-admission/reproduce-current.py rpc \
  --log research/vmtime-reset-admission/reviewer-rpc.log
```

Expected exit: 101. The script applies only the two owned annotation overlays,
runs the exact native selector, and reverses its edits while refusing to
overwrite concurrent source changes. `keeper` and `transparent-error` select
the discriminating production comparisons. For the minimal source-method
probe, rerun the complete invocation printed in `derived-error-source.log`,
with the pinned solver set by `current-native.command`. No installation or
missing-dependency restoration was needed. The call-structure reader still
reports missing `proof_state.json` (`current-call-structure.log`); no rebuild
was attempted. Current source, not that absent graph, supplies the edges.

## Historical constructor-only experiment (before approval)

**Diagnostic, not production admission or an installation proof.** On pinned
Verus `0.2026.09.18.8ed93e5`, the unchanged `VmTimeKeeper::reset_to` cannot get
past its constructor-value rejection using ordinary specification or interface
annotations while actually translating that body. A noncapturing closure
invoking the same constructor removes this particular rejection in the real
isolated body. It then fails on seven declaration/interface errors, before
body verification conditions are checked. At the time of this historical
experiment, the normalization was unapplied. The following retained commands
and identity claims describe that older input, not the current baseline;
use `reproduce-current.py` above for the current diagnostic.

## Inputs and real connection

`identity.py` / `identity.log` compare 2,571 current source, manifest, and
verification inputs with the accepted reset-installation workspace, including
the dirty scalar work and frozen manifests. The only workspace-manifest
difference is its known integration-test target. The only lockfile difference
is the accepted workspace's already-resolved `vmcore -> vstd` dependency entry;
the live lockfile and committed Duration request remain unchanged. The pinned
frontend revision matches the repository pin and has no tracked modifications.

This permits reuse of `../vmtime-reset-installation/admission.patch` and
`admission.log`: annotation-only verification of the original body failed at
`.call(KeeperRequest::Reset, vmtime)` with exit 101 in 29.649 s. That unchanged
production experiment was **not** rerun.

The call-structure reader reports missing `.verus_agent/proof_state.json`;
no snapshot was rebuilt. The connection is directly visible in current source:

- `dispatch.rs:1831-1842` registers the `"vmtime"` state unit using `run_vmtime`.
  `restore_snapshot_state` calls `self.restore(saved_state)` at 3862 and
  `state_units.advance_time(downtime)` at 3870.
- `LoadedVm::restore`, `dispatch.rs:4765-4770`, forwards the saved units to
  `StateUnits::restore`. `vmm_core/state_unit/src/lib.rs:983-1030,1048-1069`
  dispatches `StateRequest::Restore` and `StateRequest::AdvanceTime`.
  `StateRequest::apply`, lines 235-242, passes the actual buffer/duration to
  the corresponding unit method.
- `vmm_core/src/vmtime_unit.rs:38-52` parses/forwards saved state, or forwards
  the actual duration, through `VmTimeKeeper::{restore,advance}`.
  `vm/vmcore/src/vmtime.rs:489-512` passes the selected `VmTime` to `reset_to`,
  which updates its local stopped time and awaits/unconditionally unwraps
  the reset RPC result.
- `KeeperRequest::Reset` carries `Rpc<VmTime, ()>` (`vmtime.rs:618-623`).
  `support/mesh/mesh_channel/src/rpc.rs:167-175` defines the real
  `RpcSend::call`: `F: FnOnce(Rpc<I, R>) -> Self::Message`, with no Verus
  pre/postcondition. It creates a oneshot, calls `f` once on
  `Rpc(input, result_send)`, sends that message, and returns the receiver.
  Lines 298-304 implement the sender route for `&mesh_channel_core::Sender<T>`.

These edges establish the relevant implementation, not any asynchronous
installation or lifecycle theorem. In particular, `call` has no admitted
contract granting completion or clock installation.

## Why interfaces cannot repair this expression

In pinned `source/rust_verify/src/rust_to_vir_expr.rs:2864-2871`, a resolved
constructor used as an expression is rejected whenever its constructor kind
is not `Const`. `KeeperRequest::Reset` is a tuple variant (`CtorKind::Fn`).
This branch does not consult a callee's pre/postconditions or datatype opacity.
`fn_call_to_vir.rs:328-347,2966-2971` translates ordinary call arguments with
`expr_to_vir_consume`; annotating `RpcSend::call` does not bypass its argument.
By contrast, `rust_to_vir_expr.rs:2203-2216` translates a direct constructor
invocation as a datatype constructor.

The pinned `rust_verify_test/tests/fndef_types.rs:1415-1436` tests explicitly
expect the constructor-as-function-value rejection (including `Self`).
The same file's ordinary function-value tests, the guide's
`exec_funs_as_values.md` and `reference-signature-fnonce.md`, and vstd's
`std_specs/core.rs` support function/closure contracts; they provide no
constructor-value translation override. The known reborrow proof pattern is
not relevant: this failure occurs before proof obligations.

Thus specification strengthening, a datatype declaration, or a contract on
`call` cannot change this resolved Rust expression's frontend branch.
Skipping `reset_to` with `external_body`, replacing its executable, or hiding
the expression in trusted project code is not body admission. No such bypass
was installed or tested. This conclusion is scoped to the pinned frontend
and unchanged expression, not to every future Verus version.

## Native measurements and next barrier

| Experiment | Result | Classification |
| --- | --- | --- |
| `constructor_value.rs` | Exit 1, 0.414 s; exact constructor-value rejection | Minimal native frontend reproducer, even with the datatype and generic `FnOnce` interface declared |
| `constructor_invocation.rs` | Exit 0, 0.565 s; **2 verified, 0 errors** | Native positive comparison: closure invokes the constructor and proves payload equality through `FnOnce`; no RPC claim |
| `production.patch` on the isolated real `reset_to` | Exit 101, 2.669 s; seven errors, no verification-results summary | Constructor rejection removed; production declaration/interface validation fails |

The positive comparison first reached the solver but aborted because the
ambient `z3` was 4.12.5 rather than the required 4.16.0. The complete setup
failure is retained in `constructor_invocation.setup-failure.log`. Only that
incomplete positive run was repeated, selecting the already-installed pinned
solver via `VERUS_Z3_PATH`; neither verifier nor solver was modified.

The production candidate changes only the constructor argument and adds
`#[verus_verify]` to the existing method. It retains the method's name,
signature, assertion, assignment, actual RPC call, await, and unwrap. It uses
the real dependency implementations, lifetime checking enabled, rlimit 50,
and one verification thread. No body cut, new trust, or renamed/toy production
replacement is involved. `production.log` reports all next errors:

| Missing declaration/interface | Production location in the annotated candidate |
| --- | --- |
| `VmTimeKeeper` | `vmtime.rs:505-508`, receiver and field accesses |
| `TimeState::is_started` | `vmtime.rs:506`, actual stopped-state assertion |
| `TimeState` | `vmtime.rs:506-507`, state field and constructor |
| `RpcError` | `vmtime.rs:508-511`, awaited result / unwrap |
| The sender implementation's default `RpcSend::call` | `vmtime.rs:508-509` |
| `PendingRpc` | `vmtime.rs:508-509`, returned future |
| `KeeperRequest` | `vmtime.rs:508-509`, channel message / closure result |

These are **not failed verification conditions** and do not establish that
these project-owned declarations need permanent trust. The diagnostic's
suggested `assume_specification`/external declarations were not adopted.
There is no claim that all later translation, lifetime, or proof checks pass.

## Hypothetical frozen-source boundary

`normalization.patch` identifies exactly one proposed executable-source edit
in `vm/vmcore/src/vmtime.rs`, inside `VmTimeKeeper::reset_to`:

```diff
-.call(KeeperRequest::Reset, vmtime)
+.call(|rpc| KeeperRequest::Reset(rpc), vmtime)
```

The original constructor value and noncapturing closure both move the same
`Rpc<VmTime, ()>` into the same variant. The real `call` invokes either once;
it neither stores `F` nor inspects its identity. The input, response sender,
send order, `PendingRpc`, `.await`, and `.unwrap()` are unchanged. The closure
adds no captures, allocation, clone, await, or error branch. `PendingRpc::poll`
(`rpc.rs:264-270`) still maps channel failures into `RpcError::Channel`; the
original panic/error behavior and stopped assertion remain. This is
source-grounded semantic-preservation evidence, not a proof of RPC success.

Reviewer may use this as the boundary for a future minimal frozen-source
request. Neither TOP specification nor sanctioned BOTTOM declaration needs a
proposed edit for this normalization. `production.patch` additionally records
the diagnostic-only method annotation; it is **not** a validated `run.patch`.
No freeze request was submitted or approved. This bounded task requests a
classification, and the request submission destination is outside its write
boundary. Production and the isolated experimental source were restored/
preserved byte-for-byte; neither patch is applied.

## Remaining obligations and retained checks

The seven declaration/interface issues precede proof of the real `reset_to`,
`RpcSend::call`, result polling, successful secondary installation, and the
initial-source lifecycle theorem. Other-source coverage, saved-state/Duration
correspondence, `decoded_load_restore_request_view`,
`decoded_restore_request_view`, `initialized_vm_representation`,
`loaded_vm_representation`, and the TOP `external_body` remain open.
Existing InitializedVm/LoadedVmInner and dependency declaration cuts are not
discharged. No marker or production contract was added, removed, or changed.

The current identity comparison supports reuse of the four required command
results in `../vmtime-reset-installation/{make_verify,boundary,spec_drift,exec_drift}.complete.log`;
they were not rerun on unchanged source:

| Required Argus command | Retained result |
| --- | --- |
| `make_verify --crate-root .` | Exit 0; 0 verified / 0 errors; 2.580 s. Existing wrapper selects scaffolded TOP with `--no-lifetime`, **not a full-crate or clock proof**. |
| `boundary --crate-root . --baseline-dir .verus_agent check` | Exit 1; TCB-manifest provenance rejection; 7 temporary locations, 0 assumptions, 0 permanent violations in summary. Unrepaired setup/check issue. |
| `spec_drift --crate-root . --baseline-dir .verus_agent` | Exit 0, no frozen specification drift; 1.191 s. |
| `exec_drift --crate-root . --baseline-dir .verus_agent` | Exit 0, no executable drift; 5.248 s. |

From the repository root, reproduction uses the authoritative Python:

```bash
"${ARGUS_SKILL_PYTHON:-python3}" research/vmtime-reset-admission/identity.py
"${ARGUS_SKILL_PYTHON:-python3}" research/vmtime-reset-admission/run.py validate
```

`validate` checks retained results and restored source without rerunning Verus.
On a fresh copy without the diagnostic workspace, reproduce the experiments:

```bash
"${ARGUS_SKILL_PYTHON:-python3}" research/vmtime-reset-admission/run.py native
"${ARGUS_SKILL_PYTHON:-python3}" research/vmtime-reset-admission/run.py production
```

The production command creates an owned workspace and private build cache
from the accepted diagnostic, refuses to overwrite an existing workspace,
records both patches, and restores its experimental source in `finally`.
Its successful harness exit means the expected seven-error classification
was observed, **not** that production verified. Full native logs are retained.
