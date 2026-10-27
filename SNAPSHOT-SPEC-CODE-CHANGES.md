# Snapshot spec: code changes

Code changes made for the snapshot save/restore verification, to OpenVMM and
to Verus. Design rationale is in `SNAPSHOT-SPEC-DESIGN.md`; the trusted base
is in `SNAPSHOT-SPEC-TRUSTED.md`.

Every OpenVMM change must keep the code compiling and behaving the same in a
normal (non-Verus) build. Ghost (`Ghost`/`Tracked`) arguments are zero-sized
and erased.

## OpenVMM

Baseline: upstream `nanvix/openvmm` commit
`7f225e2cd635bbd1e1839e587e659996433396c8`. See
`git diff 7f225e2 HEAD -- . ':!*.md' ':!verus-patches' ':!*.txt'`.

### Made

1. **Extract `LoadedVm::restore_from_snapshot`** (behavior-preserving).
   `openvmm/openvmm_core/src/worker/dispatch/restore.rs` gains
   `restore_from_snapshot` (`begin_snapshot_restore`, `restore`,
   `finish_snapshot_restore`); `dispatch.rs` (`make_ids`, about line 3505)
   calls it instead of the three steps inline. Gives the restore side a
   single function to specify.
2. **Verus wiring** for `openvmm_defs`, `openvmm_core`, and `openvmm_entry`:
   - workspace `Cargo.toml`: `vstd = "=0.0.0-2026-10-04-0306"` with the `std`
     feature (needed for async);
   - each crate's `Cargo.toml`: `vstd.workspace = true` and
     `[package.metadata.verus] verify = true`;
   - each crate's `build.rs`: declares the `verus_keep_ghost` cfg
     (`openvmm_core` also `verus_verify_core`; `openvmm_defs` gets a new
     `build.rs`);
   - each crate root: a Verus-only `use vstd::prelude::*;`
     (`openvmm_core` also enables `proc_macro_hygiene` under Verus);
   - `Cargo.lock` updated for `vstd`.
3. **`scripts/verify-snapshot-spec.sh`**: runs `cargo verus verify` over the
   spec crates.
4. **Verus wiring for `vmcore`** (`vm/vmcore`), as for the crates above:
   `vstd.workspace = true` and `[package.metadata.verus] verify = true` in
   `Cargo.toml`, a new `build.rs` declaring `verus_keep_ghost`, and a
   Verus-only `use vstd::prelude::*;` in `src/lib.rs`. Needed because the
   `View` impl for `SavedStateBlob` must live in the crate that defines it.
5. **Spec files included under `#[cfg(verus_keep_ghost)]`** (no effect on a
   normal build):
   - `vm/vmcore/src/save_restore.rs`: `pub mod spec` including
     `save_restore.spec.rs`;
   - `openvmm/openvmm_defs/src/worker.rs`: `pub mod spec` including
     `worker.spec.rs`, and `mod proof` including `worker.proof.rs`;
   - `openvmm/openvmm_core/src/worker/dispatch.rs`: `mod spec` and
     `mod proof` including `dispatch.spec.rs` and `dispatch.proof.rs`, and a
     Verus-only `use vstd::prelude::*;`.
6. **Contracts on `LoadedVm::save` and `LoadedVm::restore`**
   (`dispatch.rs`): `verus_spec` attributes under
   `cfg_attr(verus_keep_ghost, ...)`, with `verus_verify(external_body)` for
   now. No code change.
7. **`LoadedVm` known to Verus** (`dispatch.rs`):
   `#[cfg_attr(verus_keep_ghost, verus_verify)]` on the struct, so that its
   view can read `state_units`. (An interim ghost field `LoadedVm::halves`
   was added and later removed; the halves are in `StateUnits`, item 13.)
8. **Verus wiring for `state_unit`** (`vmm_core/state_unit`), as for the
   crates above (`Cargo.toml`, new `build.rs`, Verus-only crate-root import),
   plus `mod proof` including `lib.proof.rs`.
9. **`StateUnits`' registry mutex** (`vmm_core/state_unit/src/lib.rs`):
   `Arc<parking_lot::Mutex<Inner>>` becomes `Arc<InnerMutex>`, where
   `InnerMutex = mutex_t::Mutex<Inner, InnerPred>`, the trusted wrapper in
   the new file `src/mutex_t.rs` (moved from the prototype). It wraps
   `parking_lot::Mutex<Option<Inner>>`, so the runtime lock is the same;
   every `.lock()` call site is unchanged. `InnerPred` (an empty enum) names
   the lock invariant, which `lib.proof.rs` defines (trivial for now). Same
   change for `UnitHandle::inner` and `StateUnitsInspector::inner`.
10. **New file `vmm_core/state_unit/src/mesh_rpc_t.rs`** (trusted): the mesh
    RPC wrappers from the prototype, plus ghosts attached to a channel's ends
    (`channel(k, gs, gr)`, `take_sender_ghost`, `take_receiver_ghost`;
    `clone_sender` carries none); `recv` is renamed `recv_msg`. Not yet used
    by production code. Has a runtime test (`call_round_trip`). In a normal
    build a blanket impl satisfies `MeshProtocol`, with `()` ghosts.
