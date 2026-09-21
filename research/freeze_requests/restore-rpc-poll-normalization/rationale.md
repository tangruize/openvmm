# Normalize the real reset RPC future without changing its meaning

## Requested change and boundary

The proposed patches add redundant whole-wrapper `Unpin` predicates to the
existing `Future` implementations for `PendingRpc<T>`,
`PendingFailableRpc<T, E>`, and `OneshotReceiver<T>`. They replace only the
constructor-valued callback in the real `PendingRpc::poll`:

```rust
.map_err(RpcError::Channel)
.map_err(|err| RpcError::Channel(err))
```

The second line replaces the first; they are not two operations. The real
poll body, receiver implementation, receiver storage, `Pin::new`,
`Pin::get_mut`, `ready!`, and failable-result conversions remain in place.
The predicates are on wrappers, never on payloads. No unchecked pin
operation, extra poll, cloned value, error conversion, or unwrap is added.

`freeze.patch` is independently based on the current frozen branch;
`run.patch` is independently based on the current working branch. The
working branch's pre-existing `Rpc` declaration proof scaffold is absent
from the frozen input. Neither patch adds, removes, or changes that
scaffold, and neither patch depends on applying the other.

This requests permission for a source normalization, not a correction to a
false restore specification or incorrect Rust behavior. Executable text is
frozen, so Engineer cannot install the normalization as an ordinary proof
edit. The existing TOP specification and sanctioned BOTTOM TCB stay fixed.

## Concrete translation conflict and smallest sufficient normalization

The source-matched `research/restore-rpc-pin-admission/rpc-transitive.log` reaches
the original production expression and rejects a datatype constructor used
as a function value. Its earlier structural/parameter probes and
`rpc-bounds.log` identify the auto-trait-resolution problem and the required
transitive wrapper predicate. `receiver-bounds-import.log` demonstrates
the corresponding receiver normalization without hiding its representation.

The pinned frontend's `rust_to_vir_base.rs` recognizes parameter-provided
`Unpin` but not its structural builtin candidate. Its
`rust_to_vir_expr.rs:2864-2869` explicitly rejects constructor function
values. Neither obstacle is an unproved caller invariant that could repair
translation without editing these frozen expressions/declarations.
The known reference-identity proof pattern is not this frontend failure.

The new isolated current-source probe, recorded in
`research/restore-rpc-poll-normalization/native-rpc.patch`, `.command`, and
`.log`, retains both real RPC Future implementations. It gets past these
two selected rejections and reports missing interfaces at the actual pin
and receiver operations. The captureless closure is sufficient; rewriting
the polling control flow as another match is unnecessary.

This is not native poll admission: declaration well-formedness still
reports `core::error::Error`, `RecvError`, `Pin`, `Context`, `Poll`,
`OneshotReceiver`, its real `poll`, `Pin::new`, and `Pin::get_mut`.
No selected production body VC is reached. Lifetime and trait-conflict
checks are enabled, not bypassed; this early failure is not a completed
body-stage lifetime check. Receiver-specific dependency diagnostics are
reused from the source-identical accepted probe, not suppressed.

## Original generic domain

The original receiver implementation has arbitrary `T`. Its storage is
`ManuallyDrop<OneshotReceiverCore>` plus `PhantomData<Arc<Mutex<T>>>`;
the core contains `Arc<Slot>`. Rust 1.95.0's
`library/alloc/src/sync.rs:4195-4196` implements `Unpin for Arc<T, A>`
without `T: Unpin`. `PendingRpc` contains that receiver and
`PendingFailableRpc<T, E>` contains `PendingRpc<Result<T, E>>`.
Their structural `Unpin` facts therefore hold for arbitrary payloads.

The accepted generic witnesses are compiled against the exact proposed
sources, checking these `Unpin` facts with unconstrained `T, E` and both
RPC Future implementations under only their original `Send + 'static`
bounds. Additional `receiver-domain.rs` witnesses check the receiver's
Future implementation with completely unconstrained `T`, including
borrowed non-`Send` payloads. `PhantomPinned` instances check that no
payload `Unpin` assumption has slipped in.

