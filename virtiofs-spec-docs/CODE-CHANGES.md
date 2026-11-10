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
| 9 | `vm/devices/support/fs/fuse/Cargo.toml` | depends on `vstd`; `[package.metadata.verus] verify = true` | the `Fuse` trait's contracts. A normal build of `fuse` now compiles `vstd` as well |
| 10 | `vm/devices/support/fs/fuse/src/lib.rs` | `pub trait Fuse` becomes `pub trait Fuse<G: View>: Send + Sync where G::V: FuseState`; a `pub mod spec` (`lib.spec.rs`, `FuseState`) included in every build | the ghost state passed to each operation, as the trait's type parameter (`CODE-OVERVIEW.md`, "Why a type parameter, and what it costs"); `Send + Sync` moves from `Session`'s field to the trait, since Verus refuses `dyn Fuse + Send + Sync` |
| 11 | `vm/devices/support/fs/fuse/src/session.rs`, `session/saved_state.rs`, `conn.rs`, `tests/fuse_hello.rs` | `Session` becomes `Session<G>` (field `Box<dyn Fuse<G>>`); `Connection::run` takes `Fuse<G>`; the crate's test file systems implement `Fuse<()>` | the same; `()` is the ghost state of a file system with none (`impl FuseState for ()`, which permits nothing) |
| 12 | `vm/devices/virtio/virtiofs/src/lib.rs`, `section.rs`, `virtio.rs`, `microvm/owner.rs`, `microvm/device.rs`, `microvm/fs.rs`, `microvm/owner_tests.rs` | `impl Fuse for ...` becomes `impl Fuse<spec::VirtioFsGhost> for ...` (`VirtioFs`, `SectionFs`, `DormantMicrovmFs`); `fuse::Session` becomes `fuse::Session<spec::VirtioFsGhost>`; a `pub mod spec` (`lib.spec.rs`) included in every build | the same; the ghost state's types must exist in a normal build. Nothing changes at runtime |
| 13 | `vm/devices/virtio/virtiofs/src/microvm/policy.rs` | the `spec` module (`policy.spec.rs`) included in every build (its `proof` module still under `cfg(verus_keep_ghost)`) | `SubtreePolicyView` is a field of the ghost state (an inode's view), which a normal build compiles |
| 14 | `verus-patches/vstd/vstd.rs` (patch `vstd-0005`) | in a normal build, `std_specs::path::PathView` (the alias alone) | the ghost state's types name it, in a normal build too; only the patched `vstd` is affected |
| 15 | `vm/devices/virtio/virtiofs/src/lib.rs` (and three test call sites) | **a fix, not for Verus**: `get_file` takes the request's node id, and refuses a handle unless the guest still holds that node and the handle is that node's | `BUGS.md`, bug 1: a handle kept working after the guest gave its node back, with stale names; patch `patches/0001-virtiofs-a-handle-only-with-its-node.patch` |
