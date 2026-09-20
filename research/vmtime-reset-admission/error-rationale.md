# Narrow standard Error interface for the restore RPC error path

## Requested boundary

Declare `core::error::Error` in the owning `mesh_channel` crate through
`rpc::std_error_spec::ExCoreError`, including its real `Debug + Display`
supertraits and only this method:

```rust
fn source(&self) -> Option<&(dyn core::error::Error + 'static)>;
```

The method has no precondition, postcondition, model, success axiom, or
termination/completion promise. The sole proposed sanctioned marker is
`external_trait_specification` on `ExCoreError`. Existing vstd declarations
already supply Debug and Display; this request adds no declarations for them.
It adds neither a thiserror helper specification nor a project-type opacity
declaration.

The frozen-side patch supplies the existing workspace vstd dependency,
lockfile edge, and Cargo Verus opt-in needed to export this declaration.
Both patches include the ghost-only `rpc.spec.rs` from the real RPC module.
Both add `support/mesh/mesh_channel/src` to `src_roots` so the new TCB is
visible to integrity checking; neither changes the frozen branch, goal, or
TOP-spec paths. The run-side patch additionally opts the real `RpcError`
datatype into transparent verification. Its original derives, variants,
generated implementations, and all executable bodies remain intact.

## Why this is an external boundary, not a project proof

The real `VmTimeKeeper::{restore,advance}` delegate to `reset_to`, whose
awaited `PendingRpc` has `Future::Output = Result<T, RpcError<Infallible>>`.
`RpcError` derives thiserror's Error implementation, forwarding each
transparent variant through `Error::source(field.as_dyn_error())`. Sources
are `vm/vmcore/src/vmtime.rs`, `support/mesh/mesh_channel/src/rpc.rs`, and
the installed, lockfile-selected thiserror-impl 2.0.16 `src/expand.rs`
(enum source generation around lines 215-270).

Rust's core library, not this project, owns Error and its method declaration.
The pinned vstd has no Error trait declaration. Rust 1.95.0's
`library/core/src/error.rs` supplies the exact supertraits and method signature.
The pinned Verus frontend's `vir/src/well_formed.rs:100-125` requires a
declared trait for the generated method's dynamic return type. A project
lemma cannot supply that declaration. `rust_to_vir_trait.rs:194-234` also
requires external-trait supertrait bounds to match exactly.

The accepted reset diagnostic is recorded under
`research/vmtime-reset-admission/README.md`, including
`current-transparent-error.log` and `derived-error-source.rs`.
`external.rs:649-739` and `automatic_derive.rs:27-43` explain why admitting
RpcError admits the real automatically-derived Error implementation.
Excluding those implementations would hide the problem rather than resolve
it and is not part of either patch.

## Narrow benefit and reproducible comparison

The production comparison in
`research/vmtime-reset-admission/error-final/{baseline,candidate}.log`
uses the current run-tip sources, with only transparent RpcError opt-in in
the control. The candidate adds this exact Error interface and ghost include.
The original undeclared Error diagnostic is replaced by diagnostics from
inside the retained generated implementation: unsupported thiserror
`as_dyn_error` and undeclared project-owned `RecvError`.

An earlier declaration-only candidate, with no `source` method, also exposed
an unsupported `core::error::Error::source` call in that same generated body
(`error-production-candidate.log`). Adding the signature removes that
specific additional error (`error-production-source-interface.log`). This
is why the requested interface contains `source` rather than just a trait
name. No Error methods other than source are requested.

Run from the project root, using a fresh output directory:

```bash
"${ARGUS_SKILL_PYTHON:-python3}" research/vmtime-reset-admission/package-error.py \
  --output research/vmtime-reset-admission/reviewer-error-comparison
```

The script generates the two independent patches from the live frozen/run
tips, compares the actual RPC implementation in an isolated archive, and
checks that active inputs and both tips' relevant files remain unchanged.
It uses `cargo verus focus -p mesh_channel`, the real `rpc` module and
`*source*` selector, rlimit 50, one thread, and enabled lifetime and
trait-conflict checking. No generated method is copied, replaced, or marked
external. Source correspondence and the generated-body diagnostics are the
evidence; this is not a passing verification run.

## Explicit limitations and outstanding obligations

This request removes only the specified external Error/source declaration
obstruction. It does **not** establish reset admission or any body proof.
The real generated source method still needs the external thiserror helper
interface; its exact call is now identified, but no such TCB expansion is
included. `RecvError` declaration and representation are project proof work,
not an external type to sanction.

The supporting `error-interface-probe.rs` also exposes a native
Trait-Conflict-Checker rejection of `Dyn<0, ()>: Debug/Display`. Its complete
log is `error-interface-probe.log`; the earlier header-only probe has the
same rejection. These are not passing interface probes. No checker was
disabled, no supertrait removed, and no workaround is claimed. The production
comparison stops earlier at the separately named helper/RecvError errors;
it does not establish whether the later checker rejection occurs there.
Thus the proposal is sufficient for the narrowly observed declaration
obstruction, not a claim that it is sufficient for all Error admission.
Reviewer should consider this limitation before recommending any approval.

The final run patch adds no temporary assumptions or body/representation
cuts. The broader exploratory overlay reused five temporary cuts on
`RpcSend::{call,call_failable}`, sender `send_rpc`, `PendingRpc`, and
`PendingFailableRpc`; none is needed or included in the final candidate.
The supporting probe retains its existing opaque `DerivedError` diagnostic
type; it is not a runtime replacement or part of either patch.

Existing project-owned Rpc/channel representation debt, async_task admission,
RecvError/TaskMetadata/Deferred admission, stopped-entry and reset body
proofs, transport/lifecycle proofs, the four representation bridges, and the
TOP external_body remain separate. Nothing here grants RPC success,
scheduling, completion, error impossibility, clock installation, or lifecycle
guarantees. No frozen TOP contract or executable behavior is weakened.

The existing boundary provenance rejection and generic-implementation
exec_drift warning are documented with the original check logs in the reset
README. They are separate integrity-check limitations, not successful checks,
new executable changes, or justification for checker maintenance.
