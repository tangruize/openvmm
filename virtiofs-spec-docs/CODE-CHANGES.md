# virtio-fs spec: changes to OpenVMM's code

Every change that the virtio-fs spec work makes to what a normal build
compiles (code, types, dependencies, build settings), numbered ("code
change N"). Annotations under `cfg_attr(verus_keep_ghost, ...)` and code
under `cfg(verus_keep_ghost)` are not listed: a normal build does not see
them. The base is Pedro Henrique Penna's `feature/microvm/dev` at
`db42fde04`.

| # | Where | Change | Why |
|---|---|---|---|
| 1 | `Cargo.toml` (workspace) | `[patch.crates-io]` points `verus_builtin_macros` and `vstd` to the patched copies in `verus-patches/`, which are excluded from the workspace | the patches the spec needs (`verus-patches/README.md`) |
| 2 | `Cargo.toml` (workspace) | the workspace dependency `vstd` (`=0.0.0-2026-10-04-0306`, feature `std`) | the first crate that uses Verus, `virtiofs` (change 3) |
| 3 | `vm/devices/virtio/virtiofs/Cargo.toml` | depends on `vstd`; `[package.metadata.verus] verify = true` | the access policy's spec (`microvm/policy.spec.rs`). A normal build now compiles `vstd` as well |
| 4 | `Cargo.toml` (workspace) | `cfg(verus_keep_ghost)` added to the `unexpected_cfgs` lint's `check-cfg` list | the condition under which the spec is compiled; without it, every annotation gives a warning in a normal build |
| 5 | `verus-patches/vstd/std_specs/` | new `path.rs` (patch `vstd-0003`): `Path` and `PathBuf` declared to Verus, with their view (the path's components) | Rust's orphan rule forbids implementing `vstd`'s `View` for `std`'s types in `virtiofs`; only the patched `vstd` is affected |
| 6 | `verus-patches/vstd/std_specs/` | `convert.rs` and `path.rs` (patch `vstd-0004`): a spec of `AsRef::as_ref`, for `&T`, `Path` and `PathBuf` | `lxutil`'s methods take `impl AsRef<Path>`, and their contracts must name the path they are given; only the patched `vstd` is affected |
| 7 | `vm/devices/support/fs/lx/Cargo.toml` | depends on `vstd`; `[package.metadata.verus] verify = true` | the view of `LxStr` (`src/string.spec.rs`), the bytes of a name in the FUSE operations' contracts. A normal build of `lx` now compiles `vstd` as well |
| 8 | `vm/devices/support/fs/lx/src/string.rs` | `LxStr` declared to Verus (`cfg_attr(verus_keep_ghost, ...)` only), and a `mod spec` under `cfg(verus_keep_ghost)` | the same; a normal build does not see either |
