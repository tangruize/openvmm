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
| Verus `426d8b0` with patches 0001 to 0005 | `~/programs/verus`; `verus-patches/` | The verifier, and the patches: 0001, an `async fn` without a named return keeps its postconditions at `.await`; 0002, annotation-style ghost parameters work on an `async fn`; 0003, `external_trait_specification` accepts a trait whose bound names its own associated type, external impls with higher-ranked associated-type bounds no longer panic, and an undeclared type in a bound is an error, not a panic; 0004, `async fn` in traits; 0005, ghost inputs on trait methods (also written with `cfg_attr`) and async default methods. Targeted tests pass (`verus-patches/README.md`); the full test suite has not been run. |
| `verus_builtin_macros` `0.0.0-2026-09-20-0158`, patched | `verus-patches/verus_builtin_macros` (via `[patch.crates-io]`) | The published macro crate plus patch 0005's macro fixes (`verus-patches/0005-verus_builtin_macros-published.patch`). |
| `vstd` `=0.0.0-2026-10-04-0306` | crates.io | vstd's own trusted specs (e.g. `BTreeMap::get_mut`, `FracGhost`). |

## Contracts assumed until verified

| Item | Where | What is assumed |
|---|---|---|
| `LoadedVm::save` | `openvmm/openvmm_core/src/worker/dispatch.rs` | Body is `external_body`: its contract (see design doc, "Contracts") is assumed, not proved. |
| `LoadedVm::restore` | same | Same. |
| `StateUnits::stop`, `reset`, `save`, `restore` | `vmm_core/state_unit/src/lib.rs` | Same (`external_body`; contracts in the design doc, "State-unit contracts"). |
| `StateUnits::inventory`, `StateUnits::validate_inventory` | `vmm_core/state_unit/src/inventory.rs` | Same. |
| `validate_inventory` (free function) | `openvmm/openvmm_core/src/worker/dispatch/restore.rs` | Same. |
| `LoadedVm::quiesce_for_snapshot`, `resume_after_failed_snapshot`, `establish_snapshot_boundary`, `release_snapshot_boundary`, `handle_snapshot_boundary` | `openvmm/openvmm_core/src/worker/dispatch/microvm.rs` | Same (design doc, "Quiesce and save"). |
| `StateUnits::resume_after_failed_save`, `QuiesceForSaveError::has_uncertain_state` | `vmm_core/state_unit/src/quiesce.rs` | Same. `has_uncertain_state` returns the error's `uncertain` field, which is `QuiesceForSaveError`'s view (uninterpreted). |
| `View for SnapshotBoundary` (uninterpreted) | `dispatch.spec.rs` | `active` is `stop_guard.is_some()` (private fields). |
| `LoadedVm::quiesce_and_save`, `LoadedVm::capture_time_abi` | `openvmm/openvmm_core/src/worker/dispatch/microvm.rs` | Same (design doc, "Quiesce and save"). `capture_time_abi`'s contract (the view is unchanged) is a claim about the VPs' timer check and the partition's time records. |
| `StateUnits::quiesce_for_save` | `vmm_core/state_unit/src/quiesce.rs` | Same. Its `Stop` requests go through the unverified `state_change` path. |
| `LoadedVm::restore_from_snapshot`, `begin_snapshot_restore`, `finish_snapshot_restore` | `openvmm/openvmm_core/src/worker/dispatch/restore.rs` | Same (design doc, "Restore from snapshot"). |
| `InitializedVm::load` | `openvmm/openvmm_core/src/worker/dispatch.rs` | Same (design doc, "Restore from snapshot"). Includes the fresh-VM facts: every unit's half collected, recording `Unknown`. |
| `LoadedVm::time_abi_restore` | `openvmm/openvmm_core/src/worker/dispatch/time_abi.rs` | Same. Claims the clock restore changes only time (so nothing in the view). |
| `StateUnits::advance_time` | `vmm_core/state_unit/src/advance_time.rs` | Same. |
| `View for SnapshotRestore` (uninterpreted) | `openvmm/openvmm_core/src/worker/dispatch.spec.rs` | `clock_restore_pending` is `time_abi.is_some()` (`SnapshotRestore` has private fields, so Verus cannot see it). |

## Proofs pending

Lemmas stated but not yet proved (`external_body`). They are proof
obligations, not intended assumptions.

| Lemma | Where | Statement |
|---|---|---|
| `StateUnits::lemma_view_inv` | `vmm_core/state_unit/src/lib.proof.rs` | If `StateUnits` holds a half for exactly the collected units and those have distinct names, `self@.inv()` holds (the inventory, built from the collected ids by `to_sorted_seq`, lists each unit once). |

