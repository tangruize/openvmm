# Snapshot spec: trusted base, deferred items, and assumptions to prove

Three parts (rule decided by the user, 2026-10-29):

1. **The trusted base**: only external things: the hypervisor, the guest,
   the hardware clock, protobuf, `mesh`, the locks' runtime
   (`parking_lot`), and the tools. These are trusted for good.
2. **Deferred**: what Verus cannot yet express (a Verus limit). Not
   trusted: each waits for a Verus patch or a workaround.
3. **Assumed until proved**: facts about our own code (OpenVMM's and
   ours): contracts whose bodies are not yet proved, and assumptions about
   unverified code. Not trusted: each is a TODO, with what would prove it.

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
`CloseableMutex`). Its unverified users are listed in Part 3 ("The locks' unverified users").

| Item | What is assumed |
|---|---|
| `Mutex::new_verified`, `acquire`, `MutexHandle::release` | The value satisfies `Pred::inv(m.param(), _)` while unlocked, for the mutex's own parameter (given to `new_verified`; since code change 94, in place of a key): `new_verified` and `release` require it, `acquire` provides it. `Mutex::new` (unverified code only): the parameter is unknown; assumed that the value satisfies the invariant for it (Part 3, "The locks' unverified users"). Runtime: `parking_lot::Mutex<Option<V>>`; a handle dropped without `release` leaves `None` and the next lock panics. |

## `CloseableMutex` (`support/closeable_mutex/src/closeable_mutex_t.rs`)

OpenVMM's lock for chipset devices, a lock primitive we do not verify (the
user's decision, 2026-10-30): its contracts below are trusted. The
unverified users are listed in Part 3 ("The locks' unverified users").

| Item | What is assumed |
|---|---|
| `CloseableMutex::new_verified`, `close_verified`, `VerifiedClosedGuard::borrow`, `borrow_mut`, `release` | While neither locked nor closed, the value satisfies its type's `LockInvariant::inv(param)` for the mutex's own parameter `param()` (given to `new_verified`; since code change 94, in place of a key): `new_verified` and `release` require it, `close_verified` provides it; `borrow_mut` gives the guard's value to change. Runtime: the real `CloseableMutex`; a `VerifiedClosedGuard` dropped without `release` leaves the mutex closed, so later `lock` / `close` panic. |
| `CloseableMutex::lock_verified`, `VerifiedOpenGuard::borrow_mut`, `release` | As for `close_verified`, for a lock that does not close the mutex. A `VerifiedOpenGuard` dropped without `release` leaves the mutex locked (later lockers wait forever). |

## The hypervisor and the guest

