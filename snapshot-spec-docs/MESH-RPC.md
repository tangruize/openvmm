# Verified RPC over mesh channels

A trusted Verus specification of OpenVMM's mesh RPC, so that verified code
can reason about a remote call the way it reasons about a local one, and
pass ghost resources (such as FracGhost halves) with requests and replies.

Code: crate `mesh_rpc_verus` (`support/mesh/mesh_rpc_verus`; the trusted
code is `src/mesh_rpc_t.rs`), usable by any crate. It is almost independent
of the snapshot work. (It started as a module of `state_unit`, its first
user.)

## The mental model: `thread::spawn`, for RPC

vstd's `thread::spawn` lets a closure have a contract across a thread
boundary: the spawning thread proves the closure's precondition and, at
`join`, assumes its postcondition; inside the thread, the precondition is
assumed and the postcondition must be proved. Ghost resources move with the
closure and its result.

This wrapper does the same for a call to a server task over a mesh channel:

| | Caller | Server (callee) |
|---|---|---|
| Precondition | proves it, at `call` | assumes it, at `recv_msg` |
| Postcondition | assumes it, when the reply arrives | proves it, at `complete` |
| Ghosts | sends one with the request, gets one with the reply | gets the request's, sends the reply's |

One difference from `spawn`: a server is a long-running task that handles
many requests, written separately from its callers, so the contract cannot
be read off a single closure. Instead the **message type** carries a
**protocol**: a rule, for each kind of message, saying what ghost must come
with it and what replies the server may give. Both ends share it.

Ghost values are zero-sized and erased at runtime; nothing extra crosses
the channel. The wrapper's trusted contracts are what make "passing" them
sound.

## How to use it

### 1. Define the protocol for your message type

```rust
impl MeshProtocol for MyRequest {
    type Const = ...;          // a ghost constant per channel, e.g. ids of ghost resources
    type GhostIn = ...;        // the ghost sent with a request
    type GhostOut = ...;       // the ghost returned with a reply
    type SenderGhost = ...;    // ghost attached to the sender when the channel is made
    type ReceiverGhost = ...;  // ghost attached to the receiver when the channel is made

    /// May `msg` be sent with `gin` on a channel with constant `k`?
    /// For a message holding an `Rpc`, also say which replies the server
    /// must give: "any reply satisfying P satisfies `rpc.reply_ok`".
    open spec fn request_ok(k: Self::Const, msg: Self, gin: Self::GhostIn) -> bool {
        match msg {
            MyRequest::Get(rpc) => ... && forall|r, gout| P(r, gout) ==> #[trigger] rpc.reply_ok(r, gout),
            ...
        }
    }
}
```

Use `()` for ghost types you do not need. The ghosts can be any tracked
values: FracGhost halves, tuples of them, other vstd resources.

### 2. Create the channel

```rust
let (send, recv) = channel::<MyRequest>(Ghost(k), Tracked(sender_ghost), Tracked(receiver_ghost));
```

Both ends get the constant `k`. The two ghosts are attached to the ends and
taken once by whoever ends up holding each end (`take_sender_ghost`,
`take_receiver_ghost`). This is how resources created with the channel
reach the two sides through code that only moves the ends around.

### 3. Caller: send a request and wait for the reply

```rust
let pending = call(
    &send,
    |rpc| -> (msg: MyRequest) ensures msg == MyRequest::Get(rpc) { MyRequest::Get(rpc) },
    input,
    Tracked(gin),
    Ghost(|r, gout| /* what I expect of the reply */),
);
match pending.recv().await {
    Ok((reply, gout)) => { /* my expectation holds of reply and gout@ */ }
    Err(_) => { /* the call failed: no reply, and gin is lost */ }
}
```

`call` requires `request_ok` for the message the closure builds, with the
`Rpc` carrying `input` and the caller's expectation. (The closure, with its
`ensures`, is needed because Verus does not accept an enum variant such as
`MyRequest::Get` as a function value.)

### 4. Server: receive, then answer

```rust
while let Some((msg, gin)) = recv_msg(&mut recv).await {
    // request_ok(k, msg, gin@) holds here
    match msg {
        MyRequest::Get(rpc) => {
            let (input, rpc) = split(rpc);      // the input, and an input-less Rpc
            ... compute reply and gout ...
            complete(rpc, reply, Tracked(gout)); // requires rpc.reply_ok(reply, gout)
        }
    }
}
```

### Example

The state-unit request protocol (`MeshProtocol for StateRequest`,
`vmm_core/state_unit/src/lib.proof.rs`) and its verified examples:
`example_save_call` (a caller lends a FracGhost half with `Save` and learns
the blob matches it) and `ExampleUnit::handle` (a server answering `Start`, `Stop`,
`Save`, `Restore`).

## API

| Function | Wraps | Contract |
|---|---|---|
| `channel(Ghost(k), Tracked(gs), Tracked(gr))` | `mesh::channel()` | both ends have constant `k`; `gs`, `gr` attached to the sender and receiver |
| `take_sender_ghost(&mut send)`, `take_receiver_ghost(&mut recv)` | (no-op) | return the attached ghost, once; afterwards none is attached |
| `clone_sender(&send)` | `Sender::clone` | same constant; no ghost attached |
| `call(&send, f, input, Tracked(gin), Ghost(post))` | `RpcSend::call` | requires `request_ok(k, f(rpc), gin)` for an `Rpc` carrying `input` and expecting `post`; returns a `PendingCall` expecting `post` |
| `PendingCall::recv().await` | awaiting `PendingRpc` | on `Ok((reply, gout))`: `post(reply, gout@)` |
| `PendingCall::recv_failable().await` (reply type `Result<T, E>`) | awaiting `RpcSend::call_failable`'s future | a remote `Err(e)` becomes `RpcError::Call(e)` (its ghost dropped); on `Ok((t, gout))`: `post(Ok(t), gout@)` |
| `recv_msg(&mut recv).await` | `Receiver::recv` | on `Some((msg, gin))`: `request_ok(k, msg, gin@)`; `None` when every sender is gone |
| `split(rpc)` | `Rpc::split` | returns the input (`rpc.spec_input()`) and an `Rpc` with the same reply expectation |
| `complete(rpc, reply, Tracked(gout))` | `Rpc::complete` | requires `rpc.reply_ok(reply, gout)` |