## Saved-state blobs (`vm/vmcore/src/save_restore.spec.rs`, `save_restore_t.rs`)

A blob that encodes `value: T` has view `SavedStateBlobView::of(value)`.
Protobuf encoding and decoding are not verified.

| Item | What is trusted |
|---|---|
| `View for SavedStateBlob` (uninterpreted) | A blob's view is the typed value it encodes. |
| **AXIOM** `SavedStateBlobView::axiom_of_injective`, `axiom_of_forgets_time` | `of(a) == of(b)` exactly when `equal_except_time(a, b)`: each saved-state type's protobuf encoding loses nothing, and the view forgets only the time fields of the time-keeping units' saved states (guest time is out of scope). Nothing is assumed across types. |
| `SavedStateBlobView::equal_except_time` (uninterpreted) | Equality for a type without time fields; for the partition's VP state, vmtime's and the RTC's saved state, equality except the TSC and LAPIC timer state, the VM time and its stop time, and the clock. Trusted per type. |
| `assume_specification` for `SavedStateBlob::clone` | A clone encodes the same value: `result@ == blob@`. |
| `assume_specification` for `SavedStateBlob::new` / `parse` | `new(data)@ == of(data)`; a successful `parse::<T>()` returns `value` with `of(value) == blob@` (`parse` checks the type URL). Needs Verus patch 0003. |
| Protobuf trait chain declared to Verus | `SavedStateRoot`, `DescribedProtobuf`, `Protobuf`, `DefaultEncoding`, the codec traits, `DescribeMessage`, `NoResources`, the encoders `TableEncoder` and `OneofEncoder` with their traits (`external_trait_specification` / `external_type_specification`, no methods). Verus checks the declared bounds against the real traits. |
| `assume_specification` for `ProtobufMessage::new`; **AXIOM** `ProtobufMessageView::axiom_of_injective` | `openvmm/openvmm_defs/src/rpc_t.rs` | `new(data).encoded() == of(data)`; `of::<T>` is injective. As for blobs; the protobuf encoding is not verified. |
| `assume_specification` for `RestoreError: From<payload::Error>` | No contract (nothing assumed beyond termination). |
| Unverified `ProtobufSaveRestore` implementations meet its contract | `ChipsetDeviceProxy` (`workers/chipset_device_worker`), `BoundVp` (`vmm_core/src/partition_unit/vp_set.rs`), `ErasedChipsetDevice` (`vm/chipset_device_resources`): `save` returns `saved_blob()` and keeps it; `restore` sets it. (The blanket impl for `SaveRestore` types is verified.) |

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
| Protocols of `VmRpc` and `PartitionRequest` | None assumed: `request_ok` is uninterpreted (placeholders; `openvmm_defs/src/rpc.spec.rs`, `vmm_core/src/partition_unit.spec.rs`). Their unverified call sites pass placeholder ghosts. |
| `recv_msg` | The received message and ghost satisfy `request_ok` for the channel's constant. |
| `split`, `complete` | `split` returns the `Rpc`'s input and keeps its reply contract; `complete` requires the reply contract. |
| Reply ghost type | `Rpc::reply_ok::<G>` is uninterpreted for every `G`; the only facts about it come from `call`, for `G = M::GhostOut`. Argument that a mismatched type cannot be used: file header. **Needs soundness review.** |
| Attached ghosts | `channel(k, gs, gr)` attaches `gs`/`gr` to the ends; `take_sender_ghost` / `take_receiver_ghost` return them once; `clone_sender` carries none; `recv_msg` keeps the receiver's. |
| Ends from `channel` | ASSUMPTION: an end whose ghost is taken was created by `channel`, not by `mesh::channel` directly (whose `attached()` is unknown). |
| Normal-build blanket impl | In a non-Verus build every type implements `MeshProtocol` with `()` ghosts (the trait's spec function is erased). No effect on verification. |

## Mutex (`vmm_core/state_unit/src/mutex_t.rs`)

Explained in `SNAPSHOT-SPEC-MUTEX.md`.

| Item | What is assumed |
|---|---|
| `Mutex::new`, `acquire`, `MutexHandle::release` | The value satisfies `Pred::inv` while unlocked, and keeps the mutex's key (`Pred::key`): `new` and `release` require it, `acquire` provides it. Runtime: `parking_lot::Mutex<Option<V>>`; a handle dropped without `release` leaves `None` and the next lock panics. |
| `Mutex::lock` (hidden from Verus) | ASSUMPTION: unverified code that locks keeps the invariant and the key. |
| Normal-build blanket impl | In a non-Verus build every type implements `MutexPredicate`. No effect on verification. |

## Closeable mutex (`support/closeable_mutex/src/closeable_mutex_t.rs`)

| Item | What is assumed |
|---|---|
| `CloseableMutex::new_verified`, `close_verified`, `VerifiedClosedGuard::borrow`, `borrow_mut`, `release` | While neither locked nor closed, the value satisfies its type's `LockInvariant::inv`, and its `key()` is the mutex's `key()`: `new_verified` and `release` require it, `close_verified` provides it; `borrow_mut` gives the guard's value to change. Runtime: the real `CloseableMutex`; a `VerifiedClosedGuard` dropped without `release` leaves the mutex closed, so later `lock` / `close` panic. |
| `CloseableMutex::lock_verified`, `VerifiedOpenGuard::borrow_mut`, `release` | As for `close_verified`, for a lock that does not close the mutex. A `VerifiedOpenGuard` dropped without `release` leaves the mutex locked (later lockers wait forever). |
| Unverified `lock`, `lock_if_open`, `close` | ASSUMPTION: unverified code keeps the invariant and the key. For a device: the vCPU dispatch and line-set code change it only through methods whose contracts keep its `wf()`, and call them only when their preconditions hold (for I/O handlers: the device is running). |
| Unverified `CloseableMutex::new` | ASSUMPTION: as if by `new_verified`: the value satisfied the invariant, and the mutex's key is the value's key. (Today every device's mutex is created this way, in `vmm_core/vmotherboard`.) |
| Normal-build blanket impl | In a non-Verus build every type implements `LockInvariant` (with `Key = ()`). No effect on verification. |

## State units (`vmm_core/state_unit`)

| Item | Where | What is assumed |
|---|---|---|
| No registration during a `&mut StateUnits` operation | `lib.rs`, for future proofs of `run_op` and the operations built on it | While an operation holds `&mut StateUnits`, no unit is registered: registration (`UnitBuilder::build`) needs `&StateUnits`. Removal (`UnitHandle`'s `Drop`) can happen; the operations' specs must allow it. (Replaces the two registry axioms, removed 2026-10-27.) |
| Initial ghost state | `StateUnits::new` (unverified) | It builds the registry with `new_ghosts()`'s two linked halves, and the mutex with that value, so `StateUnits::wf()` holds. |
| `assume_specification` for `<State as PartialEq>::eq` | `registry_t.rs` | `State`'s derived equality is structural. |
| `assume_specification` for `<ReadySet as Clone>::clone` | `registry_t.rs` | A clone holds the same units (same view). |
| `assume_specification` for `Inner::ready_set` | `registry_t.rs` | With `unit_ids` `None`, the ready set holds every registered unit. |
| Units collected before use | `make_ids` (`dispatch.rs`); `StateUnitsView` | `make_ids` collects the halves of all units registered while loading (Verus-only call). ASSUMPTION: a unit registered later (hot-add) is collected before the next `stop`, `reset`, `save`, or `restore`, so `StateUnits`' view covers every registered unit. |
| Registry lock invariant kept by unverified code | `lib.proof.rs`, `InnerPred` | Registration and removal change the registry only through `Inner::insert_unit` / `remove_unit` (verified); their unverified callers are trusted to meet the preconditions: `build` checks the name is new and passes a sender from `unit_channel` (with its half attached). Other unverified code that locks the registry (`run_op`, `start_with_rollback`, ...) changes only units' states and dependency lists, and is trusted to keep the invariant (and the mutex's key). |
| **Recorded unit states are real** | `lib.proof.rs`, `ExampleUnit::handle` precondition; `assume` in `DeviceUnit::handle`'s `Stop` and in `DeviceUnit::poll` (`device_unit.rs`), and in the PIT's `example_stop` | A unit receives `Stop` only while it is running, that is, `StateUnits`' registry records a unit `Running` only when it is (`run_op` sends `Stop` only to units recorded `Running`). See "`StateUnits`' unit states are the units' real states" in the design doc; may later be proved with a ghost instead. |
| `Inspect` requests | `lib.proof.rs`, request protocol | An inspect request does not change a unit's saved state (it reaches the unit through `InspectMut`, with no ghost). |
| No channel failure for a registered unit | request protocol | A registered unit's task does not exit: OpenVMM's `run_op` panics on such a failure, and the unit's half would be lost. |
| Unit side follows the protocol | request protocol | `StateRequest::apply`, `run_unit`, and `run_unit_step` are verified against `StateUnit`'s contract (`lib.rs`). Still assumed: units whose requests do not go through them answer by `request_ok`: the vmbus units (`run_async_unit` / `apply_with_concurrent_inspects`, unverified) and the units with their own request loop calling `apply` without the half (`ArcMutexChipsetDeviceUnit::run`, the chipset unit and line-set loops in `vmotherboard`, the input distributor). |
| `run_unit`'s callers | `vmtime_unit.rs` (`KeeperUnit`), `dispatch.rs` (`TimeAbiUnit`), unverified | They pass a unit with `wf()` and its own channel's receiver (`recv.constant() == unit.id()`). |
| `reply`'s two builds | `lib.rs` | The verification pass (`cfg(verus_keep_ghost_body)`) calls `complete` with the reply ghost; other builds make the same call with `Tracked::assume_new()` (the ghost is erased). |
| Error conversion | `lib.rs`: `anyhow_error`, `save_error`, `restore_error` | `external_body` with no contract (`RemoteError::new`); nothing assumed beyond termination. |
| Production `StateUnit` impls meet the contract | `ArcMutexChipsetDeviceUnit`, `ChipsetUnit`, `LineSetUnit` (`vmm_core/vmotherboard`); `KeeperUnit` (`vmtime_unit.rs`), the input distributor's `Inner` (`input_distributor.rs`), `PartitionUnitRunner` (`partition_unit.rs`), `&VmbusServerUnit`, `&ChannelUnit<T>`, `&SimpleChannelUnit<T>` (`vmbus_unit.rs`) (`vmm_core`); `TimeAbiUnit` (`openvmm_core`, `dispatch/time_abi.rs`) | Each method body is `external_body` until proved, and each unit's `StateUnitSpec` (`wf`, `id`, `running`, in the unit's `.spec.rs`) is uninterpreted until its per-unit definition is written. |
| `InspectMut::inspect_mut` (no contract) | `lib.proof.rs` declares `InspectMut` and `Inspect` without methods; `inspect_unit` (`lib.rs`, `external_body`) states the assumption for `apply` | An inspect keeps a unit's `wf()`, `id()`, and `running()` (the `Inspect` request path in `apply` and `run_async_unit`'s concurrent inspects). Open: give `inspect_mut` a contract. |
| Unverified `SaveRestore` implementations meet its contract | `vm/vmcore/src/save_restore.spec.rs` | The `SaveRestore` contract (`saved_state()`; `save` returns it and keeps it; `restore` sets it) is checked only for implementations that are verified (with `SaveRestoreSpecImpl`); all others, i.e. every device today, are trusted to meet it. |

## Assumed device facts

| Item | What is assumed |
|---|---|
| PIT entry points meet their contracts | `vm/devices/chipset/src/pit/mod.rs`: `start_device`, `reset_device`, `poll`, `pio_read`, `pio_write`, `save_device`, `restore_device` are `external_body` until proved. |
| PIT handlers called only while running | Their callers (vCPU port I/O dispatch, the unit's poll loop, `ChangeDeviceState::start`) are unverified; they are trusted to call them only when their preconditions hold (`wf()`, and running). |
| `DeviceUnit::run` (unverified glue) | Passes each message and ghost from `recv_msg` to `handle` unchanged, and polls only while its `running` flag is set; `recv` is the receiver of the unit's own channel (constant `id()`). |
| Error conversion | `device_unit.rs`: `save_error`, `restore_error` (`RemoteError::new`), `external_body` with no contract (nothing assumed beyond termination). |
| PIT ghost placeholders at creation | `PitDevice::new` (unverified) fills `ghost` and `id` with `assume_new()`; until the creation hand-off is done, the PIT's ghost is not tied to its unit's channel. |
| Stopped devices do not change | While the state units are stopped, no vCPU accesses a device: the partition unit (VPs) is stopped before the devices (stop order: dependants first) and stays stopped until `start`, and stopping it stops the vCPUs. (Not the `CloseableMutex`: a unit closes its device's mutex only during each operation.) Also, a stopped device has no activity of its own (its `stop` stops its timers and async work). To be replaced by a proof: see the design doc, "Design change: the FracGhost lives with the device". |

## Prototype only (`verus-patches/verus-rpc-prototype`, not part of OpenVMM)

`rpc_t.rs`, `rpc_async_t.rs`, `mesh_rpc_t.rs` (draft without attached
ghosts), `mutex_t.rs` (draft with a ghost predicate value): trusted wrappers
for the experiments, under the assumptions stated in each file's header.
