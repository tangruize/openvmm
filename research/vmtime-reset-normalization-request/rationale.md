# Normalize the reset RPC constructor argument

## Requested boundary

Both patches request exactly this executable-source normalization in the real
`VmTimeKeeper::reset_to`, in `vm/vmcore/src/vmtime.rs`:

```diff
-.call(KeeperRequest::Reset, vmtime)
+.call(|rpc| KeeperRequest::Reset(rpc), vmtime)
```

`freeze.patch` applies to the current Human-owned frozen branch.
`run.patch` applies **independently**, not after `freeze.patch`, to the
current working-branch commit. The committed function is identical on those
bases, so the patches are identical. Neither patch includes the uncommitted
scalar work or any part of the separate Duration request.

No function annotation, contract, manifest, TCB declaration, dependency, or
other call site changes. In particular, the diagnostic-only
`#[verus_verify]` from `research/vmtime-reset-admission/production.patch`
is omitted: that candidate still fails declaration validation. The request
does not propose trusting project-owned RPC or lifecycle guarantees.
Human approval and atomic application are separate from this delivery.

## Concrete frontend conflict

The pinned Verus frontend rejects a tuple-variant constructor used as a
function value. `KeeperRequest::Reset` is such a constructor. The rejection
occurs while translating the argument, before body verification conditions
or a callee contract could establish anything about it.

Primary frontend sources under `toolchain/verus-src/source/rust_verify/src/`:
`rust_to_vir_expr.rs` rejects resolved non-constant constructor values in its
path-expression branch, but admits constructor
invocations through `ExprKind::Call`. `fn_call_to_vir.rs` translates the
ordinary call arguments through `expr_to_vir_consume`. A stronger RPC
contract or datatype declaration does not change this expression's branch.
The pinned `rust_verify_test/tests/fndef_types.rs` constructor-value tests
expect the same rejection.

The retained, reviewed evidence is in `research/vmtime-reset-admission/`:

- `constructor_value.rs` and `constructor_value.log`: the minimal declared
  datatype and generic `FnOnce` interface still encounter the exact rejection.
- `constructor_invocation.rs` and `constructor_invocation.log`: a
  noncapturing constructor-invoking closure with an explicit postcondition
  verifies payload equality (2 verified, 0 errors). This is a minimal
  comparison, not a substitute implementation or an RPC proof.
- `normalization.patch`: the proposed source delta. Its original line
  offsets include dirty scalar annotations; these request patches use the
  committed-base offsets without incorporating those annotations.
- `production.patch` and `production.log`: translating the isolated real
  `reset_to` with this normalization and a diagnostic method annotation
  removes the constructor-value rejection. Seven declaration/interface
  errors remain; no production body verification result was reached.
- `README.md`, `identity.py`, and `run.py validate`: source connections,
  relevant-input comparison, and retained-evidence checks. The reviewed
  admission diagnostic supports precisely this limited request.

No unchanged Verus experiment needs repeating when these inputs match.
Hiding the body behind trust, changing the verifier, or replacing the runtime
implementation would not prove this body under the frozen boundary.
The request therefore asks to normalize the expression, not to waive proof.

## Source-grounded semantic-preservation argument

`VmTimeKeeper::{restore,advance}` both await `reset_to` with their selected
`VmTime`. The TOP restore path reaches them through the registered `"vmtime"`
state unit: `LoadedVm::restore_snapshot_state`, `LoadedVm::restore`,
`StateUnits::{restore,advance_time}`, `StateRequest::apply`, and
`vmm_core/src/vmtime_unit.rs`. These source edges establish relevance to
`restored_virtual_time`, not successful asynchronous installation.

`KeeperRequest::Reset` carries `Rpc<VmTime, ()>`. The actual
`support/mesh/mesh_channel/src/rpc.rs` implementation of `RpcSend::call`
requires `F: FnOnce(Rpc<I, R>) -> Self::Message` and performs:

```rust
let (result_send, result_recv) = oneshot();
self.send_rpc(f(Rpc(input, result_send)));
PendingRpc(result_recv)
```

It invokes `f` exactly once and neither stores it nor inspects its identity.
Both the original constructor value and the proposed noncapturing closure
move that same RPC, with the same `vmtime` and response sender, into the same
`KeeperRequest::Reset` variant. The closure captures nothing and adds no
allocation, clone, await, or error branch.

The stopped-state assertion remains before the local `TimeState::Stopped`
assignment. That assignment remains before `call`; oneshot creation remains
before the single constructor invocation; message construction remains
before `send_rpc`; and the same receiver is returned, awaited, and unwrapped.
The sender implementation for `&mesh_channel_core::Sender<T>` still forwards
the message to `self.send(message)`. `PendingRpc::poll` still maps receiver
errors to `RpcError::Channel`. Assertion failure, channel errors, and the
unconditional unwrap retain their existing behavior; success is not assumed.

This argument concerns observable payload, ordering, and error behavior,
not identical code generation or callable-type identity. Mechanical patch
application and the native `spec_drift` / `exec_drift` comparison outputs
are advisory Reviewer evidence, not a proof of semantic equivalence.

## Exact benefit and unresolved obligations

This removes **one frontend rejection only**. Admission of `VmTimeKeeper`,
`TimeState`, `TimeState::is_started`, `KeeperRequest`, `RpcError`,
`PendingRpc`, and the actual sender's `RpcSend::call` remains unresolved.
In particular, `call` currently has no admitted contract establishing
completion, installation, or lifecycle behavior.

The real installation/lifecycle proofs, other-source coverage, and
saved-state/Duration correspondence remain open. The four representation
bridges `decoded_load_restore_request_view`, `decoded_restore_request_view`,
`initialized_vm_representation`, and `loaded_vm_representation` remain
uninterpreted proof debt. The TOP
`LoadedVm::restore_snapshot_state` retains its `external_body`; existing
project declaration cuts are not discharged. No temporary marker is added,
removed, moved, or sanctioned by this request. The frozen TOP contract and
sanctioned BOTTOM TCB are unchanged.
