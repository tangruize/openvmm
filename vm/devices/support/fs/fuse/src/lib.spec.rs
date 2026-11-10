// High-level spec of a FUSE filesystem, as the guest sees it, for human
// review.
//
// The `fuse` crate defines no data for the filesystem's state: only the
// questions that the operations' rules ask about it, as spec functions
// without bodies (`FuseState`). Each filesystem answers them from its own
// types (virtio-fs: `virtiofs/src/lib.spec.rs`). The `Fuse` trait takes the
// filesystem's ghost state as a type parameter, `G`, whose view answers
// `FuseState`, and states each operation's rules with those functions:
// integrity (a change succeeds only through a path the guest may write) and
// confidentiality (a node is handed out only for a path the guest may see).

use vstd::prelude::*;
// Used only by spec functions, which a normal build removes.
#[cfg(verus_keep_ghost)]
use vstd::std_specs::path::PathView;

verus! {

/// std's `BorrowedFd` (the `Mapper` trait's `FileRef` on Unix), declared to
/// Verus by name only.
#[cfg(all(verus_keep_ghost, unix))]
#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExBorrowedFd<'a>(std::os::fd::BorrowedFd<'a>);

/// A request's decoded operation and arguments (a `Request`'s field),
/// declared to Verus by name only: the contracts do not look inside it.
#[cfg(verus_keep_ghost)]
#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExFuseOperation(crate::FuseOperation);

/// The questions that the operations' rules ask about a filesystem's state.
/// `Sized`, so that the `after_` functions can return the state.
pub trait FuseState: Sized {
    /// `n` is a node the guest holds.
    spec fn holds_node(&self, n: u64) -> bool;

    /// `n` is a node the guest holds, and a directory.
    spec fn is_dir(&self, n: u64) -> bool;

    /// The path, in its share, of `name` in directory `parent`.
    spec fn child_path(&self, parent: u64, name: Seq<u8>) -> PathView;

    /// The guest may change path `p` of node `n`'s share.
    spec fn can_write(&self, n: u64, p: PathView) -> bool;

    /// The guest may change node `n` itself: through every name of it.
    spec fn node_can_write(&self, n: u64) -> bool;

    /// The guest may reach an object of file type `kind` at path `p` of
    /// `parent`'s share, by the path rules. (The rule on pinned identities,
    /// about which object is at `p`, waits for a model of the host.)
    spec fn may_reach(&self, parent: u64, p: PathView, kind: lx::mode_t) -> bool;

    /// The state after node `m` is handed out for the object at path `p` of
    /// `parent`'s share, of file type `kind`: an existing node gets `p` as
    /// its main path and among its names; a new one starts with `p` alone.
    spec fn after_reach(&self, parent: u64, p: PathView, m: u64, kind: lx::mode_t) -> Self;

    /// `fh` is an open handle the guest holds.
    spec fn holds_file(&self, fh: u64) -> bool;

    /// The state after handle `fh` is opened on node `m`, for writing or not.
    spec fn after_open(&self, fh: u64, m: u64, writable: bool) -> Self;

    // The rules built from the questions above, the same for every
    // filesystem: spec functions with bodies, which every implementor gets.

    /// One plain name: not empty, no `/` and no NUL, not `.` or `..`.
    open spec fn is_plain_name(name: Seq<u8>) -> bool {
        &&& name.len() > 0
        &&& !name.contains('/' as u8)
        &&& !name.contains(0u8)
        &&& name != seq!['.' as u8]
        &&& name != seq!['.' as u8, '.' as u8]
    }

    /// `name` in `parent` names an entry: `parent` is a directory the guest
    /// holds, and `name` is one plain name.
    open spec fn is_entry(&self, parent: u64, name: Seq<u8>) -> bool {
        self.is_dir(parent) && Self::is_plain_name(name)
    }

    /// Open flags `flags` open for writing: their access mode (`O_ACCMODE`,
    /// the low two bits) is not read-only (`O_RDONLY`, 0).
    open spec fn opens_for_writing(flags: u32) -> bool {
        flags & 3u32 != 0u32
    }

    /// The common rule of the operations that make a new name `name` in
    /// `parent` (`create`, `mkdir`, `mknod`, `symlink`, `link`): it names an
    /// entry, `parent` is a directory the guest may change, and the new
    /// name's path is writable.
    open spec fn may_add_name(&self, parent: u64, name: Seq<u8>) -> bool {
        &&& self.is_entry(parent, name)
        &&& self.node_can_write(parent)
        &&& self.can_write(parent, self.child_path(parent, name))
    }
}

/// The state of a file system that has no ghost state (the `fuse` crate's
/// tests, `Fuse<()>`): it holds nothing, and permits nothing, so a verified
/// implementation of `Fuse<()>` could succeed at no operation that the
/// rules restrict.
impl FuseState for () {
    open spec fn holds_node(&self, n: u64) -> bool {
        false
    }

    open spec fn is_dir(&self, n: u64) -> bool {
        false
    }

    open spec fn child_path(&self, parent: u64, name: Seq<u8>) -> PathView {
        seq![]
    }

    open spec fn can_write(&self, n: u64, p: PathView) -> bool {
        false
    }

    open spec fn node_can_write(&self, n: u64) -> bool {
        false
    }

    open spec fn may_reach(&self, parent: u64, p: PathView, kind: lx::mode_t) -> bool {
        false
    }

    open spec fn after_reach(&self, parent: u64, p: PathView, m: u64, kind: lx::mode_t) -> Self {
        ()
    }

    open spec fn holds_file(&self, fh: u64) -> bool {
        false
    }

    open spec fn after_open(&self, fh: u64, m: u64, writable: bool) -> Self {
        ()
    }
}

} // verus!
