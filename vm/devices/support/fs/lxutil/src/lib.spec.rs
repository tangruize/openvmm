// Spec of `LxVolume`'s paths, for human review: what virtio-fs must hand
// to the volume (each method's precondition, in `lib.rs`).
//
// TRUSTED, in words only (no contract states it, since the spec has no model
// of the host): a call whose paths meet `is_share_path` touches only objects
// beneath the volume's root, and follows no symbolic link on the way (but a
// final one, for the methods about links: `lstat`, `read_link`). On Linux
// the kernel guarantees it: every path is resolved relative to the root's
// descriptor by `openat2` with `RESOLVE_BENEATH | RESOLVE_NO_SYMLINKS`
// (`unix/util.rs`), whatever is on disk. On Windows (decided 2026-11-10 to be
// in the claim), the NT kernel does: every path is opened relative to the
// root's handle with `OBJ_DONT_REPARSE`, which refuses any reparse point on
// the way (NT symbolic links, junctions, mount points;
// `windows/util.rs`, `open_relative_file`), and `rename` and `link` name
// their target as one component of a parent opened that way. Trusted: that
// `lxutil` always uses these flags (its code is not verified). Limits
// (`LxVolume`'s doc): mount points inside the share (on Linux) and existing
// hard links are not restricted.
// ASSUMED (a host assumption, decided 2026-11-10): a share contains no mount
// point (on Linux; on Windows they are reparse points, refused), so "beneath the root" is "inside the share" (only the host's setup
// can put one there; a question for Pedro, `BUGS.md`).
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
