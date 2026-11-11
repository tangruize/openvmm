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

/// `parking_lot`'s lock types, declared to Verus by name only, so that the
/// views of structs that hold one (`Session`, virtio-fs's inode) can read
/// their other fields.
#[cfg(verus_keep_ghost)]
#[verifier::external_type_specification]
#[verifier::external_body]
#[verifier::reject_recursive_types(R)]
#[verifier::reject_recursive_types(T)]
pub struct ExRwLock<R, T: ?Sized>(parking_lot::lock_api::RwLock<R, T>);

#[cfg(verus_keep_ghost)]
#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExRawRwLock(parking_lot::RawRwLock);

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

/// What the `Fuse` trait asks of a file system's ghost state `G` itself (not
/// of its view): that it is well formed. As for any concrete type, `wf()`
/// includes its view's invariant (`self@.inv()`), and the consistency of
/// what the view does not show (in virtio-fs, the ids of its pieces).
pub trait FuseGhost: View + Sized where <Self as View>::V: FuseState {
    spec fn wf(&self) -> bool;
}

/// What the `Fuse` trait asks of a file system about a ghost state `g`:
/// that `g` is this file system's (in virtio-fs, that `g`'s pieces are the
/// ones its locks hold).
pub trait FuseOwns<G> {
    spec fn owns(&self, g: &G) -> bool;
}

/// A file system with no ghost state (the `fuse` crate's tests, `G = ()`)
/// owns the one there is.
impl<T> FuseOwns<()> for T {
    open spec fn owns(&self, g: &()) -> bool {
        true
    }
}

/// `()` has no pieces: it is always well formed.
impl FuseGhost for () {
    open spec fn wf(&self) -> bool {
        true
    }
}

/// The questions that the operations' rules ask about a filesystem's state.
/// `Sized`, so that the `after_` functions can return the state.
pub trait FuseState: Sized {
    /// The state's invariant: what every operation keeps.
    spec fn inv(&self) -> bool;

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

    /// Nodes `a` and `b` are in the same share.
    spec fn same_share(&self, a: u64, b: u64) -> bool;

    /// The state after path `p` becomes one more name of node `m` (a hard
    /// link): `p` among its names; its main path stays.
    spec fn after_link(&self, m: u64, p: PathView) -> Self;

    /// `after` is the state once every name at or below path `p` of node
    /// `n`'s share is gone (`unlink`, `rmdir`): each node loses those names,
    /// and one whose main path was among them takes one of its remaining
    /// names as its main path, if it has any.
    spec fn unlinked(&self, n: u64, p: PathView, after: Self) -> bool;

    /// `after` is the state once path `p` of node `n`'s share is renamed to
    /// `q`: with `exchange`, the names at or below `p` and `q` swap;
    /// without, the names at or below `q` are gone (as `unlinked`), and then
    /// those at or below `p` move to the same place below `q`.
    spec fn renamed(&self, n: u64, p: PathView, q: PathView, exchange: bool, after: Self) -> bool;

    /// The state after node `n` is given back (`forget`, when its count
    /// reaches zero): the guest no longer holds it.
    spec fn after_forget(&self, n: u64) -> Self;

    /// The state after `destroy` (the guest unmounted): no handle, and no
    /// node but the root.
    spec fn after_destroy(&self) -> Self;

    /// `fh` is an open handle the guest holds.
    spec fn holds_file(&self, fh: u64) -> bool;

    /// The state after handle `fh` is released: it is no longer held (no
    /// change if it was not).
    spec fn after_release(&self, fh: u64) -> Self;

    /// The node of the file open as `fh`.
    spec fn file_node(&self, fh: u64) -> u64;

    /// The file open as `fh` was opened for writing.
    spec fn opened_for_writing(&self, fh: u64) -> bool;

    /// The guest may change the file open as `fh`: through every name of
    /// its node.
    spec fn file_can_write(&self, fh: u64) -> bool;

    /// The state after handle `fh` is opened on node `m`, for writing or not.
    spec fn after_open(&self, fh: u64, m: u64, writable: bool) -> Self;

    /// `n` is a node the file system made up, not a host object, which the
    /// guest may not change (virtio-fs: an aggregate's root).
    spec fn is_synthetic(&self, n: u64) -> bool;

    /// `code` is an error code the host may fail with.
    spec fn host_error(code: i32) -> bool;

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

    /// Open flags `flags` truncate the file (`O_TRUNC`, 0o1000).
    open spec fn truncates(flags: u32) -> bool {
        flags & 0o1000u32 != 0u32
    }

    /// `rename` flags `flags` ask to swap the two names (Linux's
    /// `RENAME_EXCHANGE`, 2).
    open spec fn is_exchange(flags: u32) -> bool {
        flags & 2u32 != 0u32
    }

    /// Open flags `flags` open for writing: their access mode (`O_ACCMODE`,
    /// the low two bits) is `O_WRONLY` (1) or `O_RDWR` (2). (Mode 3 neither
    /// reads nor writes.)
    open spec fn opens_for_writing(flags: u32) -> bool {
        flags & 3u32 == 1u32 || flags & 3u32 == 2u32
    }

