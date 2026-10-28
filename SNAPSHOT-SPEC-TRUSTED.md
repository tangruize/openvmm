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
| **Recorded unit states are real** | `lib.proof.rs`, `ExampleUnit::handle` precondition; `assume` in `StateRequest::apply`'s `Stop` (`lib.rs`) | A unit receives `Stop` only while it is running, that is, `StateUnits`' registry records a unit `Running` only when it is (`run_op` sends `Stop` only to units recorded `Running`). See "`StateUnits`' unit states are the units' real states" in the design doc; may later be proved with a ghost instead. |
| `Inspect` requests | `lib.proof.rs`, request protocol | An inspect request does not change a unit's saved state (it reaches the unit through `InspectMut`, with no ghost). |
| No channel failure for a registered unit | request protocol | A registered unit's task does not exit: OpenVMM's `run_op` panics on such a failure, and the unit's half would be lost. |
| Unit side follows the protocol | request protocol | `StateRequest::apply`, `run_unit`, and `run_unit_step` are verified against `StateUnit`'s contract (`lib.rs`). So are `handle_request` and `ArcMutexChipsetDeviceUnit::run` (`vmotherboard`), and the chipset unit and the line sets now run through `run_unit`. Still assumed: units whose requests do not go through them answer by `request_ok`: the vmbus units (`run_async_unit` / `apply_with_concurrent_inspects`, `external_body` with the contracts of `run_unit` / `apply`; the units still pass no halves to their owning tasks) and the input distributor (its own loop calls `apply` without the half). |
| `run_unit`'s and `ArcMutexChipsetDeviceUnit::run`'s callers | `vmtime_unit.rs` (`KeeperUnit`), `dispatch.rs` (`TimeAbiUnit`), `vmotherboard` (`builder/mod.rs` for the chipset unit, `add_dyn_device`; `line_sets.rs`; `backing/arc_mutex/services.rs`), unverified | They pass a unit with `wf()` and its own channel's receiver (`recv.constant() == unit.id()`). |
| `ArcMutexChipsetDeviceUnit::next_event`, `poll_now` | `vmotherboard`, `backing/arc_mutex/state_unit.rs`, `external_body` | `next_event` (the `select!` between the next request and the device's poll request): as `recv_msg`, a request comes with a half meeting the protocol for the channel's constant. `poll_now`: polling keeps the unit's `wf()` (the device changes only through its mutex, by its poll handler). |
| `reply`'s two builds | `lib.rs` | The verification pass (`cfg(verus_keep_ghost_body)`) calls `complete` with the reply ghost; other builds make the same call with `Tracked::assume_new()` (the ghost is erased). |
| Error conversion | `lib.rs`: `anyhow_error`, `save_error`, `restore_error` | `external_body` with no contract (`RemoteError::new`); nothing assumed beyond termination. |
| Production `StateUnit` impls meet the contract | `ArcMutexChipsetDeviceUnit`, `ChipsetUnit`, `LineSetUnit` (`vmm_core/vmotherboard`); `KeeperUnit` (`vmtime_unit.rs`), the input distributor's `Inner` (`input_distributor.rs`), `PartitionUnitRunner` (`partition_unit.rs`), `&VmbusServerUnit`, `&ChannelUnit<T>`, `&SimpleChannelUnit<T>` (`vmbus_unit.rs`) (`vmm_core`); `TimeAbiUnit` (`openvmm_core`, `dispatch/time_abi.rs`) | Each method body is `external_body` until proved. `StateUnitSpec` is defined for `LineSetUnit`, `ChipsetUnit`, the input distributor, and `KeeperUnit` (each keeps its share in a ghost field), and `TimeAbiUnit` (its share in `TimeAbiMsrs`'s atomic); and `ArcMutexChipsetDeviceUnit` (its share in the device's lock, `DeviceSlot`); `ChannelUnit` and `SimpleChannelUnit` (proxies: their shares kept by the channels' tasks); still uninterpreted for `PartitionUnitRunner` and `VmbusServerUnit`. |
| vmbus channels (`vm/devices/vmbus/vmbus_channel`) | `channel.rs`, `simple.rs`, `channel.spec.rs`; `external_body` | `View for Device` (uninterpreted): `running` is `state is DeviceState::Running`. `GenericChannelHandle::id`, `ChannelHandle::id` (uninterpreted): the internal sender's constant. The handles' `start` / `stop` / `reset` / `save` / `restore`, the `offer_*` functions, and `Device::handle_state_request` meet their contracts. `Device::run_channel` (no contract yet) passes the task's share to `handle_state_request` and keeps `keeps_share`; the guest-traffic handlers change the device only while the task holds the whole share. OPEN: a new channel's task starts `Running` (design doc, "The vmbus units"). |
| vmbus channel's half reaches `unit_channel_for` | `vmm_core`'s three offer functions (`vmbus_unit.rs`), unverified | The half the `offer_*` function gives back is the one attached to the unit's sender. |
| `DeviceSlot::new`, `ArcMutexChipsetDeviceUnit::new` | `vm/vmcore/src/device_state.rs`; `vmotherboard`, `backing/arc_mutex/state_unit.rs`; `external_body` | `DeviceSlot::new` sets up the slot's share and gives back the other half (`Unknown`, id = the key's id); `ArcMutexChipsetDeviceUnit::new`'s contract (`wf()`, id = the lock's key's id, not running). |
| Chipset device's half and `omit` | `vmotherboard`'s device builder and the unit's spawn sites (`backing/arc_mutex/device.rs`, `services.rs`, `builder/mod.rs`), unverified | The half `DeviceSlot::new` gives back is the one attached to the device unit's sender (`unit_channel_for`), and the slot's `omit` is the unit's `omit_saved_state`. |
| Code that changes a device under its lock | vCPU I/O dispatch, line sets, the unit's poll (`vmotherboard`), and `DeviceSlot`'s `Deref` / `DerefMut`, unverified | Covered by "Unverified `lock`, `lock_if_open`, `close`": they keep the lock invariant, that is, they change a device only while its unit runs and keep the share in step. |
| Units' constructors | `LineSetUnit::new`, `ChipsetUnit::new`, `KeeperUnit::new` (`external_body`); `InputDistributor::run` (`external_body`) | They take the unit's half from its channel's receiver into the unit's ghost field (contracts in the code); `InputDistributor::new` puts a placeholder there, which `run` replaces before any request. |
| `VmTimeKeeper`'s view and methods | `vm/vmcore/src/vmtime.spec.rs` (view, uninterpreted), `vmtime.rs` (`save`, `restore`, `advance`, `reset`, `start`, `stop`: `external_body` with contracts) | The view is the keeper's running flag; `save`'s result has view `VmTimeKeeperView::saved_blob()`; the others change `running` as named. |
| **AXIOM** `vmtime::SavedState::axiom_all_time` | `vm/vmcore/src/vmtime_t.rs` | Any two vmtime saved states are equal except for time (every field is time), part of `equal_except_time`'s per-type trust. |
| `TimeAbiMsrs::new`, `load_control`, `store_control` | `vmm_core/virt/src/time_abi/msr.rs`, `external_body` | `new` sets up the MSR value's atomic with the share, and gives back the other half (`Unknown`, id = `key()`); the helpers load and store the value (their ghost updates are for the proof). |
| Time-ABI half reaches `unit_channel_for` | `openvmm_core`: `partition_config` (`time_abi.rs`) and the unit's spawn (`dispatch.rs`), unverified | The half `TimeAbiMsrs::new` gives back is the one attached to the time-ABI unit's sender (`unit_channel_for`). |
| `TimeAbiUnit::new` | `dispatch/time_abi.rs`, `external_body` | Its contract: `wf()`, `id()` is `msrs.key()`, not running. |
| `InspectMut::inspect_mut` (no contract) | `lib.proof.rs` declares `InspectMut` and `Inspect` without methods; `inspect_unit` (`lib.rs`, `external_body`) states the assumption for `apply` | An inspect keeps a unit's `wf()`, `id()`, and `running()` (the `Inspect` request path in `apply` and `run_async_unit`'s concurrent inspects). Open: give `inspect_mut` a contract. |
| Unverified `SaveRestore` implementations meet its contract | `vm/vmcore/src/save_restore.spec.rs` | The `SaveRestore` contract (on the device's view: `save` keeps it and returns a state with blob `blob_of(view)`; `restore` makes `blob_of(view)` the restored state's blob; both keep `device_wf()`) is checked only for implementations that are verified (with `SaveRestoreSpecImpl`; the shutdown and snapshot-request ports); all others are trusted to meet it, the PIT's through its `save_device` / `restore_device` contracts. For the 59 devices with the placeholder view `()` (`placeholder_view!`) the view part says nothing and `blob_of` is unspecified. |