11. **`state_unit::unit_channel()`** (new file
    `vmm_core/state_unit/src/unit_channel.rs`, verified, in `verus!`): creates
    a state unit's channel and, in Verus builds, its FracGhost (value
    `UnitSnapshot::Unknown`), one half attached to each end. At runtime it is
    `mesh::channel()`. Callers switched from `mesh::channel()`:
    `UnitBuilder::spawn` (`vmm_core/state_unit/src/lib.rs`) and
    `ChipsetBuilder::new` (`vmm_core/vmotherboard/src/chipset/builder/mod.rs`).
12. **State-unit spec files** in `vmm_core/state_unit/src`: `lib.spec.rs`
    (`UnitSnapshot`, included as `pub mod spec`) and, in `lib.proof.rs`, the
    `MeshProtocol` impl for `StateRequest` (ghost types; request rules still
    to come).
13. **`StateUnits` holds the halves** (`vmm_core/state_unit/src/lib.rs`):
    `#[cfg_attr(verus_keep_ghost, verus_verify)]` on `StateUnits` and a
    Verus-only field `halves` (initialized in `new` by `proof::no_halves()`).
    `lib.proof.rs` gains the lock invariant on registered senders, the
    Verus-only `StateUnits::collect_halves`, and declarations of `Inner`,
    `Unit`, `State`, and `inspect::SensitivityLevel`; `lib.spec.rs` gains
    `StateUnitsView` and `View for StateUnits`. No runtime change.
14. **`LoadedVm::save` precondition** `snapshot_recorded()`
    (`dispatch.rs`, defined in `dispatch.spec.rs`).
15. **Request protocol** (`vmm_core/state_unit/src/lib.proof.rs`, Verus
    only): `request_ok` for `StateRequest`, helper predicates on
    `StateRequest`, `StateRequest` made transparent to Verus, declarations of
    `RemoteError` and `inspect::Deferred`, and two verified examples
    (`example_save_call`, `ExampleUnit`). `vm/vmcore/src/save_restore.spec.rs`
    gains a trusted `assume_specification` for `SavedStateBlob::clone`. No
    runtime change.
16. **Behavior-preserving restructuring of `run_op` and its helpers**
    (`vmm_core/state_unit/src/lib.rs`), so that Verus can verify them
    (Verus does not support `async` blocks, or `BTreeMap::iter_mut` and
    `HashMap::get_key_value`):
    - `state_change` is an `async fn` taking the unit's `Sender` (callers pass
      `unit.send.clone()`, cloned when the call is made, as before) instead of
      a plain `fn` returning an `async move` block; its inner instrumented
      block is the new `async fn call_unit`. Callers updated in `run_op`,
      `quiesce.rs` (3), and `start.rs` (1).
    - In `run_op`, the per-unit `async move` block is the new `async fn
      wait_then_change` (wait for dependencies, run the state change, mark
      done), and the block around `join_all` is the new `async fn
      join_all_timed`; both keep their tracing.
    - In `run_op`, the two loops over `inner.units.iter_mut().filter(...)`
      iterate the keys and use `get_mut` (same order), and the result
      `filter_map(...).collect()` is a `for` loop with `push`.
    - In `restore`, `names.get_key_value(...)` is `names.get(...)` with the
      name read from the unit, the same `Arc` (`build` stores one `Arc` in
      both).
    Checks: `cargo test -p state_unit` (10 passed), `cargo check -p
    openvmm_entry`.
