// Spec of `LxVolume`'s paths, for human review: what virtio-fs must hand
// to the volume (each method's precondition, in `lib.rs`).
//
// TRUSTED, in words only (no contract states it, since the spec has no model
// of the host): a call whose paths meet `is_share_path` touches only objects
// beneath the volume's root, and follows no symbolic link on the way (but a
// final one, for the methods about links: `lstat`, `read_link`). On Linux
// the kernel guarantees it: every path is resolved relative to the root's
// descriptor by `openat2` with `RESOLVE_BENEATH | RESOLVE_NO_SYMLINKS`
// (`unix/util.rs`), whatever is on disk. On Windows, the volume's own path
// handling and virtio-fs's walk of each path before it is used
// (`validate_confined_paths`) do it; not verified. Limits (`LxVolume`'s doc):
// mount points inside the share and existing hard links are not restricted.
// See `virtiofs-spec-docs/CODE-OVERVIEW.md`, property 1 (confinement).

use super::LxVolume;
use vstd::prelude::*;
use vstd::std_specs::path::PathView;

verus! {

impl LxVolume {
    /// A path that the volume resolves beneath its root: relative, and every
    /// component a plain name (`path::validate_relative_path` accepts only
    /// normal components). The empty path is the root itself.
    pub open spec fn is_share_path(p: PathView) -> bool {
        forall|i: int| 0 <= i < p.len() ==> Self::is_plain_component(#[trigger] p[i])
    }

    /// One plain name: not empty, no `/` and no NUL, not `.` or `..` (a
    /// root, `.` and `..` are components of these forms in a path's view).
    pub open spec fn is_plain_component(c: Seq<u8>) -> bool {
        &&& c.len() > 0
        &&& !c.contains('/' as u8)
        &&& !c.contains(0u8)
        &&& c != seq!['.' as u8]
        &&& c != seq!['.' as u8, '.' as u8]
    }
}

} // verus!