Spec functions on mesh's own types come from traits implemented for them:
`send.constant()`, `send.attached()`, `recv.constant()`, `recv.attached()`,
`rpc.spec_input()`, `rpc.reply_ok(r, g)`, `pending.reply_ok(r, g)`.

## What is trusted

Everything in `mesh_rpc_verus` (`src/mesh_rpc_t.rs`) is trusted: the functions are
`external_body` wrappers around the real mesh API, and their contracts are
assumptions about it (also listed in `TRUSTED.md`):

- **Delivery**: each message is delivered at most once, unchanged, to the
  receiver of the channel it was sent on (in-process channels), so a ghost
  is never duplicated.
- **Matching**: each `Rpc`'s reply goes only to the `PendingCall` created
  with it; `complete` consumes the `Rpc`, so it is answered at most once.
- **Failure**: if the message or the `Rpc` is dropped, the caller gets `Err`
  and no ghost; the ghost it sent is lost. Nothing can then be proved from
  it, which is acceptable on an error path.
- **Attached ghosts**: an end whose ghost is taken was created by `channel`
  (not by `mesh::channel` directly, whose ends' `attached()` is unknown).

## Design details

- **The caller's expectation travels with the `Rpc`.** `call` makes
  `rpc.reply_ok` equal to the caller's `post` and gives the same expectation
  to the returned `PendingCall`. The server, holding only the `Rpc`, must
  meet `rpc.reply_ok` at `complete`; the caller gets `post` back when the
  reply arrives. `request_ok` connects the two: it is how the server learns
  which replies satisfy the expectation, and it stops a caller from asking
  for something the protocol does not let the server deliver.
- **Why the caller chooses `post`.** Different requests have different
  reply types (`Rpc<I, R>` per message variant), so the expectation is a
  spec function of the reply and the returned ghost, given per call, rather
  than one fixed function of the message type.
- **The reply ghost's type.** `rpc.reply_ok::<G>` is uninterpreted for
  every type `G`; the only facts about it come from `call`, for
  `G = M::GhostOut` of the channel the `Rpc` was sent on, so a server cannot
  complete an `Rpc` with a ghost of another type (it could not prove
  `reply_ok` for it). **This argument needs a soundness review.**
- **Normal builds.** In a non-Verus build the `MeshProtocol` trait has no
  spec function, and a blanket impl gives every type `()` ghosts. The
  wrappers compile to plain calls of the mesh functions.

## Status and limitations

- **Tested**: a runtime round trip over a real mesh channel (`mesh_rpc_verus`
  test `call_round_trip`); the verified examples above, with injected bugs
  failing.
- **Used by OpenVMM's production code** (since 2026-10-27), for three
  message types:
  - `StateRequest` (state units): `StateUnits` sends with `call` /
    `PendingCall::recv` (`call_unit`); units receive with `recv_msg`
    (`run_unit`, `run_async_unit`) and reply with `split` / `complete`
    (`StateRequest::apply`).
  - `VmRpc` (controller to VM worker), the microVM snapshot requests
    (`QuiesceForSnapshot`, `ResumeAfterFailedSnapshot`,
    `ReleaseSnapshotBoundary`), both ends.
  - `PartitionRequest` (to the partition unit's task), the requests on the
    snapshot and restore path (`StopVps`, and `Snapshot`'s
    `StopVpsAtIoBoundary`, `CheckTimers`, `AdvanceLapic`, `OmitSavedTsc`),
    both ends.

  The code is unverified and passes placeholder ghosts
  (`Tracked::assume_new()`, `Ghost::assume_new()`), as the runtime test
  does; the verified unit `DeviceUnit` passes real ones. Other requests of
  `VmRpc` and `PartitionRequest` still call mesh directly; the two mix,
  since the wrapper sends the same messages.
- **Placeholder protocols.** `VmRpc`'s (`openvmm/openvmm_defs/src/rpc.spec.rs`)
  and `PartitionRequest`'s (`vmm_core/src/partition_unit.spec.rs`)
  `request_ok` are uninterpreted, so verified code can neither send these
  messages nor learn anything from receiving them. (A protocol of `true`
  would let a verified caller expect anything of a reply, and `false`
  would let a verified receiver conclude anything; both unsound.) The
  protocols are left to whoever verifies those layers.
- **Async limitations in Verus** affect how it can be used from OpenVMM's
  code: annotation-style ghost parameters of an `async fn` needed Verus
  patch 0002; ghost outputs of an `async fn` are still unsupported in
  annotation style (use `verus!` style, or tracked `&mut` inputs); `async`
  blocks, `async fn` in traits, and async closures are unsupported.
- **Where it lives.** The idea belongs next to `vstd::thread::spawn`. This
  code is specific to OpenVMM's mesh crate, which vstd cannot depend on, so
  it is a small crate next to mesh (`mesh_rpc_verus`). A vstd version would
  need a vstd channel type.