17. **`StateUnits`' ghost state keyed by unit id, with the registry axiom**
    (Verus only, no runtime change): the Verus-only field of `StateUnits` is
    now `units_ghost: Tracked<proof::UnitHalves>` (halves by unit id, and
    the collected units' names and ghost ids), replacing `halves`. New file
    `vmm_core/state_unit/src/registry_t.rs` (trusted, Verus-only module):
    `StateUnits::lock_registry`, the registry axiom. `lib.proof.rs`:
    `UnitHalves`, `StateUnits::wf`, `collected`, `agrees_with`; the lock
    invariant adds distinct names; `collect_halves` uses `lock_registry`.
    `LoadedVm::snapshot_recorded` (`dispatch.spec.rs`) also requires
    `state_units.wf()`.
18. **More registry specs** (Verus only, no runtime change):
    `registry_t.rs` gains `lock_registry_exact` (the exact-registry axiom)
    and assumed specifications for `State`'s equality, `ReadySet`'s clone,
    and `Inner::ready_set`; `lib.proof.rs` declares `ReadySet`,
    `ReadyState`, `UnitRecvError`, `StateTransitionError`, and
    `SavedStateUnit` (moved here from `openvmm_defs/src/worker.proof.rs`,
    since `state_unit` is now built by Verus), and adds `ReadySet::ids`,
    `Inner::unit_ids`, and `StateUnits::agrees_exactly`. `state_unit`'s
    `#![forbid(unsafe_code)]` becomes `deny` under Verus only, with
    `registry_t` allowed (Verus expands `assume_specification` to `unsafe`
    declarations); normal builds still `forbid`.

    (An attempt to verify `run_op`'s ghost flow, which moved it into
    `verus!`-style `ops.rs` with trusted helpers, is saved on branch
    `system-proof/run-op-proof-attempt`, commit `b5c333c5a`; it is not
    merged.)
19. **State-unit contracts** (no runtime change): `verus_spec` contracts
    with `verus_verify(external_body)` on `StateUnits::stop`, `reset`,
    `save`, `restore` (`vmm_core/state_unit/src/lib.rs`), `inventory` and
    `validate_inventory` (`inventory.rs`), and on `validate_inventory` in
    `openvmm/openvmm_core/src/worker/dispatch/restore.rs`. `lib.spec.rs`
    defines the view's `inventory` (collected units' names in id order) and
    adds `StateUnitsView::all_recorded`, `saved_units`, and
    `SavedStateUnit::map_of`, `distinct_names`; `SavedState`'s view
    (`openvmm_defs/src/worker.spec.rs`) uses `map_of`. `LoadedVm`'s view uses
    `saved_units`; `LoadedVm::units_consistent` is new, and `restore`
    requires it. `make_ids` (`dispatch.rs`) calls
    `state_units.collect_halves()` under `#[cfg(verus_keep_ghost)]` before
    building `LoadedVm`. `anyhow::Error`'s declaration moves from
    `openvmm_core` to `state_unit`. `collect_halves` is `pub`.
20. **Spec convention: `view` and `wf()` per concrete type** (no runtime
    change). `StateUnitsView::inv()`, `SavedStateView::inv()`, and
    `LoadedVmView::inv()` are new; `StateUnits::wf()` (now in `lib.spec.rs`),
    `SavedState::wf()` (new), and `LoadedVm::wf()` (new, replacing
    `units_consistent` and `snapshot_recorded`) include them.
    `LoadedVmView` gains `recorded`; `LoadedVm::save` requires `wf()` and
    `@.recorded`, and ensures `saved.wf()`; `restore` requires `wf()`. The
    views of `LoadedVm` and `SavedState` are now closed. `ReadySet`'s `ids()`
    becomes `impl View for ReadySet`; `collected` and `Inner::unit_ids` are
    crate-private; `StateRequest`'s helper predicates are inlined into
    `request_ok`; `ExampleUnit` has a view and `wf()`; the free lemma
    `lemma_restore_gives_saved_state` is now
    `LoadedVmView::lemma_restore_gives_saved_state`. New pending lemma
    `StateUnits::lemma_view_inv`. Each crate's `.proof.rs` is now included as
    `mod proof` inside `mod spec` (`vm/vmcore` has no `.proof.rs`).
21. **`SavedStateBlob` contract** (Verus only, no runtime change):
    `vm/vmcore/src/save_restore.spec.rs` adds `SavedStateBlobView::of::<T>`;
    new trusted file `vm/vmcore/src/save_restore_t.rs` (included as `mod
    trusted` in `save_restore.rs`) holds the injectivity axiom, the `clone`
    spec (moved from the spec file), and the stated `new`/`parse` contract,
    not yet attached.
22. **`SaveRestore` contract** (Verus only, no runtime change; the trait
    and its implementations are unchanged): `vm/vmcore/src/save_restore.spec.rs`
    gives it with `external_trait_specification` and
    `external_trait_extension(SaveRestoreSpec via SaveRestoreSpecImpl)`, and
    declares `SaveError` and `RestoreError`. New
    `vm/vmcore/src/save_restore.proof.rs` (included as `mod proof` inside
    `spec`): the verified `ExampleDevice`. (Declaring the trait in `verus!`
    in place was tried and dropped: a Verus build then requires every
    implementation in every crate to define `saved_state`.)
23. **Split `LoadedVm::quiesce_for_snapshot`** (`dispatch/microvm.rs`),
    behavior-preserving: new `quiesce_and_save` (the steps up to and
    including `save` and the VM-time cut, returning the decoded
    `SavedState` and the `TimeCapture`), called by `quiesce_for_snapshot`,
    which keeps the RAM flush, the command line, and the encoding, in the
    same order. Contracts (`external_body`) on `quiesce_and_save`, both
    `capture_time_abi` variants, and `StateUnits::quiesce_for_save`
    (`vmm_core/state_unit/src/quiesce.rs`); Verus-only `vstd` imports in
    both files; declarations of `QuiesceForSaveError` (`state_unit`),
    `SnapshotQuiesceError`, and `TimeCapture` (`openvmm_defs`).