## Assumed device facts

| Item | What is assumed |
|---|---|
| PIT entry points meet their contracts | `vm/devices/chipset/src/pit/mod.rs`: `start_device`, `reset_device`, `poll`, `pio_read`, `pio_write`, `save_device`, `restore_device` are `external_body` until proved. |
| PIT entry points' preconditions | Their callers (vCPU port I/O dispatch, the unit's poll loop, `ChangeDeviceState::start`) are unverified; they are trusted to call them only when their preconditions hold (`wf()`). (That they change the device only while its unit runs is the row "Code that changes a device under its lock".) |
| Stopped devices do not change | While the state units are stopped, no vCPU accesses a device: the partition unit (VPs) is stopped before the devices (stop order: dependants first) and stays stopped until `start`, and stopping it stops the vCPUs. (Not the `CloseableMutex`: a unit closes its device's mutex only during each operation.) Also, a stopped device has no activity of its own (its `stop` stops its timers and async work). Now stated through each device's share in its lock (design doc, "Chipset devices: the share in the device's lock"): a change needs the whole share, which the device's lock holds only while its unit runs; trusted until the code that changes devices under their locks is verified (row "Code that changes a device under its lock"). |

## Prototype only (`verus-patches/verus-rpc-prototype`, not part of OpenVMM)

`rpc_t.rs`, `rpc_async_t.rs`, `mesh_rpc_t.rs` (draft without attached
ghosts), `mutex_t.rs` (draft with a ghost predicate value): trusted wrappers
for the experiments, under the assumptions stated in each file's header.