| Item | Where | What is trusted |
|---|---|---|
| A halt reason's encoding | `PartitionSavedView::encode_halt` (uninterpreted) | What `mesh::payload::encode` gives for a `HaltReason` (protobuf). |
| The backends' VM-state access (`AccessVmState`), since 2026-10-30 | `state_spec::StateAccessSpec` for `AccessVmState` (`vmm_core/virt/src/state.rs`, generated by `state_trait!`): the getters', setters', and `commit`'s contracts, `hv_state()`; implemented by `virt_kvm/src/arch/x86_64/vm_state.rs` and `virt_mshv/src/x86_64/vm_state.rs` | Trusted: the hypervisor holds each VM-state element; a getter returns its current value and changes nothing; a setter sets that element only; `commit` changes nothing. The backends' implementations are thin wrappers over the hypervisor's calls (MSR / register reads and writes through the boot VP; about 70 lines for KVM, 100 for MSHV), so trusted with the hypervisor (the user's decision, 2026-10-30). WHP and hvf out of scope. |
| The partition's state handles: the hypervisor's part (since code change 73) | `virt::open_vm_state`, `virt::close_vm_state`, `new_vm_state_token`'s value (`vmm_core/virt/src/generic/generic_t.rs`); a handle's `hv_id()` | All handles of a partition's VM state (VTL 0) see one state, the hypervisor's: a handle opened with the partition's token starts with the token's value, and closing it gives the token back with the handle's final state; a new token's value is the partition's current state. |
| virtio-MMIO's interrupt status: the guest's part (the guest-memory boundary) | the guest | The status the guest sees: in the microVM's shared-status mode, a word in guest memory that the device sets and the guest clears directly. Trusted: only the guest (while its VPs run) changes it, besides the device's own interrupts (while the device runs). No contract states this yet: the guest-memory access contracts it needs are to write (with the proof of the transport's functions, Part 3). |

# Part 2. Deferred (Verus limits; not trusted)

Each item is marked in the code with `PROOF DEFERRED: do not attempt to
prove this until ...`, naming what it waits for (since 2026-10-30).

| Item | Where | What is assumed until Verus supports it |
|---|---|---|
| A device boxed into `dyn DynChipsetDevice` keeps its blob | `From<T> for ResolvedChipsetDevice` (`vm/chipset_device_resources/src/lib.rs`, `external_body`) | The erased device's `saved_blob()` and `device_wf()` are the device's (Rust's `dyn` dispatch calls the device's impl). Verus cannot prove it while `DynChipsetDevice` is `async_trait` (gap G5); to remove with the G5 Verus patch. |
| `ArcMutexChipsetDeviceUnit::next_event` | `vmotherboard`, `backing/arc_mutex/state_unit.rs`, `external_body` | DEFERRED (Verus does not support `select!`): as `recv_msg`, a request comes with a half meeting the protocol for the channel's constant. |
| `ArcMutexChipsetDeviceBuilder::try_add`, `try_add_async` (not marked) | `vmotherboard`, `chipset/backing/arc_mutex/device.rs` | DEFERRED: their bounds name `dyn std::error::Error + Send + Sync` (a `dyn` of more than one trait); they lock no mutex themselves but call `inner_add`, which is marked. |
| `std::sync::Weak<T>` (the PCI paths' lock users, not marked) | `base_chipset.rs` (`weak_mutex_pci`), `dispatch.rs` (`WeakMutexPciBusDevice`) | DEFERRED (and not on the microVM's path): this vstd cannot declare `Weak<T, A: Allocator>` without the unstable `allocator_api`; code change 77. |

# Part 3. Assumed until proved (our code; not trusted)

## Contracts whose bodies are not yet proved

| Item | Where | What is assumed |
|---|---|---|
| `LoadedVm::save` | `openvmm/openvmm_core/src/worker/dispatch.rs` | Body is `external_body`: its contract (see design doc, "Contracts") is assumed, not proved. |
| `LoadedVm::restore` | same | Same. |
| `StateUnits::stop`, `reset`, `save`, `restore` | `vmm_core/state_unit/src/lib.rs` | Same (`external_body`; contracts in the design doc, "State-unit contracts"). |
| `StateUnits::inventory`, `StateUnits::validate_inventory` | `vmm_core/state_unit/src/inventory.rs` | Same. |
| `validate_inventory` (free function) | `openvmm/openvmm_core/src/worker/dispatch/restore.rs` | Same. |
| `LoadedVm::quiesce_for_snapshot`, `resume_after_failed_snapshot`, `establish_snapshot_boundary`, `release_snapshot_boundary`, `handle_snapshot_boundary` | `openvmm/openvmm_core/src/worker/dispatch/microvm.rs` | Same (design doc, "Quiesce and save"). |
| `StateUnits::resume_after_failed_save` | `vmm_core/state_unit/src/quiesce.rs` | Same. (`QuiesceForSaveError::has_uncertain_state` is verified since 2026-10-30: the error's view is defined from its `uncertain` field.) |
| Lending the state units' halves (since code change 89): `StateUnits::lend_halves`, `return_halves`; `LoadedVm::lend_unit_halves`, `return_unit_halves` (Verus only) | `vmm_core/state_unit/src/quiesce.rs`; `openvmm/openvmm_core/src/worker/dispatch/microvm.rs` | Same (`external_body`, PROOF TODO; design doc, "The VM worker's request protocol"). |
| `LoadedVm::pause` (since code change 90) | `openvmm/openvmm_core/src/worker/dispatch.rs` | Same (design doc, "Roadmap", step 1). |
| `LoadedVm::quiesce_and_save`, `LoadedVm::capture_time_abi` | `openvmm/openvmm_core/src/worker/dispatch/microvm.rs` | Same (design doc, "Quiesce and save"). `capture_time_abi`'s contract (the view is unchanged) is a claim about the VPs' timer check and the partition's time records. |
| `StateUnits::quiesce_for_save` | `vmm_core/state_unit/src/quiesce.rs` | Same. Its `Stop` requests go through the unverified `state_change` path. |
| `LoadedVm::restore_from_snapshot`, `begin_snapshot_restore`, `finish_snapshot_restore` | `openvmm/openvmm_core/src/worker/dispatch/restore.rs` | Same (design doc, "Restore from snapshot"). |
| `InitializedVm::load` | `openvmm/openvmm_core/src/worker/dispatch.rs` | Same (design doc, "Restore from snapshot"). Includes the fresh-VM facts: every unit's half collected, recording `Unknown`. |
| `LoadedVm::time_abi_restore` | `openvmm/openvmm_core/src/worker/dispatch/time_abi.rs` | Same. Claims the clock restore changes only time (so nothing in the view). |
| `StateUnits::advance_time` | `vmm_core/state_unit/src/advance_time.rs` | Same. |
| `Inner::ready_set` (since 2026-10-30; was an `assume_specification`) | `vmm_core/state_unit/src/lib.rs` | Same (`external_body`): with `unit_ids` `None`, the ready set holds every registered unit. Its body is an iterator chain with closures. |
| `StateRequest::apply` (since 2026-10-29) | `vmm_core/state_unit/src/lib.rs` | Same (`external_body`; it was verified with an `assume` in its `Stop` arm, "a unit receives `Stop` only while it runs"; made `external_body` instead, by the user's decision, for consistency with the other functions still to prove). To prove: needs the running token ("Recorded unit states are real", below). |
| `VpSet`'s methods (since code changes 65, 67, 68) | `vmm_core/src/partition_unit/vp_set.rs` (`new`, `add`, `start`, `halt`, `clear_halt`, `stop`, `reset`, `scrub`, `save`, `restore`, `restore_halt`, `set_initial_regs`), `vp_set/boundary.rs` (`stop_at_io_boundary`), `vp_set/time_abi.rs` (`check_one_shot_timers`, `advance_lapic_timers`) | Same (`external_body`; contracts in the design doc, "The VP protocol"). Their bodies pass placeholder ghosts to the RPC wrapper (code change 71), to replace with `VpSet`'s halves when proved. `validate_restore_halt`, `teardown`, `get_dump_vp_state` are `external_body` without contracts. |
| `PartitionUnitRunner::run` | `vmm_core/src/partition_unit.rs` | Same: it keeps the runner's `wf()` (since 2026-10-30 including the stop tokens: one outstanding token per stop reference, VPs stopped while any is outstanding) and `id()`; it answers `StopVps` and `StopVpsAtIoBoundary` with a new stop token, and takes one back with `StartVps` (`handle_snapshot`, `stop_vps`, `resume_vps`; placeholders marked PROOF TODO). (Its other events are deferred until after G5; design doc, "Other open items".) |
| The partition's state, above the handles (since code change 73) | `openvmm/openvmm_core/src/partition.rs`: the blanket `BasicPartitionStateAccess` impl's `save` / `restore` (bodies `external_body`), `WrappedPartition`'s `SaveRestore` `save` / `restore` (bodies `external_body`, the trait's contract) | Same (design doc, "The partition's state"). The blanket impl's `reset`, `scrub_vtl`, `accept_initial_pages`, `guest_os_id` are `external_body` without contracts (TODO there). |
| `unit_channel_for` | `vmm_core/state_unit/src/unit_channel.rs` (`external_body`) | Its contract: the given half is attached to the new channel's sender, none to its receiver, and both ends' constant is the half's id. Its body is `mesh::channel()`; to prove through `mesh_rpc_verus::channel`. |

## Lemmas not yet proved

| Lemma | Where | Statement |
|---|---|---|
| `StateUnits::lemma_view_inv` | `vmm_core/state_unit/src/lib.proof.rs` | If `StateUnits` holds a half for exactly the collected units and those have distinct names, `self@.inv()` holds (the inventory, built from the collected ids by `to_sorted_seq`, lists each unit once). |

## Facts about our code

| Item | Where | What is assumed |
|---|---|---|
| `assume_specification` for `RestoreError: From<payload::Error>` | `vm/vmcore/src/save_restore.spec.rs`, `save_restore_t.rs` | No contract (nothing assumed beyond termination). |
| Unverified `ProtobufSaveRestore` implementations meet its contract | `vm/vmcore/src/save_restore.spec.rs`, `save_restore_t.rs` | `ChipsetDeviceProxy` (`workers/chipset_device_worker`), `BoundVp` (`vmm_core/src/partition_unit/vp_set.rs`), `ErasedChipsetDevice` (`vm/chipset_device_resources`): `save` returns `saved_blob()` and keeps it; `restore` sets it. (The blanket impl for `SaveRestore` types is verified.) |
| The backends' VP-state access (`AccessVpState`) | `state_spec::StateAccessSpec` for `AccessVpState` (`vmm_core/virt/src/state.rs`, generated by `state_trait!`): the getters', setters', and `commit`'s contracts | ASSUMED, to prove: obligations of each backend's implementation, which has significant code (KVM's `vp_state.rs`, about 700 lines; MSHV's, about 550). PROOF DEFERRED until gap G4 (itself deferred until after G5); then against trusted contracts on the hypervisor's calls, in a `_t.rs` file per backend. KVM and MSHV in scope; WHP and hvf out of scope (the user, 2026-10-30). |
| The locks' unverified users (since 2026-10-30) | `vmm_core/vmotherboard`, `vmm_core/state_unit` | Functions that lock a device's `CloseableMutex` or the registry's `Mutex`, or create one, are `external_body` without a contract, to prove like any other function: once verified (through `lock_verified` / `acquire` and `release`), Verus checks the lock's invariant at each release, and at creation (`new_verified`). Marked: `Chipset`'s `mmio_read`, `mmio_write`, `io_read`, `io_write`, `acknowledge_pic_interrupt`, `handle_eoi` (the guest's I/O dispatch, which calls the devices' handlers, e.g. the PIT's `pio_read` / `pio_write`, whose preconditions it must then meet); `IoRanges::lookup` (creates the missing-device mutex); `LineSetTargetDevice::set_irq`; `ArcMutexChipsetServices::finalize`; `ArcMutexChipsetDeviceUnit::inspect_mut`, `poll_now`; `UnitHandle::remove_if`; `Inspect` for `StateUnits` and `StateUnitsInspector`; `StateUnits::run_op`, `run_input_op`, `start_with_rollback`. Also marked, after code change 76 (`mut self` became `self`): `UnitBuilder::build` (registration), `ArcMutexChipsetDeviceBuilder::inner_add`, `add`, `add_async` (they create each device's mutex). Deferred (a Verus limit, Part 2): `try_add`, `try_add_async`. Skipped, may be revisited (code change 77): the PCI and PCIe paths (`base_chipset.rs`'s `weak_mutex_pci`, `dispatch.rs`'s `WeakMutexPciBusDevice` and root-complex / switch operations), `ChipsetDevices::add_dyn_device` (hotplug, `vpci`), `vmm_core`'s `device_builder.rs` (`vpci`): not on the microVM's path, and the PCI wrappers need `Weak<T>`, which this vstd cannot declare. Out of scope: OpenHCL, the fuzzer, tests. (Replaces the rows that assumed these users keep the invariants: decided by the user, 2026-10-30.) |
| No registration during a `&mut StateUnits` operation | `lib.rs`, for future proofs of `run_op` and the operations built on it | While an operation holds `&mut StateUnits`, no unit is registered: registration (`UnitBuilder::build`) needs `&StateUnits`. Removal (`UnitHandle`'s `Drop`) can happen; the operations' specs must allow it. (Replaces the two registry axioms, removed 2026-10-27.) |
| Initial ghost state | `StateUnits::new` (unverified) | It builds the registry with `new_ghosts()`'s two linked halves, and the mutex with that value, so `StateUnits::wf()` holds. |
| Units collected before use | `make_ids` (`dispatch.rs`); `StateUnitsView` | `make_ids` collects the halves of all units registered while loading (Verus-only call). ASSUMPTION: a unit registered later (hot-add) is collected before the next `stop`, `reset`, `save`, or `restore`, so `StateUnits`' view covers every registered unit. |
| **Recorded unit states are real** | `lib.proof.rs`, `ExampleUnit::handle` precondition | ASSUMED, to prove: a unit receives `Stop` only while it is running, that is, `StateUnits`' registry records a unit `Running` only when it is (`run_op` sends `Stop` only to units recorded `Running`). Provable with a running token shared by `StateUnits` and each unit (given with `Start`, taken back with `Stop`). `StateRequest::apply`, which needed it for `Stop`, is `external_body` since 2026-10-29 ("Contracts whose bodies are not yet proved"). See "`StateUnits`' unit states are the units' real states" in the design doc. |
| `Inspect` requests | `lib.proof.rs`, request protocol | ASSUMED, to prove: a read-only inspect request does not change a unit's saved state (it reaches the unit through `InspectMut`, with no ghost). Inspect writes are out of scope (the user, 2026-10-30; design doc, "Out of scope for now"): the partition unit's `"clear_halt"` is one that does change it. |
| No channel failure for a registered unit | request protocol | A registered unit's task does not exit: OpenVMM's `run_op` panics on such a failure, and the unit's half would be lost. |
| VP runners are never dropped while their `VpSet` is in use (assumed by the user's decision, C1) | the VP protocol (`vmm_core/src/partition_unit/vp_set.spec.rs`); relied on by the contracts of `VpSet::stop`, `stop_at_io_boundary`, `save`, `restore`, `reset`, `scrub`, `set_initial_regs` (`vp_set.rs`) | So no `VpEvent` call fails with a channel error, and `VpSet` never loses a VP's half (a failed call returns no ghost). `VpSet` itself tolerates a dropped runner (`stop` ignores it), but in OpenVMM's runs it does not happen: a runner's `run` ends only after `VpSet` is gone; a cancelled run is reissued; a VP that fails to bind fails the VM's build, so its `VpSet` is never used; a panic aborts the process (`panic = 'abort'`). Exception: OpenVMM's own test `failed_boundary_tolerates_dropped_runner` (`vp_set/boundary.rs`) drops one deliberately; the spec does not cover it. Decided by the user, 2026-10-28 (design doc, "The VP protocol", C1). |
| Unit side follows the protocol | request protocol | `run_unit` and `run_unit_step` are verified against `StateUnit`'s contract (`lib.rs`), given `StateRequest::apply`'s contract (its body is `external_body` since 2026-10-29). So are `handle_request` and `ArcMutexChipsetDeviceUnit::run` (`vmotherboard`), and the chipset unit and the line sets now run through `run_unit`. Still assumed: units whose requests do not go through them answer by `request_ok`: the vmbus units (`run_async_unit` / `apply_with_concurrent_inspects`, `external_body` with the contracts of `run_unit` / `apply`; the units still pass no halves to their owning tasks) and the input distributor (its own loop calls `apply` without the half). |
| `run_unit`'s and `ArcMutexChipsetDeviceUnit::run`'s callers | `vmtime_unit.rs` (`KeeperUnit`), `dispatch.rs` (`TimeAbiUnit`), `vmotherboard` (`builder/mod.rs` for the chipset unit, `add_dyn_device`; `line_sets.rs`; `backing/arc_mutex/services.rs`), unverified | They pass a unit with `wf()` and its own channel's receiver (`recv.constant() == unit.id()`). |
| Production `StateUnit` impls meet the contract | `ArcMutexChipsetDeviceUnit`, `ChipsetUnit`, `LineSetUnit` (`vmm_core/vmotherboard`); `KeeperUnit` (`vmtime_unit.rs`), the input distributor's `Inner` (`input_distributor.rs`), `PartitionUnitRunner` (`partition_unit.rs`), `&VmbusServerUnit`, `&ChannelUnit<T>`, `&SimpleChannelUnit<T>` (`vmbus_unit.rs`) (`vmm_core`); `TimeAbiUnit` (`openvmm_core`, `dispatch/time_abi.rs`) | Each method body is `external_body` until proved. `StateUnitSpec` is defined for `LineSetUnit`, `ChipsetUnit`, the input distributor, and `KeeperUnit` (each keeps its share in a ghost field), and `TimeAbiUnit` (its share in `TimeAbiMsrs`'s atomic); and `ArcMutexChipsetDeviceUnit` (its share in the device's lock, `DeviceSlot`); `ChannelUnit` and `SimpleChannelUnit` (proxies: their shares kept by the channels' tasks); `PartitionUnitRunner` (its share in a ghost field, its snapshot its saved state's blob; see below); still uninterpreted for `VmbusServerUnit`. |
| `DeviceSlot::new`, `ArcMutexChipsetDeviceUnit::new` | `vm/vmcore/src/device_state.rs`; `vmotherboard`, `backing/arc_mutex/state_unit.rs`; `external_body` | `DeviceSlot::new` sets up the slot's share and gives back the other half (`Unknown`, id = the slot's `share_id()`); `ArcMutexChipsetDeviceUnit::new`'s contract (`wf()`, id = the id in the lock's parameter, not running). |
| Chipset device's half and `omit` | `vmotherboard`'s device builder and the unit's spawn sites (`backing/arc_mutex/device.rs`, `services.rs`, `builder/mod.rs`), unverified | The half `DeviceSlot::new` gives back is the one attached to the device unit's sender (`unit_channel_for`), and the slot's `omit` is the unit's `omit_saved_state`. |
| Units' constructors | `LineSetUnit::new`, `ChipsetUnit::new`, `KeeperUnit::new` (`external_body`); `InputDistributor::run` (`external_body`) | They take the unit's half from its channel's receiver into the unit's ghost field (contracts in the code); `Inner::new` (verified, code change 57) starts the input distributor's share as `None`, with no placeholder; `run` takes the half from the channel before any request. |
| `VmTimeKeeper`'s view and methods | `vm/vmcore/src/vmtime.spec.rs` (view, **uninterpreted, by the user's decision, 2026-10-30**), `vmtime.rs` (`save`, `restore`, `advance`, `reset`, `start`, `stop`: `external_body` with contracts) | The view is the keeper's running flag (meant: `time` is `Started`); `save`'s result has view `VmTimeKeeperView::saved_blob()`; the others change `running` as named. Left uninterpreted: defining it from the fields needs every field type declared in `vmcore`, which needs two new `vmcore` dependencies used only for Verus (`async-task`, for `Task<()>`, whose declaration would move from `virtio`; `mesh_rpc_verus`, for `mesh::Sender`); and guest time is out of scope. Consequence: the keeper's own methods cannot be verified against these contracts, which stay assumptions; the vmtime unit and its other users are still verified against them. May be revisited. |
| `TimeAbiMsrs::new`, `load_control`, `store_control` | `vmm_core/virt/src/time_abi/msr.rs`, `external_body` | `new` sets up the MSR value's atomic with the share, and gives back the other half (`Unknown`, id = `key()`); the helpers load and store the value (their ghost updates are for the proof). |
| Time-ABI half reaches `unit_channel_for` | `openvmm_core`: `partition_config` (`time_abi.rs`) and the unit's spawn (`dispatch.rs`), unverified | The half `TimeAbiMsrs::new` gives back is the one attached to the time-ABI unit's sender (`unit_channel_for`). |
| `TimeAbiUnit::new` | `dispatch/time_abi.rs`, `external_body` | Its contract: `wf()`, `id()` is `msrs.key()`, not running. |
| `InspectMut::inspect_mut` (no contract) | `lib.proof.rs` declares `InspectMut` and `Inspect` without methods; `inspect_unit` (`lib.rs`, `external_body`) states the assumption for `apply` | An inspect keeps a unit's `wf()`, `id()`, and `running()` (the `Inspect` request path in `apply` and `run_async_unit`'s concurrent inspects). Open: give `inspect_mut` a contract. Read-only inspects only: inspect writes are out of scope (2026-10-30). |
| `save_all` / `restore_all` meet their contracts | `state_spec::StateAccessSpec` (`state_trait!`, `vmm_core/virt/src/state.rs`) | The generated default methods: `save_all` returns the handle's state and changes nothing; `restore_all(s)` leaves the state overwritten by `s`'s present elements. Stated on the trait's specification, so assumed; to prove from the getters' and setters' contracts (needs the generated trait to be `verus_verify`). |
| The partition runner's share at construction | `PartitionUnitRunner::new` (`external_body`) | Its contract: the runner takes the half attached to its channel's receiver. Its body is unproved (it makes the share with `Tracked::assume_new()`; to replace by taking the half from the channel, as in `KeeperUnit::new`). Since code change 56 the runner is built inside its task, and the order (build, then run) is verified in `run_partition_unit`. |
| Unverified `SaveRestore` implementations meet its contract | `vm/vmcore/src/save_restore.spec.rs` | The `SaveRestore` contract (on views: `save` keeps the device's view and returns a state whose view is `saved_view(view)`; `restore` makes `saved_view(view)` the restored state's view; both keep `device_wf()`) is checked only for implementations that are verified (with `SaveRestoreSpecImpl`): the shutdown and snapshot-request ports; the PIT's, PIC's, RTC's, portb's, IOAPIC's, and virtio-MMIO's are attached to their impls (`verus_verify`) with `external_body` bodies, to prove (the PIT's from its `save_device` / `restore_device` contracts). All others are trusted to meet it; for the devices with placeholder views (`placeholder_view!`) it says nothing (view `()`, `saved_view` unspecified). |
| PIT entry points meet their contracts |  | `vm/devices/chipset/src/pit/mod.rs`: `start_device`, `reset_device`, `poll`, `pio_read`, `pio_write`, `save_device`, `restore_device` are `external_body` until proved. |
| Stopped devices do not change |  | While the state units are stopped, no vCPU accesses a device: the partition unit (VPs) is stopped before the devices (stop order: dependants first) and stays stopped until `start`, and stopping it stops the vCPUs. (Not the `CloseableMutex`: a unit closes its device's mutex only during each operation.) Also, a stopped device has no activity of its own (its `stop` stops its timers and async work). Now stated through each device's share in its lock (design doc, "Chipset devices: the share in the device's lock"): a change needs the whole share, which the device's lock holds only while its unit runs; trusted until the code that changes devices under their locks is verified (row "Code that changes a device under its lock"). |
| The partition's state handles: our code's part (since code change 73) | `new_vm_state_token` (called once per partition, in `into_vm_partition`); `guest_os_id` and the backends' own handles; `close_vm_state`'s callers | ASSUMED, to prove: the token is created once per partition; other handles only read; `close_vm_state` is given only handles from `open_vm_state`; no one else changes the partition's VM state while the token is in use, except as listed under "Open" in the design doc ("The partition's state": the guest while its VPs run, `reset`, `scrub_vtl`, `accept_initial_pages`). |
| virtio-MMIO's interrupt status: our code's part | `transport/mmio.rs` (`MmioTransport::ghost_status`), `transport/mmio/interrupt.rs` | To prove (since 2026-10-30, the user). The device's view reads the transport's Verus-only ghost field `ghost_status` (the uninterpreted `interrupt_status()` is removed). `read_interrupt_status` / `restore_interrupt_status` are `external_body` without contracts: PROOF TODO, specify and prove them (read returns `ghost_status` while stopped; restore sets the status and `ghost_status`), keeping `ghost_status` in step with the real status (restore, reset, the device's interrupts, a trusted refresh after the guest has run); needs trusted guest-memory access contracts (to write) and a lock invariant on `InterruptState`. Its initial value is a placeholder (PROOF TODO). Legacy modes (not the microVM's): their lock protocol is a TODO (design doc). |
| Ends from `channel` | `support/mesh/mesh_rpc_verus/src/mesh_rpc_t.rs` | ASSUMPTION: an end whose ghost is taken was created by `channel`, not by `mesh::channel` directly (whose `attached()` is unknown). |

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
| Protocols of `VmRpc` and `PartitionRequest` | None assumed. `PartitionRequest`'s protocol is written since 2026-10-30 (design doc, "The partition unit's request protocol"): stop tokens; the time-ABI and boot requests cannot be sent by verified code. `VmRpc`'s protocol is written since code change 89 (design doc, "The VM worker's request protocol"): the snapshot requests move the state units' halves, `Pause` (code change 90) moves nothing, and the other 18 requests cannot be sent by verified code. |
| Normal-build blanket impl | In a non-Verus build every type implements `MeshProtocol` with `()` ghosts (the trait's spec function is erased). No effect on verification. |
| Normal-build blanket impl | In a non-Verus build every type implements `MutexPredicate` (with `Param = ()`). No effect on verification. |
| Normal-build blanket impl | In a non-Verus build every type implements `LockInvariant` (with `Param = ()`). No effect on verification. |
| `reply`'s two builds | `lib.rs`: The verification pass (`cfg(verus_keep_ghost_body)`) calls `complete` with the reply ghost; other builds make the same call with `Tracked::assume_new()` (the ghost is erased). |
| Error conversion | `lib.rs`: `anyhow_error`, `save_error`, `restore_error`: `external_body` with no contract (`RemoteError::new`); nothing assumed beyond termination. |

## Prototype only (`verus-patches/verus-rpc-prototype`, not part of OpenVMM)

`rpc_t.rs`, `rpc_async_t.rs`, `mesh_rpc_t.rs` (draft without attached
ghosts), `mutex_t.rs` (draft with a ghost predicate value): trusted wrappers
for the experiments, under the assumptions stated in each file's header.