These compiler-checked universal wrapper predicates discharge the added
where-clauses at every existing Rust caller, including the failable poll's
inner `PendingRpc<Result<T, E>>` call and its transitive receiver predicate.
No caller repair, new logical `requires`, or payload-domain restriction is
needed. This does not provide Verus contracts for the pinning operations.

## Exact behavioral preservation

`preservation.json` and the recorded patches compare the full source
against the previously audited declaration-only overlays. Beyond those
predicates, there is exactly one callback expression change. Receiver and
failable poll bodies, poll signatures, and all other executable source
bytes are unchanged.

For every state and context, the common prefix
`Pin::new(&mut self.get_mut().0).poll(cx)` executes once on the same
receiver and context. Any receiver mutation, waker registration, failure,
or unwind occurs before the sole changed callback and is identical.
Rust 1.95.0's `library/core/src/task/ready.rs:50-57` defines the same
early return for `Pending`. Its `library/core/src/result.rs:962-970`
defines `map_err` by `Ok(t) => Ok(t)` and `Err(e) => Err(op(e))`.

Thus all outcomes are accounted for:

| Real inner outcome | Both old and proposed outer outcomes |
| --- | --- |
| `Pending` | Return `Pending` immediately; no callback or extra poll |
| `Ready(Ok(t))` | `Ready(Ok(t))`, moving the same successful value |
| `Ready(Err(e))` | `Ready(Err(RpcError::Channel(e)))`, moving the entire same error |

The old constructor function and new captureless closure each construct
exactly `RpcError::Channel(e)`. Neither has captures, destructors, side
effects, fallible conversions, or a new panic path. The reasoning includes
both `RecvError::Closed` and `RecvError::Error(ChannelError)` with every
inner payload; it does not flatten, reconstruct, or discard the error.
The safe pin projections and their lifetimes are textually untouched.
All later failable-call matching and all caller unwrap behavior are
unchanged, not newly proved safe.

The ordinary baseline/candidate tests call the real futures over real
oneshot channels. They cover repeated Pending, wakeup, allocation identity
and drop counts of a non-`Unpin` successful payload, closed-channel errors,
the decode-failure channel payload and its `NodeError` source, and failable
success/call/channel outcomes. The initial test incorrectly expected
`Corruption` directly; the unchanged receiver reports the failed port as
`NodeFailure`. The corrected expectation is supported by
`oneshot.rs:389-425,566-614` and `error.rs:36-40`; both versions use the
same corrected tests. Tests support the source case analysis rather than
claiming to prove every transport behavior.

## Trust, contracts, and remaining obligations

The requested trust delta is empty: no TCB declaration, diagnostic
exclusion, temporary body trust, success assumption, or additional unsafe
code. The logical contract delta is empty: no TOP or intermediate
requires/ensures, View, invariant, or representation bridge is changed.
Only already-satisfied Rust wrapper predicates and the equivalent callback
syntax change. Diagnostic declaration annotations and test code are
isolated experiments, not part of either patch.

The real reset path remains `VmTimeKeeper::reset_to` -> `RpcSend::call`
-> `PendingRpc`, with the existing await and unwrap. The source-confirmed
multi-stage TOP connection in `restore-diagnostic-separation/README.md`
is reused; its graph reader reports an absent snapshot, not a verified
direct TOP-to-reset edge. `PendingFailableRpc` is a compatibility surface,
not the reset future.

Pin/Context/Poll interfaces, receiver and error-payload admission,
diagnostic-exclusion approval, transport/lifecycle and installation
guarantees, stopped-entry and unwrap proofs, TOP `external_body`, and
`decoded_restore_request_view`, `decoded_load_restore_request_view`,
`initialized_vm_representation`, and `loaded_vm_representation` all
remain outstanding. The current boundary-provenance and executable-drift
pairing problems are separate existing blockers; this proposal neither
repairs nor conceals them. The patches must remain unapplied pending review.