    /// A request about node `n` may use handle `fh`: the guest holds both,
    /// and `fh` is `n`'s (a handle is not used after its node is given back,
    /// nor with another node's id).
    open spec fn uses_handle(&self, n: u64, fh: u64) -> bool {
        &&& self.holds_node(n)
        &&& self.holds_file(fh)
        &&& self.file_node(fh) == n
    }

    /// The state after the lookups `steps` in directory `dir`, one after
    /// the other: each is a name, the node handed out for it, and its file
    /// type.
    open spec fn after_lookups(&self, dir: u64, steps: Seq<(Seq<u8>, u64, lx::mode_t)>) -> Self {
        steps.fold_left(*self, |st: Self, step: (Seq<u8>, u64, lx::mode_t)|
            st.after_reach(dir, st.child_path(dir, step.0), step.1, step.2))
    }

    /// `after` is the result of lookups in directory `dir` (`read_dir_plus`
    /// looks up each entry it lists), each one allowed by `lookup`'s rules:
    /// a plain name of the directory, at a path the guest may reach.
    open spec fn looked_up_in(&self, dir: u64, after: Self) -> bool {
        exists|steps: Seq<(Seq<u8>, u64, lx::mode_t)>| {
            &&& #[trigger] self.after_lookups(dir, steps) == after
            &&& forall|i: int| 0 <= i < steps.len() ==> {
                let (name, m, kind) = #[trigger] steps[i];
                &&& self.is_entry(dir, name)
                &&& self.may_reach(dir, self.child_path(dir, name), kind)
            }
        }
    }

    // When an operation fails. Each operation's rules are a set of codes:
    // each rule is a condition and its code, and the set holds the codes of
    // the rules that apply. The order in which the code checks them does not
    // matter: when several apply, the operation fails with one of their
    // codes (`fails_with`).

    /// The general rule: if any of the rules applies (`errors` holds a
    /// code), the operation fails, with the code of one of the rules that
    /// apply, or with an error of the host's (which may fail first).
    open spec fn fails_with<T>(r: Result<T, lx::Error>, errors: ISet<i32>) -> bool {
        (exists|c: i32| #[trigger] errors.contains(c)) ==> match r {
            Ok(_) => false,
            Err(e) => errors.contains(e.code()) || Self::host_error(e.code()),
        }
    }

    /// The rules of `unlink` and `rmdir` of `name` in directory `n`.
    open spec fn unlink_errors(&self, n: u64, name: Seq<u8>) -> ISet<i32> {
        ISet::new(|c: i32| {
            ||| self.is_synthetic(n) && c == lx::EROFS
            ||| !self.holds_node(n) && c == lx::EINVAL
            ||| self.holds_node(n) && {
                ||| !self.node_can_write(n) && c == lx::EROFS
                ||| !self.is_dir(n) && c == lx::ENOTDIR
                ||| !Self::is_plain_name(name) && c == lx::EINVAL
                ||| !self.can_write(n, self.child_path(n, name)) && c == lx::EROFS
            }
        })
    }

    /// The rules of `rename` of `name` in directory `n` to `new_name` in
    /// directory `m`.
    open spec fn rename_errors(&self, n: u64, name: Seq<u8>, m: u64, new_name: Seq<u8>) -> ISet<i32> {
        ISet::new(|c: i32| {
            ||| (self.is_synthetic(n) || self.is_synthetic(m)) && c == lx::EROFS
            ||| (!self.holds_node(n) || !self.holds_node(m)) && c == lx::EINVAL
            ||| self.holds_node(n) && self.holds_node(m) && {
                ||| !self.same_share(n, m) && c == lx::EXDEV
                ||| (!self.node_can_write(n) || !self.node_can_write(m)) && c == lx::EROFS
                ||| (!self.is_dir(n) || !self.is_dir(m)) && c == lx::ENOTDIR
                ||| (!Self::is_plain_name(name) || !Self::is_plain_name(new_name)) && c == lx::EINVAL
                ||| (!self.can_write(n, self.child_path(n, name))
                    || !self.can_write(m, self.child_path(m, new_name))) && c == lx::EROFS
            }
        })
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
    open spec fn inv(&self) -> bool {
        true
    }

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

    open spec fn file_can_write(&self, fh: u64) -> bool {
        false
    }

    open spec fn opened_for_writing(&self, fh: u64) -> bool {
        false
    }

    open spec fn file_node(&self, fh: u64) -> u64 {
        0
    }

    open spec fn after_release(&self, fh: u64) -> Self {
        ()
    }

    open spec fn after_forget(&self, n: u64) -> Self {
        ()
    }

    open spec fn after_destroy(&self) -> Self {
        ()
    }

    open spec fn same_share(&self, a: u64, b: u64) -> bool {
        false
    }

    open spec fn unlinked(&self, n: u64, p: PathView, after: Self) -> bool {
        false
    }

    open spec fn renamed(&self, n: u64, p: PathView, q: PathView, exchange: bool, after: Self) -> bool {
        false
    }

    open spec fn after_link(&self, m: u64, p: PathView) -> Self {
        ()
    }

    open spec fn after_open(&self, fh: u64, m: u64, writable: bool) -> Self {
        ()
    }

    open spec fn is_synthetic(&self, n: u64) -> bool {
        false
    }

    open spec fn host_error(code: i32) -> bool {
        true
    }
}

} // verus!
