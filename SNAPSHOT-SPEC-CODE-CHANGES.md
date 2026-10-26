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

### Possibly needed

From the gaps in the design doc; not started.

- `async fn` in traits ("does not yet support Opaque types in trait def"):
  `StateUnit`.
- Async closures ("does not yet support coroutine closure types"); avoided
  for now by OpenVMM change 5 above.
- Datatype constructors as function values; avoided by OpenVMM change 6.