24. **Restore-from-snapshot contracts** (Verus only, no runtime change):
    contracts (`external_body`) on `LoadedVm::restore_from_snapshot`,
    `begin_snapshot_restore`, `finish_snapshot_restore`
    (`dispatch/restore.rs`), `time_abi_restore` (`dispatch/time_abi.rs`, with
    a Verus-only `vstd` import), and `StateUnits::advance_time`
    (`vmm_core/state_unit/src/advance_time.rs`, likewise); `LoadedVm::restore`
    also keeps `clock_units`; `StateUnits`' `stop`, `reset`, `restore`, and
    `quiesce_for_save` also keep `advances_time`. `dispatch.spec.rs`:
    `SnapshotRestoreView` and `View for SnapshotRestore` (uninterpreted),
    `LoadedVmView::clock_units`. `state_unit`: the ghost registry records each
    unit's `advances_time` flag (the registry axioms cover it), and
    `StateUnitsView::advances_time` is new. Declarations of
    `RestoreTimeInput` and `ProfileSpan` (`openvmm_defs`).
25. **Request protocol follows the FracGhost rule** (Verus only, no runtime
    change; `vmm_core/state_unit/src/lib.proof.rs`, `lib.spec.rs`): `Start`
    gives `StateUnits`' half to the unit; `Stop` sends none and gets a half
    back. `StateUnits::wf()` requires halves for every collected unit only
    while stopped; its view records `Unknown` for a unit whose half it does
    not hold; `lemma_view_inv` no longer needs a half per unit;
    `collect_halves`' proof is updated. `ExampleUnit` follows the rule
    (fields `frac` and `running`, new method `set`).
