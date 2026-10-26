# Snapshot spec: code changes

Code changes made for the snapshot save/restore verification, to OpenVMM and
to Verus. Design rationale is in `SNAPSHOT-SPEC-DESIGN.md`.

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

### Possibly needed

From the gaps in the design doc; not started.

- `async fn` in traits ("does not yet support Opaque types in trait def"):
  `StateUnit`.
- Async closures ("does not yet support coroutine closure types"); avoided
  for now by OpenVMM change 5 above.
- Datatype constructors as function values; avoided by OpenVMM change 6.
