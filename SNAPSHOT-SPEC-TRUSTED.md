# Snapshot spec: trusted base and deferred items

Two parts (rule decided by the user, 2026-10-29):

1. **The trusted base**: only external things: the hypervisor, the guest,
   the hardware clock, protobuf, `mesh`, the locks' runtime
   (`parking_lot`), and the tools. These are trusted for good.
2. **Deferred**: what Verus cannot yet express (a Verus limit). Not
   trusted: each waits for a Verus patch or a workaround.

Our own code (OpenVMM's and ours) is in neither part: it is to be proved
(`SNAPSHOT-SPEC-OPEN-ITEMS.md`, section 2).

Every new item must be added to the right part. Design rationale is in
`SNAPSHOT-SPEC-DESIGN.md`; code changes are in
`SNAPSHOT-SPEC-CODE-CHANGES.md`.

Not listed: `external_type_specification` declarations (`Ex...` proxies).
Transparent ones are checked by Verus against the real type; those by
name only (`external_body`, fields hidden) only declare that a type exists and claim nothing.
Uninterpreted spec functions claim nothing by themselves; they are listed
only where an item gives them meaning.

**Placeholder ghosts.** Every `assume_new()` in the code is marked
(`PROOF TODO:`, `TRUSTED:`, `WAITS FOR PROTOCOL:` (none left since code
change 89), `OUT OF SCOPE:`; design doc, "The completeness audit"). Only the `TRUSTED:` ones are part of the
trusted base, through the contracts of the (external-boundary) functions
they are in; `scripts/verus-spec-audit.py` checks that every one is marked
and none is in verified code.

# Part 1. The trusted base (external)

## Tools

| Item | What is trusted |
|---|---|
| Verus `426d8b0` with patches 0001 to 0005 | `~/programs/verus`; `verus-patches/`: The verifier, and the patches: 0001, an `async fn` without a named return keeps its postconditions at `.await`; 0002, annotation-style ghost parameters work on an `async fn`; 0003, `external_trait_specification` accepts a trait whose bound names its own associated type, external impls with higher-ranked associated-type bounds no longer panic, and an undeclared type in a bound is an error, not a panic; 0004, `async fn` in traits; 0005, ghost inputs on trait methods (also written with `cfg_attr`) and async default methods. Targeted tests pass (`verus-patches/README.md`); the full test suite has not been run. |
| `verus_builtin_macros` `0.0.0-2026-09-20-0158`, patched | `verus-patches/verus_builtin_macros` (via `[patch.crates-io]`): The published macro crate plus patch 0005's macro fixes (`verus-patches/0005-verus_builtin_macros-published.patch`). |
| `vstd` `=0.0.0-2026-10-04-0306` | crates.io: vstd's own trusted specs (e.g. `BTreeMap::get_mut`, `FracGhost`). |
| `vstd` patch vstd-0001 | `verus-patches/vstd` (the pinned `vstd` with `verus-patches/vstd-0001-arc-unsized-view.patch`, via `[patch.crates-io]`) | `Arc<T>`'s view for unsized `T` is its contents' view (so `Arc<str>@` is the `str`'s `Seq<char>`), as for sized `T` (since 2026-10-30; replaces the uninterpreted `ArcStrView::view_str`). |

## Protobuf: saved-state blobs (`vm/vmcore/src/save_restore.spec.rs`, `save_restore_t.rs`)

A blob that encodes `value: T` has view `SavedStateBlobView::of(value)`.
Protobuf encoding and decoding are not verified.

| Item | What is trusted |
|---|---|
| `View for SavedStateBlob` (uninterpreted) | A blob's view is the typed value it encodes. |
| **AXIOM** `SavedStateBlobView::axiom_of_is_of_view` | A saved state's blob is determined by its view: `of(v) == of_view::<T>(v@)`, for every saved-state root type `T`. Sound because each saved-state type's view keeps every field except guest time (out of scope); this is reviewed per type (its `.spec.rs`; unspecified types have their identity view). |
| **AXIOM** `SavedStateBlobView::axiom_of_view_injective` | `of_view::<T>` is injective: each saved-state type's protobuf encoding loses nothing of the view. Nothing is assumed across types. |
| `SavedStateBlobView::of_view` (uninterpreted) | The blob of a saved state of type `T` given its view. |
| `assume_specification` for `SavedStateBlob::clone` | A clone encodes the same value: `result@ == blob@`. |
| `assume_specification` for `SavedStateBlob::new` / `parse` | `new(data)@ == of(data)`; a successful `parse::<T>()` returns `value` with `of(value) == blob@` (`parse` checks the type URL). Needs Verus patch 0003. |
| Protobuf trait chain declared to Verus | `SavedStateRoot`, `DescribedProtobuf`, `Protobuf`, `DefaultEncoding`, the codec traits, `DescribeMessage`, `NoResources`, the encoders `TableEncoder` and `OneofEncoder` with their traits (`external_trait_specification` / `external_type_specification`, no methods). Verus checks the declared bounds against the real traits. |
| `assume_specification` for `ProtobufMessage::new`; **AXIOM** `ProtobufMessageView::axiom_of_injective` | `openvmm/openvmm_defs/src/rpc_t.rs`: `new(data).encoded() == of(data)`; `of::<T>` is injective. As for blobs; the protobuf encoding is not verified. |

## Mesh RPC (`support/mesh/mesh_rpc_verus/src/mesh_rpc_t.rs`)

All functions are `external_body` wrappers around the real `mesh` API; their
contracts are assumptions about mesh. See the file header, and
`SNAPSHOT-SPEC-MESH-RPC.md` for an explanation of the wrapper.

| Item | What is assumed |
|---|---|
| Delivery | Each message is delivered at most once, unchanged, to the receiver of the channel it was sent on (in-process channels). A ghost sent with a message is never duplicated. |
| Replies | Each `Rpc`'s reply goes only to the `PendingCall` created with it; `complete` consumes the `Rpc`, so it is answered at most once. |
| Failure | If the message or the `Rpc` is dropped, the caller gets `Err` and no ghost. |
| `call` / `PendingCall::recv` | On `Ok`, the reply and its ghost satisfy the caller's expectation `post`; the caller had to establish `request_ok` for the message built by `f`. |
| `PendingCall::recv_failable` | As `RpcSend::call_failable`: a remote `Err(e)` becomes `RpcError::Call(e)`; on `Ok(t)`, `post(Ok(t), gout)`. |
| `recv_msg` | The received message and ghost satisfy `request_ok` for the channel's constant. |
| `send_msg` (one-way, since code change 71) | It sends `msg` with `send.send(msg)`; the caller must establish `request_ok` for `msg` and its ghost `gin`, which the receiver then gets through `recv_msg` (as `call`, without a reply). Used by `VpSet::start` (`VpEvent::Start`). |
| `split`, `complete` | `split` returns the `Rpc`'s input (`Rpc::spec_input()`, uninterpreted: the input `call` put in it) and keeps its reply contract; `complete` requires the reply contract. |
| Reply ghost type | `Rpc::reply_ok::<G>` is uninterpreted for every `G`; the only facts about it come from `call`, for `G = M::GhostOut`. Argument that a mismatched type cannot be used: file header. **Needs soundness review.** |
| Attached ghosts | `channel(k, gs, gr)` attaches `gs`/`gr` to the ends; `take_sender_ghost` / `take_receiver_ghost` return them once; `clone_sender` carries none; `recv_msg` keeps the receiver's. |
| An end passed to the VM worker (since code change 89) | `VmRpc`'s receiver, created by `vm_rpc_channel` with the protocol's ledger attached, reaches the VM worker through its launch parameters (`VmWorkerParameters`, sent by `mesh_worker`, possibly to another process). Assumed: it arrives as the same end, with its constant and its attached ghost, and messages on it are delivered as for an in-process channel (row "Delivery"). |

## The lock's runtime (`vmm_core/state_unit/src/mutex_t.rs`)

Explained in `SNAPSHOT-SPEC-MUTEX.md`: a trusted wrapper over
`parking_lot::Mutex`, a lock primitive we do not verify (as
`CloseableMutex`). Its unverified users are to be proved.

| Item | What is assumed |
|---|---|
| `Mutex::new_verified`, `acquire`, `MutexHandle::release` | The value satisfies `Pred::inv(m.param(), _)` while unlocked, for the mutex's own parameter (given to `new_verified`; since code change 94, in place of a key): `new_verified` and `release` require it, `acquire` provides it. `Mutex::new` (unverified code only): the parameter is unknown; that the value satisfies the invariant for it is to be proved, with the unverified code. Runtime: `parking_lot::Mutex<Option<V>>`; a handle dropped without `release` leaves `None` and the next lock panics. |

## `CloseableMutex` (`support/closeable_mutex/src/closeable_mutex_t.rs`)

OpenVMM's lock for chipset devices, a lock primitive we do not verify (the
user's decision, 2026-10-30): its contracts below are trusted. Its
unverified users are to be proved.

| Item | What is assumed |
|---|---|
| `CloseableMutex::new_verified`, `close_verified`, `VerifiedClosedGuard::borrow`, `borrow_mut`, `release` | While neither locked nor closed, the value satisfies its type's `LockInvariant::inv(param)` for the mutex's own parameter `param()` (given to `new_verified`; since code change 94, in place of a key): `new_verified` and `release` require it, `close_verified` provides it; `borrow_mut` gives the guard's value to change. **Closing** (since code change 95): a closed mutex panics on `lock`, and closing a closed one panics; so each mutex has a close token (a `FracGhost<()>`, id `close_id()`), handed out whole by `new_verified`: `close_verified` requires the whole token and keeps it in the guard, `release` gives it back. Runtime: the real `CloseableMutex`; a `VerifiedClosedGuard` dropped without `release` leaves the mutex closed, so later `lock` / `close` panic. |
| `CloseableMutex::lock_verified`, `VerifiedOpenGuard::borrow_mut`, `release` | As for `close_verified`, for a lock that does not close the mutex. `lock_verified` requires a piece of the close token (since code change 95), so the mutex is not closed: nobody else can hold the whole. A `VerifiedOpenGuard` dropped without `release` leaves the mutex locked (later lockers wait forever). |

## The hypervisor and the guest

| Item | Where | What is trusted |
|---|---|---|
| A halt reason's encoding | `PartitionSavedView::encode_halt` (uninterpreted) | What `mesh::payload::encode` gives for a `HaltReason` (protobuf). |
| The backends' VM-state access (`AccessVmState`), since 2026-10-30 | `state_spec::StateAccessSpec` for `AccessVmState` (`vmm_core/virt/src/state.rs`, generated by `state_trait!`): the getters', setters', and `commit`'s contracts, `hv_state()`; implemented by `virt_kvm/src/arch/x86_64/vm_state.rs` and `virt_mshv/src/x86_64/vm_state.rs` | Trusted: the hypervisor holds each VM-state element; a getter returns its current value and changes nothing; a setter sets that element only; `commit` changes nothing. The backends' implementations are thin wrappers over the hypervisor's calls (MSR / register reads and writes through the boot VP; about 70 lines for KVM, 100 for MSHV), so trusted with the hypervisor (the user's decision, 2026-10-30). WHP and hvf out of scope. |
| The partition's state handles: the hypervisor's part (since code change 73) | `virt::open_vm_state`, `virt::close_vm_state`, `new_vm_state_token`'s value (`vmm_core/virt/src/generic/generic_t.rs`); a handle's `hv_id()` | All handles of a partition's VM state (VTL 0) see one state, the hypervisor's: a handle opened with the partition's token starts with the token's value, and closing it gives the token back with the handle's final state; a new token's value is the partition's current state. |
| virtio-MMIO's interrupt status: the guest's part (the guest-memory boundary) | the guest | The status the guest sees: in the microVM's shared-status mode, a word in guest memory that the device sets and the guest clears directly. Trusted: only the guest (while its VPs run) changes it, besides the device's own interrupts (while the device runs). No contract states this yet: the guest-memory access contracts it needs are to write (with the proof of the transport's functions). |

# Part 2. Deferred (Verus limits; not trusted)

Each item is marked in the code with `PROOF DEFERRED: do not attempt to
prove this until ...`, naming what it waits for (since 2026-10-30).

| Item | Where | What is assumed until Verus supports it |
|---|---|---|
| A device boxed into `dyn DynChipsetDevice` keeps its blob | `From<T> for ResolvedChipsetDevice` (`vm/chipset_device_resources/src/lib.rs`, `external_body`) | The erased device's `saved_blob()` and `device_wf()` are the device's (Rust's `dyn` dispatch calls the device's impl). Verus cannot prove it while `DynChipsetDevice` is `async_trait` (gap G5); to remove with the G5 Verus patch. |
| `ArcMutexChipsetDeviceUnit::next_event` | `vmotherboard`, `backing/arc_mutex/state_unit.rs`, `external_body` | DEFERRED (Verus does not support `select!`): as `recv_msg`, a request comes with a half meeting the protocol for the channel's constant. |
| `ArcMutexChipsetDeviceBuilder::try_add`, `try_add_async` (not marked) | `vmotherboard`, `chipset/backing/arc_mutex/device.rs` | DEFERRED: their bounds name `dyn std::error::Error + Send + Sync` (a `dyn` of more than one trait); they lock no mutex themselves but call `inner_add`, which is marked. |
| `std::sync::Weak<T>` (the PCI paths' lock users, not marked) | `base_chipset.rs` (`weak_mutex_pci`), `dispatch.rs` (`WeakMutexPciBusDevice`) | DEFERRED (and not on the microVM's path): this vstd cannot declare `Weak<T, A: Allocator>` without the unstable `allocator_api`; code change 77. |
| `VmTimeKeeper`'s view and methods (deferred by the user's decision, 2026-10-30, not by a Verus limit) | `vm/vmcore/src/vmtime.spec.rs` (view, uninterpreted), `vmtime.rs` (`save`, `restore`, `advance`, `reset`, `start`, `stop`: `external_body` with contracts) | The view is the keeper's running flag (meant: `time` is `Started`); `save`'s result has view `VmTimeKeeperView::saved_blob()`; the others change `running` as named. Left uninterpreted: defining it from the fields needs two new `vmcore` dependencies used only for Verus (`async-task`, `mesh_rpc_verus`), and guest time is out of scope. The keeper's own methods cannot be proved until then; the vmtime unit and its other users are still verified against these contracts. |

# Out of scope (parked: neither trusted nor to prove)

Files headed `// OUT OF SCOPE (...)`: their spec is kept but parked; do
not attempt to prove anything in them. The audit script counts their
functions as out of scope (since 2026-10-30, the user).

**vmbus** (out of scope since 2026-10-27; the microVM has no VMBus; design
doc, "Out of scope for now"): `vm/devices/vmbus/vmbus_channel/src/`
`lib.rs`, `channel.rs`, `channel.spec.rs`, `channel.proof.rs`, `simple.rs`,
`simple.spec.rs`, `simple.proof.rs`; `vmm_core/src/` `vmbus_unit.rs`,
`vmbus_unit.spec.rs`, `vmbus_unit.proof.rs`. Kept (not reverted) because
the `StateUnit` trait requires the vmbus units to follow its protocol
(the units' ghost parameters; code changes 39, 46, 47), and so that the
work is not lost if vmbus comes back into scope (the user's decision,
2026-10-30). The parked items:

| Item | Where | What it says |
|---|---|---|
| vmbus channels (`vm/devices/vmbus/vmbus_channel`) | `channel.rs`, `simple.rs`, `channel.spec.rs`; `external_body` | `View for Device` (uninterpreted): `running` is `state is DeviceState::Running`. `GenericChannelHandle::id`, `ChannelHandle::id` (uninterpreted): the internal sender's constant. The handles' `start` / `stop` / `reset` / `save` / `restore`, the `offer_*` functions, and `Device::handle_state_request` meet their contracts. `Device::run_channel` (no contract yet) passes the task's share to `handle_state_request` and keeps `keeps_share`; the guest-traffic handlers change the device only while the task holds the whole share. OPEN: a new channel's task starts `Running` (design doc, "The vmbus units"). |
| vmbus channel's half reaches `unit_channel_for` | `vmm_core`'s three offer functions (`vmbus_unit.rs`), unverified | The half the `offer_*` function gives back is the one attached to the unit's sender. |

# Notes (not assumptions)

| Item | What it is |
|---|---|
| Protocols of `VmRpc` and `PartitionRequest` | None assumed. `PartitionRequest`'s protocol is written since 2026-10-30 (design doc, "The partition unit's request protocol"): stop tokens; the time-ABI and boot requests cannot be sent by verified code. `VmRpc`'s protocol is written since code change 89 (design doc, "The VM worker's request protocol"): the snapshot requests move the state units' halves, `Pause` (code change 90) and `Resume` (code change 96) move nothing, and the other 17 requests cannot be sent by verified code. |
| Normal-build blanket impl | In a non-Verus build every type implements `MeshProtocol` with `()` ghosts (the trait's spec function is erased). No effect on verification. |
| Normal-build blanket impl | In a non-Verus build every type implements `MutexPredicate` (with `Param = ()`). No effect on verification. |
| Normal-build blanket impl | In a non-Verus build every type implements `LockInvariant` (with `Param = ()`). No effect on verification. |
| `reply`'s two builds | `lib.rs`: The verification pass (`cfg(verus_keep_ghost_body)`) calls `complete` with the reply ghost; other builds make the same call with `Tracked::assume_new()` (the ghost is erased). |
| Error conversion | `lib.rs`: `anyhow_error`, `save_error`, `restore_error`: `external_body` with no contract (`RemoteError::new`); nothing assumed beyond termination. |

## Prototype only (`verus-patches/verus-rpc-prototype`, not part of OpenVMM)

`rpc_t.rs`, `rpc_async_t.rs`, `mesh_rpc_t.rs` (draft without attached
ghosts), `mutex_t.rs` (draft with a ghost predicate value): trusted wrappers
for the experiments, under the assumptions stated in each file's header.
