# Snapshot spec: trusted base

Everything the snapshot save/restore proof trusts rather than proves:
axioms, assumed specifications, trusted wrappers, and assumptions about
unverified code. Every new trusted item must be added here.

Design rationale is in `SNAPSHOT-SPEC-DESIGN.md`; code changes are in
`SNAPSHOT-SPEC-CODE-CHANGES.md`.

Not listed: `external_type_specification` declarations (`Ex...` proxies).
Transparent ones are checked by Verus against the real type; opaque ones
(`external_body`) only declare that a type exists and claim nothing.
Uninterpreted spec functions claim nothing by themselves; they are listed
only where a trusted item gives them meaning.

## Tool

| Item | Where | What is trusted |
|---|---|---|
| Verus `426d8b0` with patches 0001 and 0002 | `~/programs/verus`; `verus-patches/` | The verifier, and the patches: 0001, an `async fn` without a named return keeps its postconditions at `.await`; 0002, annotation-style ghost parameters work on an `async fn`. Targeted tests pass (`verus-patches/README.md`); the full test suite has not been run. |
| `vstd` `=0.0.0-2026-10-04-0306` | crates.io | vstd's own trusted specs (e.g. `BTreeMap::get_mut`, `FracGhost`). |

## Contracts assumed until verified

| Item | Where | What is assumed |
|---|---|---|
| `LoadedVm::save` | `openvmm/openvmm_core/src/worker/dispatch.rs` | Body is `external_body`: its contract (see design doc, "Contracts") is assumed, not proved. |
| `LoadedVm::restore` | same | Same. |

## Saved-state blobs (`vm/vmcore/src/save_restore.spec.rs`)

| Item | What is trusted |
|---|---|
| `View for SavedStateBlob` (uninterpreted) | A blob's view is the typed value it encodes. The encode/decode contract (`new`, `parse`) is still to be written (design doc, gap 4). |
| `assume_specification` for `SavedStateBlob::clone` | A clone encodes the same value: `result@ == blob@`. |

## Mesh RPC (`vmm_core/state_unit/src/mesh_rpc_t.rs`)

All functions are `external_body` wrappers around the real `mesh` API; their
contracts are assumptions about mesh. See the file header.

| Item | What is assumed |
|---|---|
| Delivery | Each message is delivered at most once, unchanged, to the receiver of the channel it was sent on (in-process channels). A ghost sent with a message is never duplicated. |
| Replies | Each `Rpc`'s reply goes only to the `PendingCall` created with it; `complete` consumes the `Rpc`, so it is answered at most once. |
| Failure | If the message or the `Rpc` is dropped, the caller gets `Err` and no ghost. |
| `call` / `PendingCall::recv` | On `Ok`, the reply and its ghost satisfy the caller's expectation `post`; the caller had to establish `request_ok` for the message built by `f`. |
| `recv_msg` | The received message and ghost satisfy `request_ok` for the channel's constant. |
| `split`, `complete` | `split` returns the `Rpc`'s input and keeps its reply contract; `complete` requires the reply contract. |
| Reply ghost type | `Rpc::reply_ok::<G>` is uninterpreted for every `G`; the only facts about it come from `call`, for `G = M::GhostOut`. Argument that a mismatched type cannot be used: file header. **Needs soundness review.** |
| Attached ghosts | `channel(k, gs, gr)` attaches `gs`/`gr` to the ends; `take_sender_ghost` / `take_receiver_ghost` return them once; `clone_sender` carries none; `recv_msg` keeps the receiver's. |
| Ends from `channel` | ASSUMPTION: an end whose ghost is taken was created by `channel`, not by `mesh::channel` directly (whose `attached()` is unknown). |
| Normal-build blanket impl | In a non-Verus build every type implements `MeshProtocol` with `()` ghosts (the trait's spec function is erased). No effect on verification. |

## Mutex (`vmm_core/state_unit/src/mutex_t.rs`)

| Item | What is assumed |
|---|---|
| `Mutex::new`, `acquire`, `MutexHandle::release` | The value satisfies `Pred::inv` while unlocked: `new` and `release` require it, `acquire` provides it. Runtime: `parking_lot::Mutex<Option<V>>`; a handle dropped without `release` leaves `None` and the next lock panics. |
| `Mutex::lock` (hidden from Verus) | ASSUMPTION: unverified code that locks keeps the invariant. |
| Normal-build blanket impl | In a non-Verus build every type implements `MutexPredicate`. No effect on verification. |

## State units (`vmm_core/state_unit`)

| Item | Where | What is assumed |
|---|---|---|
| **AXIOM: registry unchanged since collection** | `registry_t.rs`, `StateUnits::lock_registry` | Each time `StateUnits` locks its registry (`lock_registry`, used instead of `Mutex::acquire` by verified code), the value taken out agrees with the units collected so far (`StateUnits::agrees_with`): since the last collection, no collected unit was removed or changed its name or channel, and every unit added since still has its half attached to its sender. Needed because units are added through `&StateUnits` and removed through `Weak` handles, both unverified, so no lock invariant can tie the registry to `StateUnits`' ghost fields. Stated only about `StateUnits`' own registry, so it cannot be applied to another `Inner`. |
| **AXIOM: registry exact inside a `&mut` operation** | `registry_t.rs`, `StateUnits::lock_registry_exact` | Inside an operation that holds `&mut StateUnits` and has collected the halves since it began, the registry holds exactly the collected units. Registration needs `&StateUnits`, which cannot exist while `&mut StateUnits` is held; removal is excluded by the axiom above. For use by such operations only, after `collect_halves`. |
| `assume_specification` for `<State as PartialEq>::eq` | `registry_t.rs` | `State`'s derived equality is structural. |
| `assume_specification` for `<ReadySet as Clone>::clone` | `registry_t.rs` | A clone holds the same units (`ids()`). |
| `assume_specification` for `Inner::ready_set` | `registry_t.rs` | With `unit_ids` `None`, the ready set holds every registered unit. |
| Registry lock invariant kept by unverified code | `lib.proof.rs`, `InnerPred` | `UnitBuilder::build` and `UnitHandle` removal are unverified and change the registry; they are trusted to keep the invariant: each registered sender's attached half (while there) belongs to its channel and records `Unknown`, and registered units have distinct names (`build` checks `names`). In particular, `build`'s callers pass a sender from `unit_channel`. |
| `Inspect` requests | `lib.proof.rs`, request protocol | An inspect request does not change a unit's saved state (it reaches the unit through `InspectMut`, with no ghost). |
| No channel failure for a registered unit | request protocol | A registered unit's task does not exit: OpenVMM's `run_op` panics on such a failure, and the unit's half would be lost. |
| Unit side follows the protocol | request protocol | Until the unit side (`run_unit`, `StateRequest::apply`, each `StateUnit`) is verified, units are assumed to answer by `request_ok`. Blocked on Verus support for async trait functions and async closures (design doc, gap 1). |

## Assumed device facts

| Item | What is assumed |
|---|---|
| Stopped devices do not change | While the state units are stopped, a chipset device's `CloseableMutex` keeps vCPUs out, so its state changes only through its unit (design doc, gap 7). |

## Prototype only (`verus-patches/verus-rpc-prototype`, not part of OpenVMM)

`rpc_t.rs`, `rpc_async_t.rs`, `mesh_rpc_t.rs` (draft without attached
ghosts), `mutex_t.rs` (draft with a ghost predicate value): trusted wrappers
for the experiments, under the assumptions stated in each file's header.
