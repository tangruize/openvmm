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
46. **The vmbus units implement `StateUnit` for `X`** (executable change;
    behavior unchanged; design doc, "The vmbus units", option A):
    `state_unit` gains `InspectConcurrently`; `run_async_unit` requires
    `T: StateUnit + InspectConcurrently` (was `for<'a> &'a T: StateUnit`),
    takes the unit's inspector once, and passes the unit as `&mut T`;
    `apply_with_concurrent_inspects(self, unit: &mut T, inspector: &impl
    Inspect, recv)` (was `(self, unit: &mut &T, recv)`) sends inspects that
    arrive during a state change to the inspector; both are `external_body`
    with the contracts of `run_unit` / `apply`; `lib.proof.rs` declares
    `InspectConcurrently`. `vmbus_server` (`VmbusServerInspector`,
    `VmbusServer::inspector`), `vmbus_channel` (`ChannelInspector`,
    `ChannelHandle::inspector`, `SimpleDeviceHandle::inspector`),
    `vmbus_relay` (`HostVmbusTransportInspector`,
    `HostVmbusTransport::inspector`): an inspector holding a clone of the
    internal sender. `vmm_core/src/vmbus_unit.rs` (`VmbusServerUnit`,
    `ChannelUnit`, `SimpleChannelUnit`) and
    `openhcl/underhill_core/src/vmbus_relay_unit.rs` (`VmbusRelayUnit`):
    `StateUnit` for `X` instead of `&X` (same bodies), plus `InspectMut`
    (forwarding to `Inspect`) and `InspectConcurrently`; `vmbus_unit.spec.rs`
    implements `StateUnitSpec` for `X` (still uninterpreted). New test
    `tests/concurrent_inspect.rs`; `state_unit`'s dev-dependency on
    `inspect` enables `initiate`.
47. **The vmbus channels keep their units' shares; `VmbusDevice` without
    `Any`** (executable change in `vmbus_channel` and `vmm_core`, behavior
    unchanged; design doc, "The vmbus units"): `vmbus_channel` is
    Verus-enabled (`[package.metadata.verus]`, `vstd`, `mesh_rpc_verus`,
    `build.rs`, `lib.rs` import). `VmbusDevice: Send + 'static + IntoAny +
    InspectMut` (was `Send + Any + InspectMut`); new `IntoAny` with a blanket
    impl for sized `T: Any`; `ChannelHandle::revoke` uses `.into_any()`.
    New `channel.spec.rs` (`VmbusDevice`'s spec extension: `saved_blob`,
    `device_wf`; `MeshProtocol for StateRequest`; `View for Device`;
    `Device::keeps_share`; `GenericChannelHandle::id`, `ChannelHandle::id`,
    uninterpreted), `channel.proof.rs` (declarations: `IntoAny`, `Any`,
    `StateRequest`, `Device`, `ParentBus`, `Spawn`, `GenericChannelHandle`,
    `ChannelHandle`), `simple.spec.rs` (`SimpleDeviceHandle::id`),
    `simple.proof.rs` (`RingMem`, `GpadlRingMem`, `SimpleVmbusDevice`,
    `SimpleDeviceWrapper`, `VmTaskDriverSource`, `InitialDeviceState`).
    Contracts (`external_body`): the handles' `start` / `stop` / `reset` /
    `save` / `restore` (ghost input), `offer_generic`, `offer_channel`,
    `offer_generic_channel`, `offer_simple_device` (ghost out-parameter),
    `Device::handle_state_request`, `Device::run_channel` (ghost
    inputs); `revoke` / `inspector` are `external_body` / `external` in the
    `verus_verify` blocks. `run_channel` takes `self` (was `mut self`,
    which Verus does not support) and rebinds it, `let mut this = self;`,
    with `self` renamed `this` in the body. `run_channel` and `handle_state_request` moved into their own
    `impl Device` block; `handle_state_request`'s `#[instrument]` only in
    normal builds. `SimpleDeviceHandle` is `verus_verify`. vmcore:
    `UnitSnapshot::from_saved`; declarations of `InspectMut`, `Inspect`,
    `Deferred`, `anyhow::Error` moved here from `state_unit`;
    `mesh_rpc_verus`: `RemoteError`'s declaration moved here; `vmm_core`'s
    declarations of `VmbusDevice`, `SimpleVmbusDevice`, `RingMem`,
    `GpadlRingMem`, `Any` moved to `vmbus_channel`. `vmm_core`:
    `ChannelUnit` / `SimpleChannelUnit` gain a Verus-only `running` field
    and `new`, are `verus_verify` (`Debug` derived only in normal builds,
    hand-written in Verus builds), their `StateUnitSpec`s are defined, and
    their three offer functions spawn with
    `spawn_with_channel(.., unit_channel_for(), ..)`.

48. **The microVM's shutdown and snapshot-request devices specified and
    verified** (one executable change in `chipset`, behavior unchanged;
    design doc, "The microVM's units: what is specified"): new
    `chipset/src/microvm.spec.rs` (`SaveRestoreSpecImpl` for
    `MicrovmShutdown` and `MicrovmSnapshotRequest`: saved state
    `NoSavedState`, `device_wf()` true) and `microvm.proof.rs` (both
    devices declared by name only (fields hidden)); a cfg'd `spec` module and `vstd`
    import in `microvm.rs`. Both devices' `SaveRestore` impls are
    `verus_verify` and verify against the trait's contract. Their
    `restore` takes `_state` instead of the pattern `NoSavedState`
    (executable; Verus allows only identifier parameters). vmcore:
    `NoSavedState` declared to Verus (`save_restore.proof.rs`). VMBus
    recorded as out of scope for the microVM (design doc, "Out of scope for
    now").

49. **`SaveRestore`'s contract on views** (executable change in `vmcore`
    and 22 device crates' setup, behavior unchanged; design doc,
    "`SaveRestore`'s contract on views"): `SaveRestore` gains the
    supertrait `SaveRestoreView` (normal builds: an empty trait implemented
    for every type; Verus builds: `View`). The contract
    (`save_restore.spec.rs`) is stated on the view: `save` keeps
    `self@` and returns a state whose blob is `blob_of(old(self)@)`; a
    successful `restore(state)` makes `blob_of(final(self)@)` `state`'s
    blob; `spec fn blob_of(view)` replaces `spec fn saved_state()`. The
    blanket `ProtobufSaveRestore` impl's `saved_blob()` is
    `blob_of(self@)` and still verifies. `ExampleDevice` has a view.
    The PIT's `blob_of` is `of(view.saved_state())`; the shutdown and
    snapshot-request ports have view `()` and blob `of(NoSavedState)`
    (their impls still verify). New macro `vmcore::placeholder_view!`
    (nothing in normal builds), and a placeholder line before each of the
    59 other `impl SaveRestore for X` in the crates Verus compiles (in
    `chipset`, `openvmm_core`, `vmm_core`, and 22 newly Verus-enabled
    crates: `[package.metadata.verus]`, `vstd`, `build.rs`, a cfg'd
    `use vstd::prelude::verus as _;`). `chipset_legacy`'s
    `Winbond83977FloppySioDevice` uses the macro's `external` form.