26. **`CloseableMutex` trusted spec** (`support/closeable_mutex`; no change
    to existing behavior): new `closeable_mutex_t.rs` (trait
    `LockInvariant`; verified-code API `CloseableMutex::new_verified`,
    `close_verified`, and `VerifiedClosedGuard` with `borrow`, `borrow_mut`,
    `release`; the guard's `Drop` keeps the mutex closed if not released),
    `lib.proof.rs` (example, Verus only), a runtime test, `build.rs` (declares
    the `verus_keep_ghost` cfg), and a `vstd` dependency with
    `[package.metadata.verus] verify = true`.
27. **PIT spec** (`vm/devices/chipset`, `vm/vmcore`; no change to behavior):
    new `vm/vmcore/src/device_state.spec.rs` (`DeviceGhostState`, trait
    `DeviceGhost`), included as `device_state::spec`; Verus-only ghost
    fields `ghost` (a `DeviceGhostState`) and `id` on `PitDevice` (with placeholders in
    `new`, and in `save`'s destructuring); the bodies of `start`, `reset`,
    `poll_device`, `io_read`, `io_write`, `save`, `restore` moved into
    inherent methods with contracts (`start_device`, `reset_device`, `poll`,
    `pio_read`, `pio_write`, `save_device`, `restore_device`), which the
    trait methods call; `save_restore::state` is `pub(super)`; new
    `pit/mod.spec.rs` and `pit/mod.proof.rs`; `build.rs`; dependencies on
    `vstd` and `closeable_mutex`; `[package.metadata.verus] verify = true`.
    `UnitSnapshot` moved from `state_unit` (`lib.spec.rs`) to `vmcore`
    (`save_restore.spec.rs`); `state_unit` re-exports it.
28. **Verified device unit** (Verus only, no runtime change):
    `vm/vmcore/src/device_state.spec.rs` adds trait `StateUnitDevice` and
    `DeviceGhost::lemma_snapshot_known`; `vmcore` depends on
    `closeable_mutex`. `chipset`: the PIT implements `StateUnitDevice`
    (`pit/mod.proof.rs`), with trusted `pit/blob_t.rs`. `state_unit`: new
    `device_unit.rs` (`DeviceUnit<T>`, `cfg(verus_keep_ghost)`), dependency
    on `closeable_mutex`. `closeable_mutex`: `close_verified` takes `&Arc`.
29. **Run loop for the verified unit** (Verus only, no runtime change):
    `DeviceUnit` gains `poll_event` and `running`, a verified `poll`, and
    the unverified `run`; `StateUnitDevice::poll_device` (the PIT's calls
    its `poll`); `std::task::Context`'s Verus declaration moves from the
    PIT to `vmcore`; `closeable_mutex` adds `lock_verified` and
    `VerifiedOpenGuard` (with a runtime test).
30. **`InitializedVm::load` contract** (Verus only, no runtime change):
    contract (`external_body`) on `load` (`dispatch.rs`); declarations of
    `InitializedVm` and `HaltReason`; `LoadedVmView::time_units` (new; used
    by `clock_units`); `restore`, `restore_from_snapshot`,
    `finish_snapshot_restore`, `time_abi_restore` keep `time_units`
    (`time_abi_restore` now states its frame with it); verified lemma
    `LoadedVmView::lemma_load_restores` (`dispatch.proof.rs`).
31. **State requests through the mesh RPC wrapper** (runtime behavior
    unchanged; `vmm_core/state_unit/src/lib.rs`): `call_unit` sends with
    `mesh_rpc_t::call` and waits with `PendingCall::recv` instead of
    `RpcSend::call`; `run_unit` and `run_async_unit` receive with
    `mesh_rpc_t::recv_msg` instead of `recv.next()`; `StateRequest::apply`
    replies with `mesh_rpc_t::split` / `complete` instead of `Rpc::handle` /
    `handle_failable` (no async closures; errors converted with
    `RemoteError::new`, as before). Placeholder ghosts throughout.
32. **Mesh RPC wrapper as its own crate, used for `VmRpc` and
    `PartitionRequest`** (runtime behavior unchanged): new crate
    `mesh_rpc_verus` (`support/mesh/mesh_rpc_verus`; `state_unit`'s
    `mesh_rpc_t.rs` moved there, and `state_unit` uses it as
    `mesh_rpc_verus`), adding `PendingCall::recv_failable`. Placeholder
    protocols (uninterpreted `request_ok`): `openvmm/openvmm_defs/src/rpc.spec.rs`
    (`VmRpc`) and `vmm_core/src/partition_unit.spec.rs` (`PartitionRequest`).
    Switched to the wrapper, both ends: `VmRpc`'s `QuiesceForSnapshot`,
    `ResumeAfterFailedSnapshot`, `ReleaseSnapshotBoundary`
    (`openvmm_entry/src/vm_controller/microvm.rs`, `openvmm_core`'s
    `dispatch.rs`); `PartitionRequest`'s `StopVps` and `Snapshot`
    requests (`vmm_core/src/partition_unit.rs`, `partition_unit/snapshot.rs`).
    `vmm_core` is Verus-enabled (`[package.metadata.verus]`, `vstd`,
    `build.rs` declares `verus_keep_ghost`). rustfmt reordered imports in
    `dispatch/microvm.rs`, `restore.rs`, `time_abi.rs`.
33. **`SavedStateBlob` contract attached** (Verus only, no runtime change):
    `vm/vmcore/src/save_restore_t.rs` declares the protobuf trait chain and
    the struct encoder to Verus and attaches `new` / `parse`'s contract with
    `assume_specification` (plus `RestoreError`'s `From<payload::Error>`).
    The PIT's `save_blob` / `restore_blob` call `SavedStateBlob::new` /
    `parse` directly; `vm/devices/chipset/src/pit/blob_t.rs` is removed.
    Needs Verus patch 0003 (`verus-patches/`); `scripts/verify-snapshot-spec.sh`
    notes it.
34. **`ProtobufSaveRestore` contract; enum saved-state roots** (Verus
    only, no runtime change): `vm/vmcore/src/save_restore.spec.rs` adds
    `ProtobufSaveRestore`'s contract (`external_trait_extension`,
    `saved_blob()`); the blanket impl in `save_restore.rs` is `verus_verify`
    and its bodies use `match` instead of `map` and `?` (same behavior);
    `save_restore.proof.rs` defines its `saved_blob()` and adds the example
    enum root `ExampleEnumState`; `save_restore_t.rs` declares
    `OneofEncoder` and its traits. Patch 0003 gains the third fix (an
    undeclared type in a bound is an error).
35. **Registry axioms removed** (Verus only, no runtime change;
    `vmm_core/state_unit`): `Inner` gains a Verus-only `registry_ghost`
    (`RegistryGhost`); new `registry.rs` (compiled in both builds) with the
    verified `Inner::insert_unit` / `remove_unit`, called by
    `UnitBuilder::build` and `UnitHandle::remove_if` instead of the map
    insert / remove; `StateUnits::new` builds both ghost states with
    `new_ghosts()`; `UnitHalves` gains `collected`; the lock invariant
    (`InnerPred`) is rewritten; `collect_halves` is rewritten without the
    axioms; `registry_t.rs` keeps only its assumed specifications
    (`lock_registry`, `lock_registry_exact`, `agrees_with`,
    `agrees_exactly`, `collected`, `no_halves` removed). `mutex_t.rs`:
    `MutexPredicate` gains a key (`type Key`, `key`), `Mutex::key`,
    `MutexHandle::key`.
36. **The worker's quiesce side completed** (Verus only, no runtime
    change): contracts on `quiesce_for_snapshot`,
    `resume_after_failed_snapshot`, `establish_snapshot_boundary`,
    `release_snapshot_boundary`, `handle_snapshot_boundary`
    (`dispatch/microvm.rs`), `resume_after_failed_save` and
    `has_uncertain_state` (`quiesce.rs`); `quiesce_and_save` and
    `quiesce_for_save` gain error-path clauses. `LoadedVmView::runtime`
    gains `snapshot_boundary` (`SnapshotBoundary`'s view); `LoadedVm::wf()`
    ties the two running flags; `QuiesceForSaveError` gets a view.
    `openvmm_defs`: `ProtobufMessageView` and `SnapshotSaveResponse`'s
    declaration (`rpc.spec.rs`), new trusted `rpc_t.rs`; `lib.rs` forbids
    `unsafe` only outside Verus (denies it under Verus), for
    `assume_specification`. Declarations of
    `MicrovmSnapshotBoundaryRequest` and `RecvError`.
37. **The units' views leave out guest time** (Verus only, no runtime
    change): `SavedStateBlobView::equal_except_time` (new) and the axioms
    `axiom_of_injective` (now up to time) and `axiom_of_forgets_time`
    (`vm/vmcore`); `LoadedVmView` loses `time_units` and `clock_units`;
    `restore_from_snapshot`, `finish_snapshot_restore`,
    `time_abi_restore`, `StateUnits::advance_time`, `InitializedVm::load`,
    and `LoadedVm::restore` claim equality for every unit (no clock-unit
    exclusions); `lemma_load_restores` is simplified.
38. **`StateUnit`'s contract** (Verus only, no runtime change;
    `vmm_core/state_unit`): `StateUnit` is `verus_verify`, each method has
    a `verus_spec` contract (the request protocol, with the FracGhost halves
    as a ghost input `g`), and it gains the supertrait `StateUnitSpec`
    (`lib.spec.rs`: `wf`, `id`, `running`; in normal builds an empty trait
    implemented for every type, `lib.rs`). `lib.proof.rs` declares
    `inspect::InspectMut` and `inspect::Inspect`. Workspace `Cargo.toml`:
    `[patch.crates-io]` replaces `verus_builtin_macros` (the version `vstd`
    pins) with a patched copy, `verus-patches/verus_builtin_macros`
    (excluded from the workspace), carrying the macro half of Verus patch
    0005; `Cargo.lock` follows. The copy's diff from the published crate is
    `verus-patches/0005-verus_builtin_macros-published.patch`; normal builds
    use it too (the fixes change only `verus_spec(with ...)` on traits and
    trait impls, and `cfg_attr`'d `verus_spec` under `verus_verify`, which
    normal builds strip).
