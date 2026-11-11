// Spec of `LxVolume`'s paths, for human review: what virtio-fs must hand
// to the volume (each method's precondition, in `lib.rs`).
//
// CONFINEMENT, in the contracts (with the host model, `host_t.rs`, since
// 2026-11-11): every object a call reads, opens or changes is one on the
// share's disk (`lstat` reports the object at the share path, `open` opens
// it, and each change is stated exactly on the disk).
// TRUSTED, in words only: a call changes nothing outside the share, and
// follows no symbolic link on the way (but a final one, for the methods
// about links: `lstat`, `read_link`). The host model covers only the share,
// so no contract can mention what is outside it. It could be turned into
// a contract: a trusted, declared-only value `HostOutside` (the host outside
// every share) that every `lxutil` method takes by shared reference, which
// Verus then holds unchanged across each call. Not done (decided
// 2026-11-11): the trust would be the same (`lxutil`'s bodies are not
// verified, so its contracts are believed either way), and no rule reads
// the outside's state; it becomes worth doing if `lxutil` is verified
// against specs of the kernel's calls, when that contract becomes a proof
// obligation. On Linux
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
// ASSUMED (a host assumption, decided 2026-11-11): on Windows, two paths
// that differ name for name are two objects. `open_relative_file` opens
// every path with `OBJ_CASE_INSENSITIVE` (`windows/util.rs`), so in a
// directory that is not case-sensitive, a name in another case (`SECRET`
// for `secret`), or an 8.3 short name (`SECRE~1`), reaches the same object.
// The policy compares names exactly, so without the assumption a guest
// could remove or rename a hidden file by such a name (a question for
// Pedro, `BUGS.md`).
// TRUSTED, the host model (`host_t.rs`, written 2026-11-11): what a share
// holds on the host disk (the object at each path), a disk value per volume
// that the volume's methods take, and each method's contract in terms of it
// (in `lib.rs`). For example, `mkdir_stat` makes the directory and then
// stats it by name (on Linux `mkdirat`, then `fstatat`); under the host
// assumptions, the name still names the new object, so the stat is its.
// ASSUMED (a host assumption, decided 2026-11-11, with the host model): no
// two volumes share a host directory, so only a volume's own calls change
// what its disk holds.
// ASSUMED (a host assumption about the share as attached, decided
// 2026-11-11): no host hard link joins a path the guest may write and a
// visible path it may not (Pedro's setup rule), and a directory has one
// path. Unlike the others, these are about what the disk holds, not how the
// host behaves, so they are an axiom (`host_t.rs`, `attached_disk`, which
// says why).
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

    /// The host's form of a guest's name, as a component of a path that
    /// `PathBufExt::push_lx` builds (`path.rs`). On Linux, the name itself.
    #[cfg(unix)]
    pub open spec fn host_name(name: Seq<u8>) -> Seq<u8> {
        name
    }

    /// The host's form of a guest's name, as a component of a path that
    /// `PathBufExt::push_lx` builds (`path.rs`, `windows/path.rs`,
    /// `path_from_lx`). On Windows, each byte (`host_bytes`) in turn.
    #[cfg(windows)]
    pub open spec fn host_name(name: Seq<u8>) -> Seq<u8>
        decreases name.len(),
    {
        if name.len() == 0 {
            seq![]
        } else {
            Self::host_name(name.drop_last()) + Self::host_bytes(name.last())
        }
    }

    /// One byte of a name in the host's form, on Windows: a byte that NTFS
    /// does not allow in a name (`needs_escape`) becomes the UTF-8 encoding
    /// of the private-use character U+F000 plus the byte; any other byte
    /// stays.
    #[cfg(windows)]
    pub open spec fn host_bytes(c: u8) -> Seq<u8> {
        if Self::needs_escape(c) {
            seq![0xefu8, (0x80 + c / 64) as u8, (0x80 + c % 64) as u8]
        } else {
            seq![c]
        }
    }

    /// A byte that NTFS does not allow in a name, by lxutil's table
    /// (`NTFS_LEGAL_ANSI_CHARACTERS`): a control character, `"`, `*`, `:`
    /// (which names an alternate data stream), `<`, `>`, `?`, `\` or `|`.
    /// (`/` is not escaped: it separates components.)
    #[cfg(windows)]
    pub open spec fn needs_escape(c: u8) -> bool {
        ||| c < 0x20
        ||| c == '"' as u8
        ||| c == '*' as u8
        ||| c == ':' as u8
        ||| c == '<' as u8
        ||| c == '>' as u8
        ||| c == '?' as u8
        ||| c == '\\' as u8
        ||| c == '|' as u8
    }
}

} // verus!