50. **Saved states compared by their views; the PIC and the RTC
    specified** (one executable change in `vmcore`, behavior unchanged;
    design doc, "`SaveRestore`'s contract on views"):
    `SaveRestore::SavedState` gets the bound `SaveRestoreView` (normal
    builds: implemented for every type). The contract compares views:
    `save`'s result has view `saved_view(old(self)@)`; after a successful
    `restore(state)`, `saved_view(final(self)@) == state@`; `spec fn
    saved_view(view)` replaces `blob_of(view)`. Blobs are generic only:
    `ProtobufSaveRestore`'s blanket `saved_blob()` is
    `of_view(saved_view(self@))`, verified with a module-level `broadcast
    use` of the new link axiom. Trusted (`save_restore_t.rs`):
    `SavedStateBlobView::of_view` (new), `axiom_of_is_of_view` (a blob is
    determined by its saved state's view) and `axiom_of_view_injective`
    replace `equal_except_time`, `axiom_of_injective`,
    `axiom_of_forgets_time`; `vmtime_t.rs` (`axiom_all_time`) removed,
    vmtime's `SavedState` has view `()`. Views: `NoSavedState` (itself,
    Verus-declared), `SavedStateNotSupported` (`()`, cfg'd plain impl in
    `save_restore.rs`), `ExampleDevice`'s `saved_view`. Placeholder
    saved-state views: `placeholder_view!(saved T)` (identity, outside
    `verus!`) after 45 device placeholders, two in `chipset_legacy`
    (Winbond), and in `virt` for `VmSavedState` / `VpSavedState` (x86,
    aarch64). The PIT and the PIC: saved-state view the identity,
    `saved_view = view.saved_state()`. The ports: `saved_view(()) =
    NoSavedState`.
    New PIC spec (`pic/mod.spec.rs`, `pic/mod.proof.rs`, a cfg'd `spec`
    module inside `pic::save_restore`): `PicView` / `DualPicView`, `inv()`
    (ICW2's low bits zero; the first PIC primary), `wf()`, `saved_view`;
    `DualPic`, `Ocw3`, the saved `SavedState`, and `IRQ_MASK` are
    `verus_verify` (cfg'd attributes); the `SaveRestore` impl is
    `verus_verify` with `external_body` methods; narrow cfg'd imports of
    `verus_spec` / `verus_verify`.
    New RTC spec (`cmos_rtc.spec.rs`, `cmos_rtc.proof.rs`, a cfg'd
    `verus_spec` module inside `cmos_rtc::save_restore`; not `spec`, the
    RTC's register module): `RtcView` (`addr`, CMOS bytes, transaction
    mask; no time), `inv()` (no transaction), `wf()`, the saved state's
    view `RtcView` (without `clock_time_millis`, `time_valid`),
    `saved_view` the identity; `Rtc`, `RtcState`, `CmosData` are
    `verus_verify`; the `SaveRestore` impl is `verus_verify` with
    `external_body` methods.

51. **portb and the IOAPIC specified; a saved state's view is its
    meaning** (one dependency change in `chipset`, behavior unchanged;
    design doc, "`SaveRestore`'s contract on views" and the unit table):
    - portb (`microvm.spec.rs`, `microvm.proof.rs`): `MicrovmPortbView`
      (`rx`, `tx` as `Seq<u8>`), also its saved state's view;
      `saved_view` the identity; `wf()`: each buffer at most
      `BUFFER_MAX`. `MicrovmPortb`, `BUFFER_MAX`, `GENERATION_ID_SIZE`
      get cfg'd `verus_verify`; its `SaveRestore` impl is `verus_verify`
      with `external_body` methods; declared: `MicrovmPortbSavedState`,
      and by name only (fields hidden) `PortbTimeAbi`, `MicrovmPortbDrain`,
      `std::task::Waker`, the traits `SerialIo`, `futures::AsyncRead`,
      `futures::AsyncWrite`. Executable: `chipset` gets the dependency
      `mesh_rpc_verus` (its declarations of `mesh::Receiver` and `Rpc`),
      referenced by a cfg'd `use mesh_rpc_verus as _;` in `lib.rs`.
    - IOAPIC (`ioapic/mod.spec.rs`, `ioapic/mod.proof.rs`, a cfg'd
      `verus_spec` module inside `ioapic::save_restore`): `IoApicView`
      (ID, index, entries' bits, line levels), `wf()`; its saved state's
      view is the state's meaning (an empty `line_levels` is all low; a
      pending interrupt is delivered, i.e. remote IRR set);
      `saved_view` delivers pending interrupts. `IoApicDevice`,
      `IrqEntry`, `RedirectionEntry` (with a closed `bits()` accessor in
      the register module) and the saved `SavedState` get cfg'd
      `verus_verify`; its `SaveRestore` impl is `verus_verify` with
      `external_body` methods; declared: `IndexRegister`, and by name only
      `IoApicStats` and the trait `IoApicRouting`.
    - Rule (user, 2026-10-27): a saved-state type's view is its meaning,
      what `restore` makes of it; both IOAPIC normalizations are
      intended (commit `4e396f9df`).

52. **virtio transports: `save` copies the captured device state**
    (executable change in `vm/devices/virtio/virtio`, **behavior changes in
    one case**; decided 2026-10-28, user; design doc, "Executable code
    changes" and "Other open items"). `TransportRestore`'s
    `take_device_state` (`self.restore.captured.take()`) is replaced by
    `captured_device_state`: on a successful capture it returns a clone of
    the `SavedStateBlob` and keeps the capture (until the next `start` /
    reset, which clear it as before); on a failed capture it returns (and
    drops) the error; with no capture it fails as before. Callers: the MMIO
    and PCI transports' `save` (`mmio.rs`, `pci.rs`). Scope: every virtio
    device (net, console, fs, block, ...) on both the MMIO and the PCI
    transport, since the change is in their shared transport code.
    Behavior: a second
    `save` before the next `start` now returns the same state instead of
    failing; nothing else changes. Reason: `SaveRestore`'s contract (`save`
    keeps the view; its result is determined by the view). The same change
    against `origin/main` (where the method is in `transport/core.rs`,
    renamed `saved_device_state` there since the field is
    `captured_device_state`) is
    `verus-patches/openvmm-0001-virtio-save-keeps-captured-device-state.patch`,
    for OpenVMM developers (`verus-patches/README.md`, "OpenVMM patches").
    Checked: `cargo test -p virtio` 141 passed on this branch, 140 on
    `main`.

53. **virtio transports: `restore` sets the captured device state too**
    (executable change in `vm/devices/virtio/virtio`, **behavior changes in
    one case**; decided 2026-10-28, user; design doc, "Executable code
    changes"). `TransportRestore::stage` (called by `restore_common`) also
    sets `captured = Some(Ok(device_state.clone()))`, so the device's
    private state `D` restored by `restore(D)` is in both slots: staged
    (for the next `start`) and captured (for `save`). Scope: every virtio
    device on both the MMIO and the PCI transport. Before: a `save` after
    `restore(D)` and before the next `start` failed ("device state was not
    captured") after a fresh VM or a reset (OpenVMM's flows), and returned
    the pre-restore state after a `stop` without a reset (no OpenVMM flow
    today). Now it returns `D`; `start` and reset are unchanged. Reason:
    `SaveRestore`'s contract (after `restore(state)`, the device saves
    `state`). Same change against `origin/main` + `openvmm-0001`:
    `verus-patches/openvmm-0002-virtio-restore-sets-captured-device-state.patch`.
    Checked: `cargo test -p virtio` 141 passed on this branch, 140 on
    `main`.

54. **virtio-MMIO specified** (two dependency changes in `virtio`, behavior
    unchanged; design doc, the unit table and "Other open items"): new
    `transport/mmio.spec.rs`, `mmio.proof.rs` (a cfg'd `spec` module inside
    `mmio::saved_state`). `VirtioMmioView` (the transport's saved
    registers, two feature banks, queues as `VirtioQueueView`s with their
    progress, the captured private state, the interrupt status) is also
    the saved state's view; `saved_view` keeps it, with the private state
    to save; `wf()`: two feature banks. `VirtioMmioDevice` and
    `MmioTransport` get cfg'd `verus_verify`; the `SaveRestore` impl is
    `verus_verify` with `external_body` methods. Declared: the transport
    core, `QueueData`, `QueueParams`, `QueueState`, the saved-state types
    (transparent), and by name only (fields hidden) `TransportRestore`, the feature and
    status bitfields, `DeviceCommand`, `TransportState`, `VirtioDoorbells`,
    `StalledIo`, `InterruptState`, `InterruptAck`, `GuestMemory`,
    `DeferredWrite`, `pal_event::Event`, `async_task::Task`,
    `TaskMetadata`, `lock_api::Mutex`, `parking_lot::RawMutex`, the
    validator `Validator`. One trusted accessor (uninterpreted):
    `MmioTransport::interrupt_status` (TODO in the design doc). Defined
    accessors: `TransportRestore::captured()` (closed, in
    `transport/core/restore.rs`; `TransportRestore` is `verus_verify`),
    `VirtioDeviceFeatures::bits()` / `bank_bits(i)` and
    `VirtioDeviceStatus::bits()` (closed, in `virtio_spec`; both bitfields
    `verus_verify`, the status with `verifier::external_derive` for its
    `zerocopy` derives). `std::task::Waker`'s
    declaration moved from `chipset` to vmcore (both crates need it).
    Executable: `virtio` gets the dependencies `mesh_rpc_verus` and
    `async-task`, referenced by cfg'd `use .. as _;` lines in `lib.rs`;
    `virtio_spec` is Verus-enabled (`[package.metadata.verus]`, `vstd`,
    `build.rs`, a cfg'd import); `TransportRestore`'s validator is wrapped
    in a new private struct `Validator` (same validator, same calls).

55. **The partition unit's `StateUnitSpec`** (one executable change in
    `vmm_core`, behavior unchanged; design doc, the unit table and "Other
    open items"). `PartitionUnitRunner` is `verus_verify` and gets a
    Verus-only field `share` (its FracGhost share), a placeholder in
    `PartitionUnit::new` replaced in the unit's task by the new Verus-only
    `attach_share` (contract, `external_body`), and a contract on `run`
    (`external_body`). `partition_unit.spec.rs`: `PartitionSavedView` (the
    saved state's view: the partition's blob, the VPs' indexes and blobs,
    the encoded halt reason), the runner's `saved_view()`, and
    `StateUnitSpec` (`wf`: the share follows the rule for
    `Saved(saved_view().blob())` and runs while started; `id`; `running`).
    The saved state's `View` and `blob()` are in a cfg'd `verus!` block
    inside `save_restore`; `state::Partition` and `state::Vp` get cfg'd
    `verus_verify`. Trusted accessors: `VpSet::saved_vps` (until the VP
    layer), `PartitionSavedView::encode_halt`. Declared: the trait
    `VmPartition`, and by name only `VpSet`, `InternalHaltReason`,
    `Topology`, `InitialRegs`, and `HaltReason` (moved here from
    `openvmm_core`'s `dispatch.proof.rs`). Executable: the runner's
    `topology` field is wrapped in a new private struct `Topology` (built
    with `Topology(..)`, read as `.0`; also in a test).

56. **The partition runner is built inside its task** (executable change
    in `vmm_core`, behavior unchanged; decided 2026-10-28, user; design
    doc, "The partition runner's construction order"). Before, the runner
    was built before `spawn`, with a placeholder share replaced in the
    task by `attach_share` (code change 55). Now `PartitionUnit::new`
    computes the runner's parts (`RunnerParts`, new private struct) where
    it built the runner, in the same order and with the same error; the
    spawn closure is `|recv| async move { run_partition_unit(parts,
    recv).await }`; `run_partition_unit` (new, **verified**) builds the
    runner with `PartitionUnitRunner::new(parts, &mut recv)` (new;
    contract: takes the channel's half; `external_body`) and runs it.
    `attach_share` and the placeholder in `PartitionUnit::new` are gone.
    The `async` block works around a Verus crash when an unverified
    closure returns a verified async fn's future directly.

57. **The input distributor's share starts as `None`** (executable change
    in `vmm_core`, behavior unchanged; decided 2026-10-28, user; design
    doc, "The input distributor's initial share"). `InputDistributor::new`
    builds `Inner` with the new `Inner::new()` (verified: `!running`,
    `ghost@ is None`) instead of a struct literal with the placeholder
    `Tracked::assume_new()`; the Verus-only `Inner::no_share()` (verified,
    in a `verus!` block) gives `Tracked(None)`. `Forwarder::new` gets a
    cfg'd `verus_verify(external_body)`. Not built inside the task like
    the partition runner (code change 56): the distributor's client is
    handed out before `spawn`. The order "take the share, then handle
    requests" is in `InputDistributor::run` (in Verus, body to prove).

58. **virtio-MMIO's interrupt status: a guest-memory boundary** (small
    executable changes in `virtio`, behavior unchanged; decided
    2026-10-28, user; design doc, "virtio-MMIO's interrupt status: the
    guest-memory boundary"). The microVM's virtio-MMIO devices use the
    shared-status interrupt mode, where the status is a word in guest
    memory. `MmioTransport::interrupt_status()` (uninterpreted) is that
    value, with trusted contracts (`external_body`) on
    `read_interrupt_status` (returns it) and the new
    `restore_interrupt_status` (sets it on `Ok`). Executable: `restore`'s
    inline block (lock, `restore_status`) moved into
    `restore_interrupt_status`, so that it can carry a contract. The lock protocol for the legacy modes (not the
    microVM's) stays a TODO, now with the acknowledgement-worker race.

59. **The hypervisor boundary** (Verus only; no executable change; design
    doc, "The hypervisor boundary"). `state_trait!`
    (`vmm_core/virt/src/state.rs`) emits, under `cfg(verus_keep_ghost)`,
    a module `state_spec` per invocation: the element types declared by
    name only, closed spec accessors on the saved-state type (`$get()`,
    `$set(v)`), and the trait's `external_trait_specification` with the
    extension `StateAccessSpec` (`hv_state()`, uninterpreted; trusted
    contracts on each getter, setter, and `commit`). The saved-state
    struct gets cfg'd `verus_verify` and `verifier::external_derive`.
    `std::error::Error` declared to Verus once (`state.rs`). Checked
    with a throwaway probe (getter keeps the state: verified; setter
    keeps the state: fails).

60. **The PIT's `SaveRestore` impl checked against the trait's contract**
    (Verus only; no executable change). It was plain Rust, so the
    contract was not attached to it (found while confirming which impls
    the contract enforces: a negative check, an `ExampleDevice` whose
    `restore` ignores its state, fails the trait's postcondition). Now
    `verus_verify`, with `save` / `restore` `external_body` (they call
    `save_device` / `restore_device`; proof hint in the code: reveal the
    PIT's closed `device_wf()` and `saved_view`).

61. **Gap G2 fixed: a chipset device unit reaches its device through a
    checked contract** (executable change in `vmotherboard`, behavior
    unchanged; design doc, "The trust audit"). `DynDevice` (private,
    `async_trait`) had `save` / `restore` without contracts and a plain
    blanket impl. They moved into a new private, non-async trait
    `DynDeviceState` (`DynDevice`'s supertrait, now carrying
    `DeviceSnapshot`), `verus_verify`, with contracts on the device's blob
    (`saved_blob()`, `device_wf()` from `DeviceSnapshot`: those of
    `ProtobufSaveRestore`). Its blanket impl for `T: VmmChipsetDevice` is
    `verus_verify` and verified (it calls `ProtobufSaveRestore::save` /
    `restore`; negative check: a `restore` returning `Ok` without
    restoring fails). `state_unit.proof.rs`: `ExDynDevice`'s supertrait is
    `DynDeviceState`. Not possible inside `async_trait`: Verus does not
    support its `dyn Future + Send` return type ("dyn with more than one
    trait").

62. **Gap G1 fixed: the erased chipset devices' blobs reach their units**
    (a dependency change in `chipset_device_resources`, behavior
    unchanged; design doc, "The trust audit"). The crate is
    Verus-enabled. `ErasedChipsetDevice` is `verus_verify` (with
    `verifier::external_derive`); its `ProtobufSaveRestoreSpecImpl` (in a
    `verus!` block next to its impl, since its field is private) makes its
    blob and `device_wf` its `dyn` device's; its `ProtobufSaveRestore`
    impl is `verus_verify` and verified. `From<T> for
    ResolvedChipsetDevice`, where devices are boxed, has a TRUSTED
    `external_body` contract: the erased device's blob and `device_wf`
    are `t`'s (not provable while `DynChipsetDevice` is `async_trait`,
    gap G5). `lib.proof.rs` declares `DynChipsetDevice`,
    `ResolvedChipsetDevice`, and (moved here from `vmotherboard`, which
    depends on this crate) `ChipsetDevice` and `ChangeDeviceState`.
    Also gap G5 logged: the `async_trait` glue, with the Verus patch's
    five pieces.

63. **The VP thread answers `Stop` through an `Rpc`** (executable change in
    `vmm_core`, behavior unchanged; design doc, "The VP protocol").
    `VpEvent::Stop(mesh::OneshotSender<()>)` becomes `Stop(Rpc<(), ()>)`,
    so that the reply can carry the VP's half of its FracGhost back to
    `VpSet`, as every other VP event's reply does. Senders: `VpSet::stop`
    and `VpSet::stop_at_io_boundary` (`vp_set/boundary.rs`) send
    `vp.send.call(VpEvent::Stop, ())` and await it with `.ok()` (before:
    a `mesh::oneshot()` pair, the sender in the event, the receiver
    awaited with `.ok()`); both ignore a VP whose runner is gone, as
    before. Receivers: the VP thread's two `Stop` arms (`run_inner`)
    answer `send.complete(())` (before: `send.send(())`); the boundary
    tests answer `.complete(())`. A mesh `Rpc` with input `()` and reply
    `()` is a one-shot reply channel, as the old sender was; same
    messages, same order.

64. **`vm_topology` is Verus-enabled** (dependency and build change,
    behavior unchanged). `[package.metadata.verus]`, dependency `vstd`
    (`Cargo.lock`: one line), `build.rs` declares the `verus_keep_ghost`
    cfg, `lib.rs` a cfg'd `vstd` import. Verus only: `VpIndex` is
    `verus_verify` (with `verifier::external_derive`), with a closed
    `spec_index()` and a contract on `index()` (returns `spec_index()`).
    `VpIndex::new` (a `const fn`, used by the `const` `BSP`) has no
    contract yet (since code change 66 it has one). Needed so that `VpSet::save` / `restore`'s contracts can
    name the VP indexes.

65. **`VpSet`'s contracts, the VP protocol, and the VP's FracGhost**
    (executable changes in `vmm_core`, behavior unchanged; design doc,
    "The VP protocol"). Executable:
    - `VpSet::restore` takes `states: Vec<(VpIndex, SavedStateBlob)>`
      instead of `impl IntoIterator<Item = ..>` (Verus cannot state a
      contract over an `impl IntoIterator`); its one caller
      (`PartitionUnitRunner::restore`) already passes a `Vec`. The body
      is unchanged (it passes `states` to `select_instantiated_vp_states`,
      which still takes `impl IntoIterator`).
    - `VpSet::add` creates the VP's channel with a new private function
      `vp_channel()` (in a `verus!` block; normal builds:
      `mesh::channel()`, through `mesh_rpc_verus::channel`) instead of
      `mesh::channel()`. The binding is `let (mut send, recv)` with
      `cfg_attr(not(verus_keep_ghost), expect(unused_mut))` (the `mut`
      is for Verus builds, which take the sender's half).
    - `vp_set.rs` imports `vstd::prelude::Ghost` and `Tracked` (used by
      `vp_channel` in normal builds too: zero-sized wrappers).
    Verus only: `vp_channel` creates the VP's FracGhost (`UnitSnapshot`,
    `Unknown`) and attaches one half to each end (as
    `state_unit::unit_channel`; verified); `Vp` gets a ghost field `share`
    (`VpSet`'s half, taken from the sender in `add`); `VpSet`, `Vp`,
    `VpEvent`, `StateEvent` are `verus_verify` (with
    `verifier::external_derive`); `vp_set.spec.rs` (for review): the VP
    protocol (`MeshProtocol for VpEvent`, `StateEvent::request_ok`), the
    view `VpSetView` (`started`, each VP's recorded `UnitSnapshot`;
    `inv`, `all_saved`, `saved_vps`), `VpSet::wf()`; `vp_set.proof.rs`:
    declarations. Contracts (`external_body`) on `add`, `start`, `halt`,
    `clear_halt`, `stop`, `reset`, `scrub`, `save`, `restore`,
    `restore_halt`, `set_initial_regs`; `validate_restore_halt` and
    `teardown` are `external_body` without contracts; `new` is
    `verus_verify(external)` (its `GuestMemory` parameter: virtio already
    declares `GuestMemory`, and a second declaration crashes Verus; since
    code change 67 it has a contract).
    `partition_unit.spec.rs`: the trusted, uninterpreted
    `VpSet::saved_vps()` is replaced by `VpSetView::saved_vps()`, defined
    from `VpSet`'s halves; `partition_unit.proof.rs` no longer declares
    `VpSet` by name only. `openvmm_core`'s `dispatch.proof.rs` no longer
    declares `mesh::RecvError` (now declared in `vmm_core`, upstream; two
    declarations are an error).

66. **`VpIndex::new` has a contract** (executable change in `vm_topology`,
    behavior unchanged). `VpIndex::new` (a `const fn`) moved, with the
    same body, into a `verus!` block (no longer cfg'd), next to
    `spec_index()`: in normal builds the block expands to the same
    `pub const fn new(index: u32) -> Self { Self(index) }`; in Verus
    builds `new` ensures `result.spec_index() == index` (verified).
    Reason: Verus's attribute form (`verus_verify` / `verus_spec`) does
    not accept a `const fn` (code change 64 left it without a contract).
    `VpSet::save`'s proof needs it. `BSP` (`Self::new(0)`) is unchanged.

67. **`guestmem` is Verus-enabled and declares `GuestMemory`** (dependency
    and build change, behavior unchanged). `[package.metadata.verus]`,
    dependency `vstd` (`Cargo.lock`), new `build.rs` (declares the
    `verus_keep_ghost` cfg). Verus only: a cfg'd module
    `verus_declarations` at the end of `lib.rs` declares `GuestMemory` to
    Verus (by name only), once for every crate that names it. virtio's
    declaration (`transport/mmio.proof.rs`) is removed: two declarations
    of a type crash Verus. With it, `VpSet::new` gets its contract
    (`external_body`): `wf()`, not started, no VPs (code change 65 had
    left it `verus_verify(external)`).

68. **Contracts on the rest of `VpSet`'s microVM functions** (Verus only;
    no executable change). `external_body`, to prove:
    `stop_at_io_boundary` (`vp_set/boundary.rs`): as `stop`, also on
    `Err`; `check_one_shot_timers`, `advance_lapic_timers`
    (`vp_set/time_abi.rs`): `wf()` kept and the view unchanged (no ghost
    travels; the LAPIC timers are out of the saved-state views, the time
    rule (b)); `get_dump_vp_state` (`dump` feature): `external_body`, no
    contract (`&self`). `vp_set.proof.rs` declares `mesh::OneshotSender`.
    The `gdb` methods stay unseen (out of scope).

69. **The partition's saved VPs are a set of (index, blob) pairs**
    (Verus only; no executable change; decided by the user, 2026-10-28:
    first a map by index, then a set; design doc, "The VP protocol",
    concern C2). `PartitionSavedView::vps` becomes
    `Set<(u32, SavedStateBlobView)>` (was `Seq<(u32, SavedStateBlobView)>`):
    `state::Partition`'s view is the set of its list's (index, blob)
    pairs, and the partition unit's saved view uses
    `VpSetView::saved_vps().to_set()` (`saved_vps()` stays the sequence in
    index order, what `VpSet::save` returns). Reason: `restore` accepts
    the VPs in any order and `save` returns index order; with a sequence,
    a snapshot in another order made the partition unit's restore
    contract false. The order carries no meaning, so the view leaves it
    out. A set rather than a map: no arbitrary choice for an index listed
    twice (such a list keeps all its pairs; restore rejects it anyway).
    `partition_unit.rs`'s saved-state view module gets a cfg'd
    `use vstd::prelude::*;`.

70. **The partition unit after a restore of a VP prefix** (Verus only; no
    executable change; decided by the user, 2026-10-28; design doc, "The
    VP protocol", concern C3). With MSHV's restore-time VP prefix
    (`active_vp_count`, set when the partition unit is created) the unit
    instantiates VPs `0..k` of a topology of `capacity` VPs; `restore`
    takes a snapshot with all `capacity` VPs and keeps those below `k`;
    `save` then fails (`validate_save_vp_count`, deliberate). The generic
    state-unit protocol is unchanged (restore `Ok` records the given
    blob; save `Ok` returns the recorded one, vacuous here). Changed: the
    partition unit's `wf()` (`partition_unit.spec.rs`) has two cases: all
    VPs instantiated: the recorded blob is its saved state's, as before;
    fewer: the recorded blob is that of some saved state `v` whose
    `v.restrict_vps(k)` (new, on `PartitionSavedView`: only the VPs below
    `k`) is the unit's saved state. `VpSetView` gains `capacity`
    (`vp_capacity`); `new` ensures it, and the contracts that keep the VP
    count keep it too.

71. **`VpSet` sends its VP events through the mesh RPC wrapper**
    (executable change in `vmm_core` and `mesh_rpc_verus`, behavior
    unchanged; design doc, "The VP protocol", open item 3). As code
    change 31 for the state units. Each call site, `vp.send.call(f, x)
    .await` becomes `mesh_rpc_verus::call(&vp.send, f, x, ghost, ghost)
    .recv().await.map(|(r, _ghost)| r)`; each `vp.send.call_failable(f,
    x).await` the same with `.recv_failable()`; `vp.send.send(
    VpEvent::Start)` becomes `mesh_rpc_verus::send_msg(&vp.send,
    VpEvent::Start, ghost)`. Sites: `VpSet::start`, `stop`, `reset`,
    `scrub`, `save`, `restore`, `set_initial_regs`, `get_dump_vp_state`
    (`vp_set.rs`), `stop_at_io_boundary` (`vp_set/boundary.rs`),
    `check_one_shot_timers`, `advance_lapic_timers` (`vp_set/time_abi.rs`).
    Not changed: the `gdb` methods (out of scope), `Inspect` (through the
    derive). At run time the wrapper calls `mesh`'s own functions (`call`
    is `send.call(f, input)`, sending at once as before, which
    `stop_at_io_boundary` relies on: every VP has a queued `Stop` before
    the I/O is released; `recv` awaits it; `recv_failable` maps a remote
    error to `RpcError::Call` as `call_failable` does; `send_msg` is
    `send.send(msg)`): same messages, same order, same errors. Ghosts:
    placeholders (`Tracked::assume_new()`, `Ghost::assume_new()`) in the
    `external_body` bodies, to be replaced by the real halves (`Vp.share`)
    when the methods are proved. New in `mesh_rpc_verus`: `send_msg`
    (TRUSTED, `external_body`: requires `request_ok`; trusted doc, "Mesh
    RPC"), and a test `send_msg_arrives` (the test message type gains a
    one-way variant `Note(u32)`; `call_round_trip`'s server ignores it).
    Imports: `boundary.rs` and `time_abi.rs` drop `mesh::rpc::RpcSend` and
    import `vstd::prelude::Ghost` and `Tracked`; in `vp_set.rs`
    `RpcSend` is now imported only with the `gdb` feature (its only
    remaining users).

72. **OpenVMM's lint passes again** (`cargo xtask fmt --pass lints`;
    a dependency change, behavior unchanged; decided by the user,
    2026-10-28). Executable (dependencies only, no code):
    `closeable_mutex` removed from the dependencies of `vmm_core`,
    `state_unit`, `openvmm_core`, `chipset`, `chipset_device_fuzz`, and
    `vmbus_ring` from `vmm_core` (`Cargo.lock`: six lines). None is used
    any more: code change 44 replaced the direct `CloseableMutex` uses
    with `vmcore::device_state::DeviceMutex` (`closeable_mutex` stays in
    the build through `vmcore`); `chipset`'s and `state_unit`'s were
    added for Verus and are not referenced; `vmbus_ring` (code change 39)
    is no longer referenced. Not executable: two `// UNSAFETY:` comments
    (`state_unit/src/lib.rs`, `openvmm_defs/src/rpc.rs`: the Verus-only
    modules whose `assume_specification`s expand to `unsafe`
    declarations); `verus-patches/README.md`'s trailing newlines; two new
    `.ignore` files that the lint's file walker honors:
    `verus-patches/.ignore` skips the vendored Verus macro crate
    (`verus_builtin_macros/`, third-party code under the Verus project's
    license) and the RPC prototype's `Cargo.toml` and
    `rust-toolchain.toml`; `scripts/.ignore` skips
    `verus-trust-classify.py`. Files written for this work get no
    Microsoft copyright header (the user: they are under a University of
    Chicago license), so they are skipped rather than given one.

73. **Gap G3: the partition's VM state, through a state token** (small
    executable change in `virt` and `openvmm_core`, behavior unchanged;
    decided by the user, 2026-10-28: option B, then option 1; design doc,
    "The partition's state"). Executable:
    - `virt` (`generic.rs`): two new public functions,
      `open_vm_state(partition, vtl)` (normal builds:
      `partition.access_state(vtl)`) and `close_vm_state(access)` (normal
      builds: `drop(access)`), with TRUSTED contracts (`external_body`):
      the handle starts with the state the partition's token records and
      holds the token; closing it returns the token with the handle's
      final state.
    - `openvmm_core` (`partition.rs`): the blanket
      `BasicPartitionStateAccess` impl's `save` and `restore` open the
      handle with `open_vm_state` into a named variable, call
      `save_all` / `restore_all`, close it with `close_vm_state`, then
      apply `.context(..)?` (before: one statement on a temporary from
      `access_state`, dropped at the statement's end). Same calls, same
      order; the handle is now dropped before `.context` (which only wraps
      the error). `WrappedPartition`'s `save` / `restore` bind the inner
      call's result before `map_err` (`let r = ..; r.map_err(..)`).
      `partition.rs` imports `vstd::prelude::*` in both builds (for
      `proof_with!`, which normal builds erase).
    Verus only: `PartitionAccessState` is `verus_verify` (an
    `external_trait_specification` with its generic associated type
    crashes the compiler); the boundary macro (`state_trait!`) gains a
    handle's `hv_id()`, `overwrite` on the saved-state type, and
    contracts on `save_all` / `restore_all` (assumed until proved; trusted
    doc); `VmSavedState` (x86) has a real view, `VmSavedStateView`
    (`hypercall`, `reference_tsc_page`; `reftime` left out: the time
    rule, (a)); `BasicPartitionStateAccess` is `verus_verify`, `save` /
    `restore` take the token as a ghost parameter, with contracts (save
    returns the token's state and keeps it; restore overwrites it); the
    blanket impl's bodies are `external_body` (`anyhow`'s `Context` is not
    supported), its other methods `external_body` without contracts;
    `WrappedPartition` holds the token (ghost field `hv`, created by the
    TRUSTED `new_vm_state_token` in `into_vm_partition`), has the view
    `VmSavedStateView` (the token's state) and a `SaveRestoreSpecImpl`
    (saved view the identity; `device_wf`: it holds the whole token), and
    its `SaveRestore` impl is `verus_verify` (bodies `external_body`,
    passing the token). `virt/src/generic.spec.rs` (new; no header,
    skipped by the lint through `vmm_core/virt/src/.ignore`) declares
    `Vtl` (transparent, moved from `vmm_core`'s `vp_set.proof.rs`),
    `InitialVpStateSource`, `InitialPageImport`, `Hv1`, `Partition`,
    `StateError`.

74. **No assumed specifications for our own registry code** (a small
    executable change in `state_unit`, behavior unchanged; decided by the
    user, 2026-10-30, while reviewing what is trusted). `registry_t.rs`
    held three `assume_specification`s, a tool meant for library
    functions; all three were about our own code. Now:
    - `Inner::ready_set`: `verus_verify(external_body)` with the same
      contract (to prove; its `impl` block is `verus_verify`).
    - `ReadySet`'s `Clone`: EXECUTABLE, `#[derive(Clone)]` replaced by the
      same impl written out (`ReadySet(self.0.clone())`, what the derive
      generates), `verus_verify` with the contract "a clone holds the same
      units"; **verified** (through vstd's spec of `Arc::clone`).
    - `State`'s `PartialEq`: the `assume_specification` is dropped; no
      verified code compares `State`s (`run_op`, `start_with_rollback`
      are plain, `resume_after_failed_save` is `external_body`). When one
      does, Verus may handle the derived `==`, or the impl is written out.
    - `registry_t.rs` is deleted, with its `allow(unsafe_code)` and
      `UNSAFETY:` comment; `lib.rs`'s crate attribute is
      `#![forbid(unsafe_code)]` in both builds (before: `forbid` in normal
      builds, `deny` in Verus builds, for `assume_specification`; normal
      builds unchanged).
    Also (2026-10-30): `StateRequest::apply` is `external_body` (Verus
    only), instead of verified with an `assume` in its `Stop` arm.

75. **The locks' unverified users are `external_body`** (Verus only; no
    executable change; decided by the user, 2026-10-30). Instead of rows
    assuming that unverified code keeps the locks' invariants, the
    functions that lock (or create) a device's `CloseableMutex` or the
    registry's `Mutex` are `verus_verify(external_body)`, without a
    contract, to prove like the others. `CloseableMutex`'s and `mutex_t`'s
    own specifications stay trusted (lock primitives we do not verify).
    Marked: `vmotherboard`'s `Chipset::mmio_read`, `mmio_write`,
    `io_read`, `io_write`, `acknowledge_pic_interrupt`, `handle_eoi`;
    `IoRanges::lookup`; `LineSetTargetDevice::set_irq` (its `impl` is
    `verus_verify`); `ArcMutexChipsetServices::finalize` (likewise);
    `ArcMutexChipsetDeviceUnit::inspect_mut` (likewise); `state_unit`'s
    `UnitHandle::remove_if`, `Inspect` for `StateUnits` and
    `StateUnitsInspector`, `StateUnits::run_op`, `run_input_op`,
    `start_with_rollback`. Declared to Verus (by name only) for their
    signatures: `IoRanges`, `LookupResult`, `LineSetTargetDevice`,
    `PciConflict`, `ArcMutexChipsetServices`, `FinalizeError`
    (`vmotherboard`, next to their definitions); `UnitHandle`,
    `StateUnitsInspector` (`state_unit`'s `lib.proof.rs`);
    `inspect::Request` (`vmcore`); the PCI configuration-space types
    (`chipset_device_resources`). Not marked: `UnitBuilder::build` and
    `ArcMutexChipsetDeviceBuilder::inner_add` / `add*` (they take
    `mut self`: needs an executable change, to decide), and the paths out
    of the microVM's scope (PCI, PCIe, hotplug, `vpci`; `Weak<T>`, which
    the PCI wrappers use, cannot be declared with this vstd).

76. **`self` instead of `mut self` in six builder functions, so that they
    can be `external_body`** (EXECUTABLE, behavior unchanged; decided by
    the user, 2026-10-30; completes code change 75). Verus rejects a
    `mut self` parameter even in an `external_body` signature. In
    `state_unit`'s `UnitBuilder::build` and `vmotherboard`'s
    `ArcMutexChipsetDeviceBuilder::inner_add`, `add`, `add_async`, the
    parameter is now `self`, the body starts with `let mut this = self;`,
    and every `self` in the body is `this` (as in code change 47). Same
    statements, same order; `#[instrument(.. fields(device =
    self.dev_name ..))]` still reads the parameter `self`. The four are
    then `verus_verify(external_body)` (no contract, to prove); declared
    to Verus (by name only) for their signatures: `UnitBuilder`, `NameInUse`
    (`state_unit`'s `lib.proof.rs`), `ArcMutexChipsetDeviceBuilder`,
    `AddDeviceError` (next to their definitions), and the trait
    `address_filter::RangeKey` (for `IoRanges::lookup`'s `impl`).
    Not changed: `try_add` and `try_add_async`: their bounds name
    `dyn std::error::Error + Send + Sync`, which Verus cannot express (a
    `dyn` of more than one trait, as in gap G5), so they stay plain and
    are recorded as deferred. Remaining Verus warning: `add_async`'s bound
    `AsyncFnOnce` is an undeclared std trait.
77. **Skipped: the PCI and PCIe paths' lock users, for now** (no code
    change; decided by the user, 2026-10-30, with code change 75).
    Not marked `external_body`: `vmotherboard`'s `base_chipset.rs`,
    module `weak_mutex_pci` (`WeakMutexPciDeviceWrapper`'s
    `GenericPciBusDevice` impl: `pci_cfg_read`, `pci_cfg_write`,
    `pci_cfg_read_with_routing`, `pci_cfg_write_with_routing`; the two
    `RegisterWeakMutexPci::add_pci_device` impls); `openvmm_core`'s
    `dispatch.rs` (`WeakMutexPciBusDevice`'s `GenericPciBusDevice` impl;
    the PCIe root complexes' and switches' operations, including hotplug);
    `ChipsetDevices::add_dyn_device` (PCIe hotplug, `vpci`); `vmm_core`'s
    `device_builder.rs` (`vpci`). Reasons: (1) the microVM has no PCI or
    PCIe devices (its inventory: IOAPIC, PIC, PIT, portb, shutdown,
    snapshot-request, and virtio-MMIO devices), so these are not on its
    save/restore path; (2) the PCI wrappers hold
    `std::sync::Weak<DeviceMutex<..>>`, and this vstd provides no
    declaration of `Weak<T>` (std's `Weak<T, A: Allocator>` needs the
    unstable `allocator_api` to declare). **May be revisited**: declaring
    `Weak<T>` (a vstd addition or a local declaration with the allocator
    parameter) would let these be marked like the others.

78. **Three of our types' views defined from their fields, instead of
    uninterpreted** (Verus only; no executable change; decided by the
    user, 2026-10-30: avoid uninterpreted views of our own types, which
    keep the code using them from being verified).
    - `QuiesceForSaveError` (`state_unit`, `quiesce.rs`): the struct is
      `verus_verify` (with `verifier::external_derive`); its view, next to
      it, is `uncertain: self.uncertain`. `has_uncertain_state` is now
      **verified** (its body was `external_body`). Its by-name-only declaration
      in `lib.proof.rs` is removed.
    - `SnapshotRestore` (`openvmm_core`, `dispatch/restore.rs`): the
      struct is `verus_verify`; view `clock_restore_pending:
      self.time_abi is Some`. Declared for its fields: `std::fs::File`,
      `pal_async::timer::Instant` (`dispatch.proof.rs`), `StopGuard`
      (`vmm_core`'s `partition_unit.proof.rs`).
    - `SnapshotBoundary` (`openvmm_core`, `dispatch/microvm.rs`): the
      struct is `verus_verify`; view `active: self.stop_guard is Some`.
      Declared: `MicrovmSnapshotScratchPolicy`.
    The uninterpreted views in `dispatch.spec.rs` and `lib.spec.rs` are
    removed. **Not done, by the user's decision**: `VmTimeKeeper`'s view
    (`vmcore`) stays uninterpreted; defining it needs two new `vmcore`
    dependencies used only for Verus (`async-task`, `mesh_rpc_verus`), and
    guest time is out of scope; its methods' contracts stay assumptions
    (trusted doc, Part 3).

79. **Inspect writes out of scope** (docs and a comment only; decided by
    the user, 2026-10-30). `inspect_unit`'s contract (an inspect keeps the
    unit's `wf()`, `id()`, `running()`) holds only for read-only
    inspects; the partition unit's `"clear_halt"` inspect write clears
    its `halt_reason`, part of its saved state. Inspect writes are a
    debugging interface and out of scope, like `gdb` (design doc, "Out of
    scope for now", with the microVM's units checked); read-only inspects
    stay to prove. `inspect_unit`'s comment and two trusted-doc rows say
    so.

80. **`poll_now` is a lock user, not an assumption** (docs and a comment
    only; decided by the user, 2026-10-30). Its comment said "ASSUMED: it
    keeps the unit's `wf()`", and the trusted doc had a row for it; but it
    has no contract (it takes `&self`, so the verified `run` loop needs
    nothing from it), and the change it makes is inside the device's lock.
    It is one of "The locks' unverified users" (code change 75): to prove
    through `lock_verified` / `release` and `poll_device`'s contract. The
    comment says so; the separate row is removed.

81. **Explicit `PROOF DEFERRED` markers** (comments and docs only; asked
    by the user, 2026-10-30). Every deferred item now says, in its
    comment, `PROOF DEFERRED: do not attempt to prove this until ...`:
    `ArcMutexChipsetDeviceUnit::next_event` (until Verus can state
    postconditions on futures, G5; then rewrite `select!` with
    `futures::future::select` and an `assume_specification`); `From<T> for
    ResolvedChipsetDevice` (until G5's patch); `try_add`, `try_add_async`
    (until Verus supports a `dyn` of more than one trait); the PCI lock
    users (`base_chipset.rs`'s `weak_mutex_pci`, `dispatch.rs`'s
    `WeakMutexPciBusDevice`; until `Weak<T>` can be declared);
    `VmTimeKeeper`'s `save`, `restore`, `advance`, `reset`, `start`,
    `stop` (until its view is defined from its fields). `view_str` is
    labelled deferred (until vstd has a view of `Arc<str>`).

82. **The trust boundary moves to the hypervisor; the partition's state
    handles in a `_t.rs` file** (a small executable change in `virt`,
    behavior unchanged; decided by the user, 2026-10-30).
    - EXECUTABLE: `open_vm_state` and `close_vm_state` (code change 73)
      moved, unchanged, from `virt/src/generic.rs` into a new module
      `virt/src/generic/generic_t.rs` (`mod generic_t;` in `generic.rs`,
      re-exported with `pub use generic_t::*;`, so `virt::open_vm_state`
      and `virt::close_vm_state` keep their paths; a glob, so that Verus's
      companion functions for the ghost parameters are exported too).
      Verus only: `new_vm_state_token` moved there from `generic.spec.rs`.
      The file is headed "TRUSTED, do not attempt to prove anything in
      this file" (thin wrappers: `access_state`, a drop); "called once
      per partition" stays a PROOF TODO. No header otherwise (skipped by
      the lint, `vmm_core/virt/src/.ignore`).
    - Spec: `state_trait!`'s getters', setters', and `commit`'s contracts
      are no longer trusted: they are obligations of each backend's
      implementation, to prove against the trusted contracts of its
      hypervisor API (a `_t.rs` file per backend, to write: KVM and MSHV;
      WHP and hvf out of scope). Reason (the user): the trust boundary is
      the hypervisor, not OpenVMM's backends, which have significant code
      (e.g. KVM's VM state works around a KVM bug and converts the
      reference time; MSHV's builds register arrays). Trusted doc: the row
      moved from Part 1 to Part 3.

83. **virtio-MMIO's interrupt status: the transport's functions are to
    prove, not trusted** (comments and docs only; decided by the user,
    2026-10-30). `MmioTransport::read_interrupt_status` and
    `restore_interrupt_status` keep their contracts but are now labelled
    PROOF TODO, not trusted: they are our code (a lock, the
    acknowledgement, the two modes). The proof needs trusted contracts on
    guest-memory access (`GuestMemory::read_plain`, `compare_exchange`;
    not written yet) and a lock invariant on `InterruptState`, which is
    left to the proof (it may need a lock with an invariant).
    `interrupt_status()` stays uninterpreted for now, labelled a PROOF
    TODO (to define from the transport's state); only the guest's
    behavior stays trusted.

84. **virtio-MMIO's interrupt status as a ghost field** (Verus only; no
    executable change; decided by the user, 2026-10-30). The
    uninterpreted `MmioTransport::interrupt_status()` is removed: the
    device's view read it, but an unknown value cannot be related to the
    real status (a word in guest memory, in the microVM's mode), so the
    save/restore contract on it could never be proved. Instead
    `MmioTransport` has a Verus-only ghost field `ghost_status: Ghost<u32>`
    (`#[inspect(skip)]`), which the view reads; its one construction site
    (`VirtioMmioDevice::new`) sets it with a placeholder (PROOF TODO: the
    word's value at creation); the test helpers are not built for Verus.
    `read_interrupt_status` and `restore_interrupt_status` lose their
    contracts (`external_body`, PROOF TODO: specify and prove them,
    keeping `ghost_status` in step). `mmio.rs` imports `vstd::prelude::Ghost`
    in Verus builds.

85. **The backends' VM-state access trusted as thin wrappers; VP-state
    access deferred** (comments and docs only; decided by the user,
    2026-10-30; revises part of code change 82). Code change 82 made all
    of `state_trait!`'s contracts obligations of the backends. Looking at
    the code: KVM's and MSHV's VM-state access (`vm_state.rs`, about 70
    and 100 lines) are thin wrappers over the hypervisor's calls (MSR or
    register reads and writes through the boot VP), so they are trusted
    with the hypervisor, avoiding a ghost token for the hypervisor's
    MSRs and a new handle type. Their VP-state access (`vp_state.rs`,
    about 700 and 550 lines) is significant code: obligations, PROOF
    DEFERRED until gap G4. `state.rs`'s comment says both; the trusted
    doc has one row for each (Part 1, Part 3).

86. **Encoding functions into their `_t.rs` files; the vmbus spec marked
    out of scope** (Verus only; no executable change; decided by the
    user, 2026-10-30).
    - The uninterpreted encoding functions moved in with their trusted
      contracts: `SavedStateBlobView::of`, `of_view`, `View for
      SavedStateBlob` (from `save_restore.spec.rs` to `save_restore_t.rs`);
      `ProtobufMessageView::of` and `ProtobufMessage`'s `encoded` (from
      `rpc.spec.rs` to `rpc_t.rs`); `PartitionSavedView::encode_halt` (from
      `partition_unit.spec.rs` to a new `vmm_core/src/partition_unit_t.rs`,
      included as a Verus-only `mod trusted` in `partition_unit.rs`; no
      header, skipped by the lint through a new `vmm_core/src/.ignore`).
    - The ten files with vmbus spec (`vmbus_channel`'s `lib.rs`,
      `channel.rs`, `channel.spec.rs`, `channel.proof.rs`, `simple.rs`,
      `simple.spec.rs`, `simple.proof.rs`; `vmm_core`'s `vmbus_unit.rs`,
      `vmbus_unit.spec.rs`, `vmbus_unit.proof.rs`) are headed
      `// OUT OF SCOPE (vmbus): ... Do not attempt to prove anything
      here.` The audit script counts functions in files so headed as out
      of scope. The trusted doc has a new "Out of scope" section; the
      vmbus rows moved there. Not reverted (the user): the `StateUnit`
      trait requires the vmbus units to follow its protocol.

87. **`PartitionRequest`'s protocol: stop tokens** (Verus only, except
    for `StopGuard`'s two construction sites' syntax; decided by the
    user, 2026-10-30; design doc, "The partition unit's request
    protocol").
    - `PartitionRequest` and `SnapshotRequest` are `verus_verify`; the
      protocol (`MeshProtocol for PartitionRequest`,
      `partition_unit.spec.rs`): `Const` is the stop-token authority's id,
      the ghosts are `Option<GhostSingleton<u64>>`; `StopVps` and
      `StopVpsAtIoBoundary` reply with a new stop token (the latter also on
      `Err`); `StartVps` gives one back; the time-ABI and boot requests
      cannot be sent by verified code. Declared: `InitialRegError`,
      `AcceptInitialPagesError`.
    - `PartitionUnitRunner` gets a Verus-only field `stop_tokens:
      Tracked<GhostSetAuth<u64>>`, set in `new` with a placeholder (PROOF
      TODO); its `wf()` gains: the authority's id is `req_recv`'s
      constant; one outstanding token per stop reference; the VPs are
      stopped while any is outstanding.
    - `StopGuard` gets a Verus-only second field, its stop token, and is
      `verus_verify` (its by-name-only declaration removed); `StopGuard::wf()`.
      Its two construction sites (`temporarily_stop_vps`,
      `temporarily_stop_vps_at_io_boundary`) are written
      `StopGuard { 0: .., #[cfg(verus_keep_ghost)] 1: .. }` instead of
      `StopGuard(..)` (EXECUTABLE in form only: the same value; a tuple
      struct's constructor call cannot take a cfg'd argument), the token a
      placeholder (PROOF TODO).
    - Markers: the partition unit's `StopVps` placeholders change from
      WAITS FOR PROTOCOL to PROOF TODO. 18 unmarked placeholders were
      found once the audit script also scans files that use `vstd`
      without Verus attributes: `snapshot.rs` (4 PROOF TODO for
      `StopVpsAtIoBoundary`, 8 OUT OF SCOPE for the time-ABI requests)
      and `openvmm_entry`'s `vm_controller/microvm.rs` (6 WAITS FOR
      PROTOCOL, `VmRpc`).

88. **A patched vstd: `Arc<str>` has a view** (a dependency change:
    `vstd` now comes from a patched copy; decided by the user,
    2026-10-30). Unit names are `Arc<str>`, and the spec compares them
    (snapshots are keyed by name); vstd gives `Arc<T>` a view only for
    sized `T`, so the spec used an uninterpreted stand-in,
    `ArcStrView::view_str`. Now `verus-patches/vstd` is the pinned vstd
    (`=0.0.0-2026-10-04-0306`) with
    `verus-patches/vstd-0001-arc-unsized-view.patch` (`View` and
    `DeepView` for `Arc<A>` with `A: ?Sized`), used through
    `[patch.crates-io]` (`Cargo.toml`; excluded from the workspace;
    skipped by the lint; `Cargo.lock` changes the source of `vstd`).
    `ArcStrView` is removed; `state_unit`'s spec uses `name@`.
    Side effect: vstd's own warnings (13) showed in builds, as for any
    path dependency; `verus-patches/vstd-0002-allow-warnings.patch`
    (`#![allow(warnings)]` in `vstd.rs`) silences the 10 lint warnings;
    3 remain, rustc's "negative impls are experimental" (`impl !Sync` /
    `!Send` in vstd's `invariant.rs` and `thread.rs`), which are not lints.
    README: "vstd patches".

89. **`VmRpc`'s protocol: the state units' halves go to the controller**
    (decided by the user, 2026-10-30; design doc, "The VM worker's
    request protocol"). After a `QuiesceForSnapshot` that leaves the units
    stopped, the worker lends the state units' halves to the controller;
    `ResumeAfterFailedSnapshot` brings them back. While the controller
    holds them, no unit can change its state.
    - `VmRpc`'s channel is created through the mesh RPC wrapper: new
      `openvmm_defs::rpc::vm_rpc_channel()` (normal builds:
      `mesh::channel()`, through `mesh_rpc_verus::channel`), called in
      `openvmm_entry`'s `lib.rs` instead of `mesh::channel()`
      (EXECUTABLE: the same channel). In Verus builds it creates the
      protocol's ledger (a `GhostVarAuth` / `GhostVar` pair, no units
      lent), attached to the receiver; the channel's constant is its id.
      Verified. New imports in `rpc.rs`: `vstd::prelude::Ghost` and
      `Tracked` in normal builds, the rest cfg'd. A new trust-base row
      (`SNAPSHOT-SPEC-TRUSTED.md`, mesh): the receiver reaches the VM
      worker through its launch parameters, possibly in another process,
      with its constant and attached ghost.
    - `openvmm_defs` (`rpc.spec.rs`): the protocol (`request_ok` was
      uninterpreted); `SnapshotHalves` (the halves with the ledger's
      `GhostVar`); `VmRpc` declared with its variants (was declared by
      name only), so the protocol can `match` on it: all 22 variants
      listed, the 19 not specified mapped to `false` (`Pause`, `Resume`,
      `Reset` next). The types inside its variants that Verus did not
      know are declared by name only (`external_body`, no claims):
      `DeviceVtl`, `PulseSaveRestoreError`, `Guid`, `CancelContext`,
      `File`, `UnixStream`, `Resource` (and its trait `ResourceKind`), the
      two handle kinds. `File`'s declaration moves here from `openvmm_core`'s
      `dispatch.proof.rs` (Verus allows one).
    - `state_unit`: the view gains `missing_halves` (the collected units
      whose half `StateUnits` does not hold) and `holds_halves()`. `wf()`
      no longer says "stopped ⇒ holds every half" (false while lent):
      `stop` and `quiesce_for_save` ensure `holds_halves()`; `reset`,
      `restore`, `advance_time`, `resume_after_failed_save` require and
      keep it. `collect_halves` (verified) ensures no half goes missing,
      its proof adjusted. New `LentHalves` (spec) and the Verus-only
      `StateUnits::lend_halves` / `return_halves` (`external_body`,
      PROOF TODO).
    - `openvmm_core`: `LoadedVm`'s view gains `runtime.missing_halves`;
      `load` ensures the halves are held; `restore`,
      `restore_from_snapshot`, `finish_snapshot_restore`,
      `time_abi_restore`, `resume_after_failed_snapshot` require it;
      `quiesce_and_save` and `quiesce_for_snapshot` ensure it on `Ok` and
      `RollbackSafe`; `establish_snapshot_boundary`,
      `release_snapshot_boundary`, `handle_snapshot_boundary` keep the
      halves where they are. Verus-only `LoadedVm::lend_unit_halves` /
      `return_unit_halves` (`external_body`, PROOF TODO), for the request
      loop.
    - Markers: the 6 `VmRpc` placeholders (the worker's request loop, the
      controller) change from WAITS FOR PROTOCOL to PROOF TODO, naming the
      ghosts to pass.
    - Wording (comments and docs only; the user: misleading next to
      Verus's `opaque` keyword, which hides a spec function's body): a type
      declared with `external_type_specification` + `external_body` is
      now called "declared by name only (fields hidden)", not "opaque",
      in all our comments and docs (87 lines). OpenVMM's own uses of the
      word are unchanged.

90. **`Pause`: its protocol case and `LoadedVm::pause`'s contract**
    (planned with the user, 2026-10-30; design doc, "Roadmap", step 1).
    Verus only. `VmRpc::Pause`: nothing sent, nothing in the reply (the
    worker keeps the halves). `LoadedVm::pause` gets a contract (the result
    is whether the units were running; if so, afterwards they are stopped,
    recorded, and the state units hold every half, from
    `StateUnits::stop`'s contract; otherwise nothing changes), its body
    `external_body`, to prove. `Resume` and `Reset` remain next.

91. **`StopGuard`'s `Drop` sends through the mesh RPC wrapper** (approved
    by the user, 2026-10-30). A `StopGuard` is a hold on the VPs: while it
    exists they stay stopped; dropping it (Rust's `Drop`, when the guard
    is destroyed) sends `StartVps`, and the runner runs the VPs again once
    no guard holds them. EXECUTABLE: `self.0.send(PartitionRequest::StartVps)`
    becomes `mesh_rpc_verus::send_msg(&self.0, PartitionRequest::StartVps,
    Tracked::assume_new())`, as `VpSet::start` (code change 71): the same
    message at the same moment (`send_msg` is `send.send(msg)`). The ghost
    is a placeholder (PROOF TODO): the guard's stop token, which `drop`
    must take out of `&mut self` (e.g. an `Option` field). Needed for
    `Resume` (design doc, "Roadmap", step 2), which releases the restore's
    guard. Checks: `vmm_core` spec-checks; workspace builds; `vmm_core`
    tests pass (40).

92. **`LoadedVm`'s boundary guard belongs to its own partition unit**
    (asked by the user, 2026-10-30, who chose to make `LoadedVmInner`
    visible to Verus over a ghost field or moving the field). Verus only.
    - `LoadedVmInner` is `verus_verify` (was declared by name only), so
      specs can name its fields; the types of its fields that Verus did
      not know are declared by name only (`dispatch.proof.rs`: 29 types,
      among them `GuestMemoryManager`, `ChipsetDevices`, `IgvmFile`,
      `ProcessorTopology`, the cfg'd `TimeAbiState` and the two VFIO
      clients), and the traits behind its trait objects and generic
      fields (`HvlitePartition` with its supertraits `RequestYield` and
      `MicrovmPartition`, `ArchTopology`, std's `AsRef`; no claims).
      `openvmm_core` enables `sized_hierarchy` in Verus builds (for
      `AsRef`'s `PointeeSized` bound). `state_unit` declares
      `SpawnedUnit`; `vmm_core`'s `PartitionUnit` is `verus_verify`.
    - New spec functions: `PartitionUnit::id()` (its request channel's
      id), `StopGuard::partition_id()`, `SnapshotBoundary::wf(partition)`
      (an active boundary's guard is well formed and holds that
      partition unit's VPs). `LoadedVm::wf()` adds
      `snapshot_boundary.wf(inner.partition_unit.id())`.

93. **Two small spec items** (asked by the user, 2026-10-30). Verus only.
    - The restore's guard belongs to its own partition unit:
      `SnapshotRestore::wf(partition)` (the guard, while held, is well
      formed and holds that partition unit's VPs stopped);
      `LoadedVm::wf()` adds `snapshot_restore.wf(inner.partition_unit.id())`,
      as for the boundary's guard (code change 92).
    - A convention TODO: `SavedStateUnit::map_of(units)` and
      `distinct_names(units)` (which took a sequence instead of `self`)
      become `units@.by_name()` and `units@.distinct_names()`, methods of
      the new spec trait `SavedStateUnits`, implemented for
      `Seq<SavedStateUnit>`; their uses (`StateUnits::save`,
      `StateUnits::restore`, `SavedState`'s view and `wf`) updated.

94. **Each lock's own invariant, as in vstd** (decided by the user,
    2026-10-31, rejecting the per-type invariant plus key). Both lock
    wrappers now give each lock its own invariant, chosen when it is
    created: the invariant's form comes from a type, and each lock has its
    own ghost parameter, which the invariant takes; the keys are gone.
    - `mutex_t::Mutex<V, Pred>` (the registry's lock): `MutexPredicate`
      has `type Param` and `inv(param, v)` (was `Key`, `key(v)`,
      `inv(v)`); `m.param()`; new `new_verified(v, Ghost(param))`
      (requires `Pred::inv(param, v)`); `acquire` / `release` use
      `Pred::inv(m.param(), _)`. `Mutex::new(v)` stays for unverified code,
      hidden from Verus (EXECUTABLE in form only: moved out of the
      `verus!` block, the same body and signature). The registry's
      `InnerPred`: parameter = the id of the registry's half of the shared
      record; its invariant is "that half has this id" and
      `Inner::registry_inv()` (the former invariant, now a spec function
      on `Inner`, which `insert_unit` / `remove_unit` keep, still verified,
      their proofs unchanged). `StateUnits::wf()` uses `inner.param()`.
    - `CloseableMutex<T>` (each chipset device's lock): `LockInvariant` has
      `type Param` and `inv(&self, param)` (was `Key`, `key()`, `inv()`);
      `m.param()` and the guards' `param()`; `new_verified(value,
      Ghost(param))` (EXECUTABLE: the extra parameter, erased at runtime;
      called only by tests, now passing `Ghost::assume_new()`). The type
      `CloseableMutex<T>` is unchanged.
    - `DeviceSlot`: new closed spec functions `share_id()` and `omit()`;
      its `LockInvariant`: parameter `(id, omit)`, the invariant says the
      slot's share has id `id` and its `omit` is `omit`, plus the device's
      `device_wf()` and the share's rule. `DeviceSlot::new`'s contract and
      `ArcMutexChipsetDeviceUnit`'s spec (`id()`, `wf()`, `new`) use the
      lock's `param()`.
    - The example (`closeable_mutex/src/lib.proof.rs`): a counter whose
      limit is the mutex's parameter, with a verified `example_new`.
    - The two creators on the microVM's path, both unverified, keep
      calling `new` (parameter unknown): `StateUnits::new` (the registry)
      and `ArcMutexChipsetDeviceBuilder::inner_add` (every chipset
      device's lock). Each has a PROOF TODO comment naming the
      `new_verified` call and the parameter to pass once it is verified
      (the registry's record id; the device's ghost id and `omit`). The
      other creators (hot-plug, the fallback "missing device", OpenHCL,
      VPCI, tests) are not on the path.
    - Docs: `SNAPSHOT-SPEC-MUTEX.md` (how to use, API, design choices),
      `SNAPSHOT-SPEC-TRUSTED.md`, the overview's 4.2.2, notes in the
      design doc.

95. **`CloseableMutex`'s panics as preconditions: the close token** (asked
    by the user, 2026-10-31). A closed `CloseableMutex` panics on `lock`,
    and closing a closed one panics; the trusted spec said nothing about
    it, so a verified caller could panic without Verus noticing (as if
    `Vec` indexing had no bound). Now each mutex has a close token, a
    `FracGhost<()>` with id `close_id()`:
    - `new_verified` hands it out whole (returns `(mutex, token)`);
      `close_verified(m, token)` requires the whole token and the closed
      guard keeps it (`VerifiedClosedGuard::close_id()`);
      `VerifiedClosedGuard::release` gives it back (returns
      `(mutex, token)`); `lock_verified(&self, token)` requires a piece
      (`token@.id() == close_id()`). Whoever holds a piece knows nobody
      holds the whole, so the mutex is not closed: neither call can panic.
      EXECUTABLE in form: the new ghost parameters and results, erased at
      runtime; only tests call these functions (updated, placeholders
      marked OUT OF SCOPE). The trusted bodies hand out the token with
      `Tracked::assume_new()` (marked TRUSTED).
    - The example: `example_new` returns the token, `example_increment`
      closes with it and gets it back, new `example_read` locks with a
      piece (verified; negative check: without the piece, its
      `lock_verified` call fails).
    - Who gives the pieces to the code that locks a device (the vCPUs'
      accesses, the unit's poll) is for the proof of that code; it needs
      the order in which units stop and start. `inner_add`'s PROOF TODO
      comment mentions the token.
    - Docs: `SNAPSHOT-SPEC-TRUSTED.md` (the `CloseableMutex` rows), the
      overview's 4.2.2 ("Closing"), notes in the design doc.

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