39. **The production `StateUnit` impls converted** (Verus only, no runtime
    change): the ten impls (`ArcMutexChipsetDeviceUnit`, `ChipsetUnit`,
    `LineSetUnit` in `vmm_core/vmotherboard`; `KeeperUnit`,
    `input_distributor::Inner`, `PartitionUnitRunner`, `&VmbusServerUnit`,
    `&ChannelUnit<T>`, `&SimpleChannelUnit<T>` in `vmm_core`; `TimeAbiUnit`
    in `openvmm_core`) are `verus_verify`; each method takes the ghost input
    (`verus_spec(with Tracked(g): ...)`) and is `external_body` for now. Each
    unit file includes a new `<file>.spec.rs` (the unit's `StateUnitSpec`,
    uninterpreted until its per-unit definition) and `<file>.proof.rs` (the
    unit type's declaration); for the partition unit, its existing
    `partition_unit.spec.rs` gains the impl. `vmbus_unit.proof.rs` declares
    `VmbusDevice`, `SimpleVmbusDevice`, `RingMem`, `GpadlRingMem`, and
    `core::any::Any`. vmotherboard becomes Verus-enabled
    (`[package.metadata.verus] verify = true`, `vstd` dependency, a
    `build.rs` declaring the `verus_keep_ghost` cfg, a cfg'd `vstd` import in
    `lib.rs`); vmm_core gains a `vmbus_ring` dependency (already in its
    dependency graph through `vmbus_channel`).
40. **`StateRequest::apply` and `run_unit` verified** (`vmm_core/state_unit`,
    `lib.rs`; same runtime behavior, `state_unit`'s tests pass): `apply`
    takes the request's half as a ghost input (`with Tracked(gin)`) and
    passes it to each `StateUnit` method (`proof_with!`); errors are
    converted with `match` and three helpers (`anyhow_error`, `save_error`,
    `restore_error`) instead of `map_err(RemoteError::new)`; replies go
    through a new `reply` (ghost input, then `complete`); `Inspect` through
    a new `inspect_unit`. `run_unit` is now in `verus!` syntax, its loop
    calling a new `run_unit_step` (one `recv_msg` and `apply`).
    `no_reply_ghost` is removed. `lib.rs` imports `vstd::prelude` in both
    builds (for the macros); `build.rs` declares the
    `verus_keep_ghost_body` cfg.
41. **More request loops verified** (same runtime behavior): new verified
    `state_unit::handle_request(unit, req, gin)` (takes the half as an
    argument, so `verus!` code can call it; `run_unit_step` uses it) and
    the alias `RequestGhost`. `vmotherboard`: the chipset unit's and each
    line set's task call `run_unit` instead of their own
    `recv` / `apply` loop; `ArcMutexChipsetDeviceUnit::run` is in `verus!`,
    its loop verified, its `select!` moved into `next_event` and its poll
    into `poll_now` (both `external_body`; `next_event` receives with
    `recv_msg`); its two spawn sites await `run` inside an `async` block
    (a Verus/rustc bug otherwise, see the design doc); `mesh_rpc_verus`
    dependency.
42. **Per-unit `StateUnitSpec` definitions, first four** (Verus only, no
    runtime change): `LineSetUnit`, `ChipsetUnit` (now `verus_verify`),
    `KeeperUnit` gain a Verus-only ghost field (`Tracked<DeviceGhostState>`,
    `#[inspect(skip)]`) and a constructor `new(.., &mut recv)` that takes the
    unit's half (`external_body`, contract); their tasks construct them with
    it. The input distributor's `Inner` gains
    `Tracked<Option<DeviceGhostState>>`, filled by `InputDistributor::run`
    (now `external_body` with a contract). Their `StateUnitSpec`s are
    defined (closed): the share follows the rule and records
    `NoSavedState` (line sets, chipset unit, input distributor) or
    `Saved(of(keeper@.saved))` (time keeper). `vm/vmcore`: new
    `vmtime.spec.rs` (`VmTimeKeeperView`) and `vmtime.proof.rs`;
    contracts on `VmTimeKeeper::save`, `restore`, `advance`, `reset`,
    `start`, `stop` (`external_body`). Declarations: `LineSet`, `Chipset`,
    `Forwarder`, `KeyboardData`, `MouseData`, `InputDistributor`,
    `VmTimeKeeper`, vmtime's `SavedState`.
43. **The time-ABI unit's share with its state; creator-made FracGhost**
    (Verus only, no runtime change): `state_unit` gains `unit_channel_for`
    (a channel whose sender carries a given half; `external_body`) and
    `UnitBuilder::spawn_with_channel` (`spawn` with a given channel;
    `spawn` uses it). `vmm_core/virt` is Verus-enabled (metadata, `vstd`
    dependency, `build.rs` cfg, `lib.rs` import); in Verus builds
    `TimeAbiMsrs::tsc_invariant_control` is vstd's atomic with ghost
    (normal builds keep `std`'s `AtomicU64`), `TimeAbiMsrs` is
    `verus_verify` with a hand-written `Debug` in Verus builds, `new` takes a
    ghost out-parameter for the other half (`external_body`), and the
    value's loads and stores go through two private helpers
    (`load_control`, `store_control`; `external_body` in Verus builds); new
    `time_abi/msr.spec.rs` (`TscControlPred`, `key`, `wf`) and
    `msr.proof.rs`. `openvmm_core`: `TimeAbiUnit` gains a Verus-only
    `running` and `new` (`external_body`), its `StateUnitSpec` is defined,
    and `dispatch.rs` spawns it with `spawn_with_channel(..,
    unit_channel_for(), ..)`, awaiting `run_unit` in an `async` block. The
    time keeper: `VmTimeKeeperView` keeps only `running`; the keeper's
    snapshot is the constant `VmTimeKeeperView::saved_blob()`; new trusted
    `vm/vmcore/src/vmtime_t.rs` (`SavedState::axiom_all_time`); the keeper's
    method contracts keep their `running` clauses, `save` ensures its
    result's view is `saved_blob()`.
44. **Chipset devices' shares in their locks; the PIT folded in** (Verus
    only, no runtime change; design doc, "Chipset devices: the share in the
    device's lock", option B): `vm/vmcore/src/device_state.rs` gains
    `DeviceSlot<T>` (the device and, Verus only, its unit's share and
    `omit`; `Deref` / `DerefMut` to the device), `DeviceMutex<T>`
    (= `CloseableMutex<DeviceSlot<T>>`), and the `DeviceSnapshot` stand-in;
    `device_state.spec.rs` gains `DeviceSnapshot` and the slot's
    `LockInvariant`, and loses `DeviceGhost`, `StateUnitDevice`, and
    `ExContext` (moved to the PIT). `closeable_mutex_t.rs`: `key()` for
    unsized values. Every device lock is created as
    `CloseableMutex::new(DeviceSlot::new(dev))` and named
    `DeviceMutex<..>`: vmotherboard (about 10 files), `vmm_core`'s
    `device_builder.rs`, `vpci` (`bus.rs`, `device.rs`; tests),
    `vpci_client`'s tests, `chipset_arc_mutex_device` (now depends on
    vmcore), `chipset_device_fuzz` (depends on vmcore; `&mut **locked_dev`),
    `openvmm_core`'s `dispatch.rs`, OpenHCL's `underhill_core/src/worker.rs`
    (fallback MMIO device). `ArcMutexChipsetDeviceUnit` is `verus_verify`,
    its `new` has a contract (`external_body`), its `StateUnitSpec` is
    defined; `DynDevice` gains `DeviceSnapshot` as a supertrait; its two
    spawn sites use `spawn_with_channel(.., unit_channel_for(), ..)`;
    `state_unit.proof.rs` declares `ChipsetDevice`, `ChangeDeviceState`,
    `VmmChipsetDevice`, `DynDevice`, `PollEvent`. The PIT: Verus-only
    `ghost` / `id` fields, its `DeviceGhost` / `LockInvariant` /
    `StateUnitDevice` impls, `PitDeviceView::snapshot`, the `example_*`
    walk-throughs, and the ghost clauses of its entry points' contracts
    removed; `vmm_core/state_unit/src/device_unit.rs` (`DeviceUnit`)
    removed.
45. **Trait-level `device_wf()`; `DeviceGhostState::wf` renamed** (Verus
    only, no runtime change; no normal-build code changes): `SaveRestore`'s
    and `ProtobufSaveRestore`'s spec extensions (`vm/vmcore/src/save_restore.spec.rs`)
    declare `device_wf()`, and their `save` / `restore` contracts require
    and keep it; the blanket `ProtobufSaveRestoreSpecImpl` defines it as the
    type's `SaveRestore` one, `ExampleDevice`'s is `true`, the PIT's is its
    `wf()` (`pit/mod.spec.rs`); `DeviceSnapshot` gains `device_wf()`, and the
    slot's lock invariant includes it. `DeviceGhostState::wf(id, snapshot)`
    is renamed `follows_rule(id, snapshot)` (so that `wf` is always the
    zero-argument convention); its callers in the units' spec files and
    `TscControlPred` follow.

### Planned: mesh RPC trusted spec

From the draft in `verus-patches/verus-rpc-prototype/src/mesh_rpc_t.rs`
(see "Mesh RPC trusted spec" in the design doc). Not yet made.

All in `vmm_core/state_unit`:

1. *(Done: `src/mesh_rpc_t.rs`, item 10 above.)*
2. *(Done: Verus wiring and the mutex wrapper, items 8 and 9 above.)*
3. **`UnitBuilder::spawn`** (`src/lib.rs:989`): `mesh::channel()` →
   `channel(Ghost(k))`.
4. **`state_change`** (`src/lib.rs:843`): `unit.send.clone()` →
   `clone_sender(&unit.send)`, and `send.call(request, input).await` →
   `call(&send, request, input, Tracked(gin), Ghost(post)).recv().await`.
5. **`StateRequest::apply`** (`src/lib.rs:224-245`):
   `rpc.handle_failable(async |..| unit.save().await)` → await the unit
   method, then `complete(rpc, result.map_err(RemoteError::new), Tracked(gout))`.
   Needed also because Verus rejects async closures.
6. **`run_op` callers** (`StateUnits::save`, `restore`, ...): pass closures
   (`|rpc| StateRequest::Save(rpc)`) instead of the variant constructors,
   which Verus rejects as function values.

## Verus

Baseline: Verus commit `426d8b0` ("Support /= and %= for signed integers
(#3013)"), matching the pinned `vstd` (`=0.0.0-2026-10-04-0306`). `vstd`
itself is unmodified.

### Made

1. **`0001-async-fn-unnamed-return-keeps-ensures.patch`**
   (`verus-patches/`): an `async fn` without a named return lost its
   postconditions at `.await`. Changes `rust_verify/src/rust_to_vir_func.rs`
   and adds four tests to `rust_verify_test/tests/async_functions.rs`.
   Applied to the local checkout (`~/programs/verus`, uncommitted) and built.
   Bug, fix, and test results: `verus-patches/README.md`.

2. **`0002-async-fn-ghost-params.patch`** (`verus-patches/`):
   annotation-style ghost parameters (`#[verus_spec(with ...)]`) on an
   `async fn` failed with an internal error. Changes
   `rust_verify/src/pre_header.rs` (find the parameter by name when the
   async body refers to it through the binding that moves it into the
   coroutine) and adds two tests to `rust_verify_test/tests/syntax_attr.rs`.
   Applied to the local checkout and built. Ghost outputs of an `async fn`
   are still unsupported. Details and test results: `verus-patches/README.md`.

3. **`0003-external-trait-own-assoc-bounds-and-hrtb.patch`**
   (`verus-patches/`): a trait whose supertrait bound names its own
   associated type (mesh_protobuf's `Protobuf`) could not be declared with
   `external_trait_specification`, and an external impl whose associated
   type has a `for<'a>` bound panicked Verus, as did a bound naming a type
   not declared to Verus. Changes `rust_verify/src/rust_to_vir_trait.rs`
   (map the proxy's associated types to the external trait's before
   comparing bounds), `rust_verify/src/rust_to_vir_impl.rs` (erase
   late-bound regions instead of leaving them escaping), and
   `vir/src/recursive_types.rs` (an error instead of a panic), and adds
   three tests to
   `rust_verify_test/tests/external_traits.rs`. Applied to the local
   checkout and built. Details and test results: `verus-patches/README.md`.

4. **`0004-async-fn-in-trait.patch`** (`verus-patches/`): `async fn` in
   a trait ("does not yet support Opaque types in trait def"), for
   `StateUnit`. Details and test results: `verus-patches/README.md`.
5. **`0005-trait-method-ghost-params-and-async-defaults.patch`**
   (`verus-patches/`): ghost inputs (`with`) on trait methods, and async
   default methods. Its macro half also lives in the patched copy of the
   published macro crate, `verus-patches/verus_builtin_macros`, which
   OpenVMM's build uses (OpenVMM change 38). Details and test results:
   `verus-patches/README.md`.

### Possibly needed

From the gaps in the design doc; not started.

- Async closures ("does not yet support coroutine closure types"); avoided
  for now by OpenVMM change 5 above.
- Datatype constructors as function values; avoided by OpenVMM change 6.
