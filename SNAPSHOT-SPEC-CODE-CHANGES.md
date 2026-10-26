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
7. **Ghost field `LoadedVm::halves`** (`dispatch.rs`): the FracGhost halves
   `LoadedVm` holds, by unit name. Declared and initialized (in `make_ids`,
   via `proof::no_halves()`) only under `#[cfg(verus_keep_ghost)]`, so a
   normal build is unchanged. `LoadedVm` also gets
   `#[cfg_attr(verus_keep_ghost, verus_verify)]`, so that Verus sees its
   fields.

### Planned: mesh RPC trusted spec

From the draft in `verus-patches/verus-rpc-prototype/src/mesh_rpc_t.rs`
(see "Mesh RPC trusted spec" in the design doc). Not yet made.

All in `vmm_core/state_unit`:

1. **New file `src/mesh_rpc_t.rs`**: the trusted wrappers, moved from the
   prototype. **New file `src/mutex_t.rs`**: the trusted `Mutex` wrapper,
   moved from the prototype; `StateUnits::inner` and `UnitHandle::inner`
   change from `parking_lot::Mutex<Inner>` to it (`lock()` call sites
   unchanged). `LoadedVm::halves` (made, item 7) is then removed.
2. **Verus wiring** for `state_unit`, as for the crates above.
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
