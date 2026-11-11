// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

//! A Rust helper library for creating File-system in User Space (FUSE) daemons, aimed in
//! particular at virtio-fs.

#![expect(missing_docs)]

#[cfg(unix)]
mod conn;
pub mod protocol;
mod reply;
mod request;
mod session;
mod util;

#[cfg(target_os = "linux")]
pub use conn::Connection;
pub use reply::DirEntryWriter;
pub use reply::ReplySender;
pub use request::FuseOperation;
pub use request::Request;
pub use request::RequestReader;
pub use request::check_name;
pub use session::Session;
pub use session::SessionInfo;
pub use session::saved_state::SessionInfoState;
pub use session::saved_state::SessionState;
pub use session::saved_state::SessionStateError;

use lx::LxStr;
use lx::LxString;
use protocol::*;
use spec::FuseGhost;
use spec::FuseOwns;
use spec::FuseState;
use std::time::Duration;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;
use zerocopy::FromBytes;
use zerocopy::FromZeros;
use zerocopy::Immutable;
use zerocopy::IntoBytes;
use zerocopy::KnownLayout;

/// Verus specification of a FUSE filesystem's state: the questions that the
/// operations' rules ask about it. Included in every build, not only under
/// `cfg(verus_keep_ghost)`: the `Fuse` trait's bound names `FuseState`, so a
/// normal build needs it (its spec functions are removed there).
pub mod spec {
    include!("lib.spec.rs");
}

/// Reply data for the `create` operation.
///
/// The `create` operation includes two values in its reply, but fuse.h has no wrapper for the
/// combination of these values as they're just passed as separate arguments to `fuse_reply_create`
/// in libfuse.
#[cfg_attr(verus_keep_ghost, verus_verify)]
#[cfg_attr(verus_keep_ghost, verifier::external_derive)]
#[repr(C)]
#[derive(Debug, IntoBytes, Immutable, KnownLayout, FromBytes)]
pub struct CreateOut {
    pub entry: fuse_entry_out,
    pub open: fuse_open_out,
}

/// Trait that FUSE file systems must implement.
///
/// Most operations are loosely based on `fuse_lowlevel_ops` in libfuse, so check the [official
/// libfuse documentation](http://libfuse.github.io/doxygen/index.html) for more information on how
/// these operations should behave.
///
/// For many operations, a reply of `ENOSYS` is taken as permanent failure, preventing the client
/// from ever issuing that operation again.
///
/// `G` is the file system's ghost state (Verus only; nothing of it exists at
/// runtime), lent to each operation: the operations' contracts are stated on
/// its view, which answers [`FuseState`]'s questions.
///
/// Errors. Each operation's contract says when it may succeed; an error
/// leaves the state unchanged (`read_dir_plus` keeps the lookups it did
/// before one). An operation may fail when:
/// - its rules (its contract's "succeeds only if") do not hold, e.g. a node
///   the guest does not hold, a name that is not plain, a path the guest may
///   not write;
/// - the request is malformed, e.g. unknown open flags, or a buffer larger
///   than the file system accepts;
/// - the file system does not support it (`ENOSYS`, the default bodies).
///
/// Where an operation must fail: its error rules (`FuseState`, e.g.
/// `entry_errors`: each a condition and its code) and `fails_with`: if
/// any rule applies, it fails, with the code of one of the rules that apply,
/// or a host error. So far for the operations that change names (`unlink`,
/// `rmdir`, `rename`, `create`, `mkdir`, `mknod`, `symlink`, `link`), and
/// `open` and `open_dir` (`release` and `release_dir` always succeed), and
/// `lookup`, `get_attr`, `set_attr`, `read_link`, `statfs`, `read`, `write`,
/// `fsync`, `read_dir`, `read_dir_plus`, `fsync_dir`.
///
/// The other reasons are not yet in the contracts, so that they do not yet
/// say when an operation must succeed: a limit reached (the counts and caps
/// are not in the spec), a confinement refusal, and a host failure (the
/// spec has no model of the host). See the virtio-fs spec's open items
/// (`virtiofs-spec-docs/CODE-OVERVIEW.md`).
#[cfg_attr(verus_keep_ghost, verus_verify)]
pub trait Fuse<G: FuseGhost>: Send + Sync + FuseOwns<G>
where
    G::V: FuseState,
{
    /// Looks up a child of an inode.
    ///
    /// This increases the lookup count of the found entry by one.
    #[cfg_attr(verus_keep_ghost, verus_verify(external_body))]
    #[cfg_attr(verus_keep_ghost, verus_spec(r =>
        with Tracked(g): Tracked<&mut G>
        requires
            // The ghost state is well formed, and is this file system's.
            self.owns(old(g)),
            old(g).wf(),
        ensures
            self.owns(final(g)),
            final(g).wf(),
            match r {
                // Succeeds only for a name the guest may reach, and hands out
                // that path's node. Only the path rules: the rule on pinned
                // identities is about which object is at the path, which the
                // spec cannot say without a model of the host (deferred). The
                // kind is the reply's.
                Ok(entry) => {
                    let s = old(g)@;
                    let n = _request@.node_id;
                    let p = s.child_path(n, _name@);
                    let kind = entry.attr.mode & lx::S_IFMT;
                    &&& s.is_entry(n, _name@)
                    &&& s.may_reach(n, p, kind)
                    &&& final(g)@ == s.after_reach(n, p, entry.nodeid, kind)
                },
                // An error changes nothing.
                Err(_) => final(g)@ == old(g)@,
            },
            // Fails where one of its rules applies.
            G::V::fails_with(r, old(g)@.lookup_errors(_request@.node_id, _name@)),
    ))]
    fn lookup(&self, _request: &Request, _name: &LxStr) -> lx::Result<fuse_entry_out> {
        Err(lx::Error::ENOSYS)
    }

    /// Tells the FUSE file system to reduce the lookup count of an inode by the specified amount.
    ///
    /// # Note
    ///
    /// The client is not guaranteed to send a forget message for every inode if the file system
    /// is unmounted.
    #[cfg_attr(verus_keep_ghost, verus_verify(external_body))]
    #[cfg_attr(verus_keep_ghost, verus_spec(
        with Tracked(g): Tracked<&mut G>
        requires
            // The ghost state is well formed, and is this file system's.
            self.owns(old(g)),
            old(g).wf(),
        ensures
            self.owns(final(g)),
            final(g).wf(),
            // The node may be gone (when its count reaches zero; the counts
            // are not in the spec yet); never the root.
            final(g)@ == old(g)@
                || (_node_id != FUSE_ROOT_ID && final(g)@ == old(g)@.after_forget(_node_id)),
    ))]
    fn forget(&self, _node_id: u64, _lookup_count: u64) {
        // No reply is allowed from this message, not even error.
    }

    /// Retrieves the attributes of a file.
    ///
    /// # Note
    ///
    /// If attributes are retrieved through an open file descriptor (i.e. using `fstat`), the
    /// `fh` parameter will be set to the file handle returned by the `open` call.
    #[cfg_attr(verus_keep_ghost, verus_verify(external_body))]
    #[cfg_attr(verus_keep_ghost, verus_spec(r =>
        with Tracked(g): Tracked<&G>
        requires
            self.owns(g),
            g.wf(),
        ensures
            // Succeeds only on the open file `fh` (with `FUSE_GETATTR_FH`), or on a node the guest holds.
            r is Ok ==> if _flags & FUSE_GETATTR_FH != 0 { g@.uses_handle(_request@.node_id, _fh) } else { g@.holds_node(_request@.node_id) },
            // Fails where one of its rules applies.
            G::V::fails_with(r, g@.get_attr_errors(_request@.node_id, _flags, _fh)),
    ))]
    fn get_attr(&self, _request: &Request, _flags: u32, _fh: u64) -> lx::Result<fuse_attr_out> {
        Err(lx::Error::ENOSYS)
    }

    /// Changes the attributes of a file.
    #[cfg_attr(verus_keep_ghost, verus_verify(external_body))]
    #[cfg_attr(verus_keep_ghost, verus_spec(r =>
        with Tracked(g): Tracked<&G>
        requires
            self.owns(g),
            g.wf(),
        ensures
            // Changing attributes (anything beyond the handle and the lock owner) succeeds only on a
            // file the guest may change: the open file `fh`, or the request's node.
            r is Ok ==> (_arg.valid & !(FATTR_FH | FATTR_LOCKOWNER) != 0 ==> if _arg.valid & FATTR_FH != 0 {
                g@.uses_handle(_request@.node_id, _arg.fh) && g@.file_can_write(_arg.fh)
            } else {
                g@.holds_node(_request@.node_id) && g@.node_can_write(_request@.node_id)
            }),
            // Fails where one of its rules applies.
            G::V::fails_with(r, g@.set_attr_errors(_request@.node_id, _arg.valid, _arg.fh)),
    ))]
    fn set_attr(&self, _request: &Request, _arg: &fuse_setattr_in) -> lx::Result<fuse_attr_out> {
        Err(lx::Error::ENOSYS)
    }

    /// Reads the target of a symbolic link.
    #[cfg_attr(verus_keep_ghost, verus_verify(external_body))]
    #[cfg_attr(verus_keep_ghost, verus_spec(r =>
        with Tracked(g): Tracked<&G>
        requires
            self.owns(g),
            g.wf(),
        ensures
            // Succeeds only on a node the guest holds.
            r is Ok ==> g@.holds_node(_request@.node_id),
            // Fails where one of its rules applies.
            G::V::fails_with(r, g@.node_errors(_request@.node_id, false)),
    ))]
    fn read_link(&self, _request: &Request) -> lx::Result<LxString> {
        Err(lx::Error::ENOSYS)
    }

    /// Creates a symbolic link as a child of the specified inode.
    #[cfg_attr(verus_keep_ghost, verus_verify(external_body))]
    #[cfg_attr(verus_keep_ghost, verus_spec(r =>
        with Tracked(g): Tracked<&mut G>
        requires
            // The ghost state is well formed, and is this file system's.
            self.owns(old(g)),
            old(g).wf(),
        ensures
            self.owns(final(g)),
            final(g).wf(),
            match r {
                // Succeeds only where the guest may add a name, and hands out
                // the node of the new symbolic link: a new node, or an existing one of
                // the same identity (a node the guest still holds for a removed
                // object whose inode number the host reused: the file system
                // keys nodes by identity, so `after_reach` covers both).
                Ok(entry) => {
                    let s = old(g)@;
                    let n = _request@.node_id;
                    &&& s.may_add_name(n, _name@)
                    &&& final(g)@ == s.after_reach(n, s.child_path(n, _name@), entry.nodeid, lx::S_IFLNK)
                },
                Err(_) => final(g)@ == old(g)@,
            },
            // Fails where one of its rules applies.
            G::V::fails_with(r, old(g)@.symlink_errors(_request@.node_id, _name@, _target@)),
    ))]
    fn symlink(
        &self,
        _request: &Request,
        _name: &LxStr,
        _target: &LxStr,
    ) -> lx::Result<fuse_entry_out> {
        Err(lx::Error::ENOSYS)
    }

    /// Creates a regular file, fifo, socket, or character or block device node as a child of
    /// the specified inode.
    #[cfg_attr(verus_keep_ghost, verus_verify(external_body))]
    #[cfg_attr(verus_keep_ghost, verus_spec(r =>
        with Tracked(g): Tracked<&mut G>
        requires
            // The ghost state is well formed, and is this file system's.
            self.owns(old(g)),
            old(g).wf(),
        ensures
            self.owns(final(g)),
            final(g).wf(),
            match r {
                // Succeeds only where the guest may add a name, and hands out
                // the node of the new file (of the type the reply reports): a new node, or an existing one of
                // the same identity (a node the guest still holds for a removed
                // object whose inode number the host reused: the file system
                // keys nodes by identity, so `after_reach` covers both).
                Ok(entry) => {
                    let s = old(g)@;
                    let n = _request@.node_id;
                    &&& s.may_add_name(n, _name@)
                    &&& final(g)@ == s.after_reach(n, s.child_path(n, _name@), entry.nodeid, entry.attr.mode & lx::S_IFMT)
                },
                Err(_) => final(g)@ == old(g)@,
            },
            // Fails where one of its rules applies.
            G::V::fails_with(r, old(g)@.entry_errors(_request@.node_id, _name@)),
    ))]
    fn mknod(
        &self,
        _request: &Request,
        _name: &LxStr,
        _arg: &fuse_mknod_in,
    ) -> lx::Result<fuse_entry_out> {
        Err(lx::Error::ENOSYS)
    }

    /// Creates a directory as a child of the specified inode.
    #[cfg_attr(verus_keep_ghost, verus_verify(external_body))]
    #[cfg_attr(verus_keep_ghost, verus_spec(r =>
        with Tracked(g): Tracked<&mut G>
        requires
            // The ghost state is well formed, and is this file system's.
            self.owns(old(g)),
            old(g).wf(),
        ensures
            self.owns(final(g)),
            final(g).wf(),
            match r {
                // Succeeds only where the guest may add a name, and hands out
                // the node of the new directory: a new node, or an existing one of
                // the same identity (a node the guest still holds for a removed
                // object whose inode number the host reused: the file system
                // keys nodes by identity, so `after_reach` covers both).
                Ok(entry) => {
                    let s = old(g)@;
                    let n = _request@.node_id;
                    &&& s.may_add_name(n, _name@)
                    &&& final(g)@ == s.after_reach(n, s.child_path(n, _name@), entry.nodeid, lx::S_IFDIR)
                },
                Err(_) => final(g)@ == old(g)@,
            },
            // Fails where one of its rules applies.
            G::V::fails_with(r, old(g)@.entry_errors(_request@.node_id, _name@)),
    ))]
    fn mkdir(
        &self,
        _request: &Request,
        _name: &LxStr,
        _arg: &fuse_mkdir_in,
    ) -> lx::Result<fuse_entry_out> {
        Err(lx::Error::ENOSYS)
    }

    /// Removes a non-directory child from the specified inode.
    #[cfg_attr(verus_keep_ghost, verus_verify(external_body))]
    #[cfg_attr(verus_keep_ghost, verus_spec(r =>
        with Tracked(g): Tracked<&mut G>
        requires
            // The ghost state is well formed, and is this file system's.
            self.owns(old(g)),
            old(g).wf(),
        ensures
            self.owns(final(g)),
            final(g).wf(),
            match r {
                // Succeeds only where the guest may remove the name: the
                // directory may be changed, and the name's path is writable.
                // Every name at or below that path is gone after.
                Ok(()) => {
                    let s = old(g)@;
                    let n = _request@.node_id;
                    let p = s.child_path(n, _name@);
                    &&& s.is_entry(n, _name@)
                    &&& s.node_can_write(n)
                    &&& s.can_write(n, p)
                    &&& s.unlinked(n, p, final(g)@)
                },
                Err(_) => final(g)@ == old(g)@,
            },
            // Fails where one of its rules applies.
            G::V::fails_with(r, old(g)@.entry_errors(_request@.node_id, _name@)),
    ))]
    fn unlink(&self, _request: &Request, _name: &LxStr) -> lx::Result<()> {
        Err(lx::Error::ENOSYS)
    }

    /// Removes a directory child from the specified inode.
    #[cfg_attr(verus_keep_ghost, verus_verify(external_body))]
    #[cfg_attr(verus_keep_ghost, verus_spec(r =>
        with Tracked(g): Tracked<&mut G>
        requires
            // The ghost state is well formed, and is this file system's.
            self.owns(old(g)),
            old(g).wf(),
        ensures
            self.owns(final(g)),
            final(g).wf(),
            match r {
                // Succeeds only where the guest may remove the name: the
                // directory may be changed, and the name's path is writable.
                // Every name at or below that path is gone after.
                Ok(()) => {
                    let s = old(g)@;
                    let n = _request@.node_id;
                    let p = s.child_path(n, _name@);
                    &&& s.is_entry(n, _name@)
                    &&& s.node_can_write(n)
                    &&& s.can_write(n, p)
                    &&& s.unlinked(n, p, final(g)@)
                },
                Err(_) => final(g)@ == old(g)@,
            },
            // Fails where one of its rules applies.
            G::V::fails_with(r, old(g)@.entry_errors(_request@.node_id, _name@)),
    ))]
    fn rmdir(&self, _request: &Request, _name: &LxStr) -> lx::Result<()> {
        Err(lx::Error::ENOSYS)
    }

    /// Renames a file.
    ///
    /// The file's original parent is the request's inode, while the new parent is indicated using
    /// `new_dir`.
    #[cfg_attr(verus_keep_ghost, verus_verify(external_body))]
    #[cfg_attr(verus_keep_ghost, verus_spec(r =>
        with Tracked(g): Tracked<&mut G>
        requires
            // The ghost state is well formed, and is this file system's.
            self.owns(old(g)),
            old(g).wf(),
        ensures
            self.owns(final(g)),
            final(g).wf(),
            match r {
                // Succeeds only within one share, where the guest may change
                // both directories and write both paths. After, either the
                // two names are swapped (`RENAME_EXCHANGE`), or the new name's
                // old entries are gone and the old name's are at the new one.
                Ok(()) => {
                    let s = old(g)@;
                    let n = _request@.node_id;
                    let p = s.child_path(n, _name@);
                    let q = s.child_path(_new_dir, _new_name@);
                    &&& s.is_entry(n, _name@)
                    &&& s.is_entry(_new_dir, _new_name@)
                    &&& s.same_share(n, _new_dir)
                    &&& s.node_can_write(n)
                    &&& s.node_can_write(_new_dir)
                    &&& s.can_write(n, p)
                    &&& s.can_write(_new_dir, q)
                    &&& s.renamed(n, p, q, G::V::is_exchange(_flags), final(g)@)
                },
                Err(_) => final(g)@ == old(g)@,
            },
            // Fails where one of its rules applies.
            G::V::fails_with(r, old(g)@.rename_errors(_request@.node_id, _name@, _new_dir, _new_name@)),
    ))]
    fn rename(
        &self,
        _request: &Request,
        _name: &LxStr,
        _new_dir: u64,
        _new_name: &LxStr,
        _flags: u32,
    ) -> lx::Result<()> {
        Err(lx::Error::ENOSYS)
    }

    /// Creates a hard-link to an existing inode, as a child of the specified inode..
    #[cfg_attr(verus_keep_ghost, verus_verify(external_body))]
    #[cfg_attr(verus_keep_ghost, verus_spec(r =>
        with Tracked(g): Tracked<&mut G>
        requires
            // The ghost state is well formed, and is this file system's.
            self.owns(old(g)),
            old(g).wf(),
        ensures
            self.owns(final(g)),
            final(g).wf(),
            match r {
                // Succeeds only where the guest may add a name, for a target
                // node in the same share that the guest may change (a new name
                // must not make a read-only file writable); the new name is
                // the target's, and the reply is the target's node.
                Ok(entry) => {
                    let s = old(g)@;
                    let n = _request@.node_id;
                    &&& s.may_add_name(n, _name@)
                    &&& s.holds_node(_target)
                    &&& s.same_share(n, _target)
                    &&& s.node_can_write(_target)
                    &&& entry.nodeid == _target
                    &&& final(g)@ == s.after_link(_target, s.child_path(n, _name@))
                },
                Err(_) => final(g)@ == old(g)@,
            },
            // Fails where one of its rules applies.
            G::V::fails_with(r, old(g)@.link_errors(_request@.node_id, _name@, _target)),
    ))]
    fn link(&self, _request: &Request, _name: &LxStr, _target: u64) -> lx::Result<fuse_entry_out> {
        Err(lx::Error::ENOSYS)
    }

    /// Opens a file.
    ///
    /// Flags are Linux `O_*` values from [`lx`], using the platform's Linux ABI,
    /// not FUSE `FOPEN_*` reply flags.
    ///
    /// If not implemented, this call will succeed, which can be used if the file system doesn't
    /// need any state for open files, since the inode number is also provided to functions
    /// such as `read` and `write`.
    #[cfg_attr(verus_keep_ghost, verus_verify(external_body))]
    #[cfg_attr(verus_keep_ghost, verus_spec(r =>
        with Tracked(g): Tracked<&mut G>
        requires
            // The ghost state is well formed, and is this file system's.
            self.owns(old(g)),
            old(g).wf(),
        ensures
            self.owns(final(g)),
            final(g).wf(),
            match r {
                // Succeeds only on a node the guest holds, and, to open a file for
                // writing or to truncate it, only if the guest may change it;
                // hands out a new handle on it.
                Ok(out) => {
                    let s = old(g)@;
                    let n = _request@.node_id;
                    &&& s.holds_node(n)
                    &&& (G::V::opens_for_writing(_flags) || G::V::truncates(_flags)) ==> s.node_can_write(n)
                    &&& !s.holds_file(out.fh)
                    &&& final(g)@ == s.after_open(out.fh, n, G::V::opens_for_writing(_flags))
                },
                Err(_) => final(g)@ == old(g)@,
            },
            // Fails where one of its rules applies.
            G::V::fails_with(r, old(g)@.open_errors(_request@.node_id, _flags)),
    ))]
    fn open(&self, _request: &Request, _flags: u32) -> lx::Result<fuse_open_out> {
        Err(lx::Error::ENOSYS)
    }

    /// Reads data from an open file.
    #[cfg_attr(verus_keep_ghost, verus_verify(external_body))]
    #[cfg_attr(verus_keep_ghost, verus_spec(r =>
        with Tracked(g): Tracked<&G>
        requires
            self.owns(g),
            g.wf(),
        ensures
            // Succeeds only on a file the guest holds open.
            r is Ok ==> g@.uses_handle(_request@.node_id, _arg.fh),
            // Fails where one of its rules applies.
            G::V::fails_with(r, g@.read_errors(_request@.node_id, _arg.fh, _arg.size as nat)),
    ))]
    fn read(&self, _request: &Request, _arg: &fuse_read_in) -> lx::Result<Vec<u8>> {
        Err(lx::Error::ENOSYS)
    }

    /// Writes data to an open file.
    #[cfg_attr(verus_keep_ghost, verus_verify(external_body))]
    #[cfg_attr(verus_keep_ghost, verus_spec(r =>
        with Tracked(g): Tracked<&G>
        requires
            self.owns(g),
            g.wf(),
        ensures
            // Succeeds only on a file the guest opened for writing, and may
            // change.
            r is Ok ==> g@.uses_handle(_request@.node_id, _arg.fh) && g@.opened_for_writing(_arg.fh) && g@.file_can_write(_arg.fh),
            // Fails where one of its rules applies.
            G::V::fails_with(r, g@.write_errors(_request@.node_id, _arg.fh, _data@.len())),
    ))]
    fn write(&self, _request: &Request, _arg: &fuse_write_in, _data: &[u8]) -> lx::Result<usize> {
        Err(lx::Error::ENOSYS)
    }

    /// Retrieves the attributes of the file system.
    #[cfg_attr(verus_keep_ghost, verus_verify(external_body))]
    #[cfg_attr(verus_keep_ghost, verus_spec(r =>
        with Tracked(g): Tracked<&G>
        requires
            self.owns(g),
            g.wf(),
        ensures
            // Succeeds only on a node the guest holds.
            r is Ok ==> g@.holds_node(_request@.node_id),
            // Fails where one of its rules applies.
            G::V::fails_with(r, g@.node_errors(_request@.node_id, true)),
    ))]
    fn statfs(&self, _request: &Request) -> lx::Result<fuse_kstatfs> {
        Err(lx::Error::ENOSYS)
    }

    /// Closes an open file.
    ///
    /// If not implemented, this call will succeed. Won't be called if `open`
    /// returned `ENOSYS` (the default).
    #[cfg_attr(verus_keep_ghost, verus_verify(external_body))]
    #[cfg_attr(verus_keep_ghost, verus_spec(r =>
        with Tracked(g): Tracked<&mut G>
        requires
            // The ghost state is well formed, and is this file system's.
            self.owns(old(g)),
            old(g).wf(),
        ensures
            self.owns(final(g)),
            final(g).wf(),
            // The handle is gone (if the guest held it).
            final(g)@ == old(g)@.after_release(_arg.fh),
            // Always succeeds.
            r is Ok,
    ))]
    fn release(&self, _request: &Request, _arg: &fuse_release_in) -> lx::Result<()> {
        Ok(())
    }

    /// Synchronize file contents.
    #[cfg_attr(verus_keep_ghost, verus_verify(external_body))]
    #[cfg_attr(verus_keep_ghost, verus_spec(r =>
        with Tracked(g): Tracked<&G>
        requires
            self.owns(g),
            g.wf(),
        ensures
            // Succeeds only on a file the guest holds open.
            r is Ok ==> g@.uses_handle(_request@.node_id, _fh),
            // Fails where one of its rules applies.
            G::V::fails_with(r, g@.handle_errors(_request@.node_id, _fh)),
    ))]
    fn fsync(&self, _request: &Request, _fh: u64, _flags: u32) -> lx::Result<()> {
        Err(lx::Error::ENOSYS)
    }

    /// Add or change an extended attribute on an inode.
    #[cfg_attr(verus_keep_ghost, verus_verify(external_body))]
    #[cfg_attr(verus_keep_ghost, verus_spec(r =>
        with Tracked(g): Tracked<&G>
        requires
            self.owns(g),
            g.wf(),
        ensures
            // Succeeds only on a node the guest may change.
            r is Ok ==> g@.holds_node(_request@.node_id) && g@.node_can_write(_request@.node_id),
    ))]
    fn set_xattr(
        &self,
        _request: &Request,
        _name: &LxStr,
        _value: &[u8],
        _flags: u32,
    ) -> lx::Result<()> {
        Err(lx::Error::ENOSYS)
    }

    /// Retrieve an extended attribute on an inode.
    #[cfg_attr(verus_keep_ghost, verus_verify(external_body))]
    #[cfg_attr(verus_keep_ghost, verus_spec(r =>
        with Tracked(g): Tracked<&G>
        requires
            self.owns(g),
            g.wf(),
        ensures
            // Succeeds only on a node the guest holds.
            r is Ok ==> g@.holds_node(_request@.node_id),
    ))]
    fn get_xattr(&self, _request: &Request, _name: &LxStr, _size: u32) -> lx::Result<Vec<u8>> {
        Err(lx::Error::ENOSYS)
    }

    /// Retrieve the size of an extended attribute on an inode.
    #[cfg_attr(verus_keep_ghost, verus_verify(external_body))]
    #[cfg_attr(verus_keep_ghost, verus_spec(r =>
        with Tracked(g): Tracked<&G>
        requires
            self.owns(g),
            g.wf(),
        ensures
            // Succeeds only on a node the guest holds.
            r is Ok ==> g@.holds_node(_request@.node_id),
    ))]
    fn get_xattr_size(&self, _request: &Request, _name: &LxStr) -> lx::Result<u32> {
        Err(lx::Error::ENOSYS)
    }

    /// List all extended attributes on an inode.
    #[cfg_attr(verus_keep_ghost, verus_verify(external_body))]
    #[cfg_attr(verus_keep_ghost, verus_spec(r =>
        with Tracked(g): Tracked<&G>
        requires
            self.owns(g),
            g.wf(),
        ensures
            // Succeeds only on a node the guest holds.
            r is Ok ==> g@.holds_node(_request@.node_id),
    ))]
    fn list_xattr(&self, _request: &Request, _size: u32) -> lx::Result<Vec<u8>> {
        Err(lx::Error::ENOSYS)
    }

    /// Retrieve the size of the list of extended attributes on an inode.
    #[cfg_attr(verus_keep_ghost, verus_verify(external_body))]
    #[cfg_attr(verus_keep_ghost, verus_spec(r =>
        with Tracked(g): Tracked<&G>
        requires
            self.owns(g),
            g.wf(),
        ensures
            // Succeeds only on a node the guest holds.
            r is Ok ==> g@.holds_node(_request@.node_id),
    ))]
    fn list_xattr_size(&self, _request: &Request) -> lx::Result<u32> {
        Err(lx::Error::ENOSYS)
    }

    /// Remove an extended attribute from an inode.
    #[cfg_attr(verus_keep_ghost, verus_verify(external_body))]
    #[cfg_attr(verus_keep_ghost, verus_spec(r =>
        with Tracked(g): Tracked<&G>
        requires
            self.owns(g),
            g.wf(),
        ensures
            // Succeeds only on a node the guest may change.
            r is Ok ==> g@.holds_node(_request@.node_id) && g@.node_can_write(_request@.node_id),
    ))]
    fn remove_xattr(&self, _request: &Request, _name: &LxStr) -> lx::Result<()> {
        Err(lx::Error::ENOSYS)
    }

    /// Called on each `close()` of a file descriptor for an opened file.
    ///
    /// This is called for every file descriptor, so may be called more than once.
    ///
    /// Use `release` to know when the last file descriptor was closed.
    #[cfg_attr(verus_keep_ghost, verus_verify(external_body))]
    #[cfg_attr(verus_keep_ghost, verus_spec(r =>
        with Tracked(g): Tracked<&G>
        requires
            self.owns(g),
            g.wf(),
        ensures
            // Succeeds only on a file the guest holds open.
            r is Ok ==> g@.uses_handle(_request@.node_id, _arg.fh),
    ))]
    fn flush(&self, _request: &Request, _arg: &fuse_flush_in) -> lx::Result<()> {
        Err(lx::Error::ENOSYS)
    }

    /// Negotiate file system parameters with the client.
    #[cfg_attr(verus_keep_ghost, verus_verify(external_body))]
    fn init(&self, _info: &mut SessionInfo) {}

    /// Opens a directory.
    ///
    /// If not implemented, this call will succeed, which can be used if the file system doesn't
    /// need any state for open files, since the inode number is also provided to functions
    /// such as `read_dir`.
    #[cfg_attr(verus_keep_ghost, verus_verify(external_body))]
    #[cfg_attr(verus_keep_ghost, verus_spec(r =>
        with Tracked(g): Tracked<&mut G>
        requires
            // The ghost state is well formed, and is this file system's.
            self.owns(old(g)),
            old(g).wf(),
        ensures
            self.owns(final(g)),
            final(g).wf(),
            match r {
                // Succeeds only on a node the guest holds, and, to open a directory for
                // writing or to truncate it, only if the guest may change it;
                // hands out a new handle on it.
                Ok(out) => {
                    let s = old(g)@;
                    let n = _request@.node_id;
                    &&& s.holds_node(n)
                    &&& (G::V::opens_for_writing(_flags) || G::V::truncates(_flags)) ==> s.node_can_write(n)
                    &&& !s.holds_file(out.fh)
                    &&& final(g)@ == s.after_open(out.fh, n, G::V::opens_for_writing(_flags))
                },
                Err(_) => final(g)@ == old(g)@,
            },
            // Fails where one of its rules applies.
            G::V::fails_with(r, old(g)@.open_dir_errors(_request@.node_id, _flags)),
    ))]
    fn open_dir(&self, _request: &Request, _flags: u32) -> lx::Result<fuse_open_out> {
        Err(lx::Error::ENOSYS)
    }

    /// Reads the contents of a directory.
    ///
    /// Use `DirEntryWriter` to create a buffer containing directory entries.
    #[cfg_attr(verus_keep_ghost, verus_verify(external_body))]
    #[cfg_attr(verus_keep_ghost, verus_spec(r =>
        with Tracked(g): Tracked<&G>
        requires
            self.owns(g),
            g.wf(),
        ensures
            // Succeeds only on a directory the guest holds open.
            r is Ok ==> g@.uses_handle(_request@.node_id, _arg.fh),
            // Fails where one of its rules applies.
            G::V::fails_with(r, g@.read_dir_errors(_request@.node_id, _arg.fh, _arg.size as nat)),
    ))]
    fn read_dir(&self, _request: &Request, _arg: &fuse_read_in) -> lx::Result<Vec<u8>> {
        Err(lx::Error::ENOSYS)
    }

    /// Closes a directory.
    ///
    /// If not implemented, this call will succeed. Won't be called if `opendir`
    /// returned ENOSYS (the default).
    #[cfg_attr(verus_keep_ghost, verus_verify(external_body))]
    #[cfg_attr(verus_keep_ghost, verus_spec(r =>
        with Tracked(g): Tracked<&mut G>
        requires
            // The ghost state is well formed, and is this file system's.
            self.owns(old(g)),
            old(g).wf(),
        ensures
            self.owns(final(g)),
            final(g).wf(),
            // The handle is gone (if the guest held it).
            final(g)@ == old(g)@.after_release(_arg.fh),
            // Always succeeds.
            r is Ok,
    ))]
    fn release_dir(&self, _request: &Request, _arg: &fuse_release_in) -> lx::Result<()> {
        Ok(())
    }

    /// Synchronize directory contents.
    #[cfg_attr(verus_keep_ghost, verus_verify(external_body))]
    #[cfg_attr(verus_keep_ghost, verus_spec(r =>
        with Tracked(g): Tracked<&G>
        requires
            self.owns(g),
            g.wf(),
        ensures
            // Succeeds only on a directory the guest holds open.
            r is Ok ==> g@.uses_handle(_request@.node_id, _fh),
            // Fails where one of its rules applies.
            G::V::fails_with(r, g@.handle_errors(_request@.node_id, _fh)),
    ))]
    fn fsync_dir(&self, _request: &Request, _fh: u64, _flags: u32) -> lx::Result<()> {
        Err(lx::Error::ENOSYS)
    }

    /// Test for a POSIX file lock.
    #[cfg_attr(verus_keep_ghost, verus_verify(external_body))]
    #[cfg_attr(verus_keep_ghost, verus_spec(r =>
        with Tracked(g): Tracked<&G>
        requires
            self.owns(g),
            g.wf(),
        ensures
            // Succeeds only on a file the guest holds open.
            r is Ok ==> g@.uses_handle(_request@.node_id, _arg.fh),
    ))]
    fn get_lock(&self, _request: &Request, _arg: &fuse_lk_in) -> lx::Result<fuse_file_lock> {
        Err(lx::Error::ENOSYS)
    }

    /// Acquire, modify or release a POSIX file lock.
    ///
    /// If not implemented, the client still allows for local file locking.
    #[cfg_attr(verus_keep_ghost, verus_verify(external_body))]
    #[cfg_attr(verus_keep_ghost, verus_spec(r =>
        with Tracked(g): Tracked<&G>
        requires
            self.owns(g),
            g.wf(),
        ensures
            // Succeeds only on a file the guest holds open (a lock changes no contents).
            r is Ok ==> g@.uses_handle(_request@.node_id, _arg.fh),
    ))]
    fn set_lock(&self, _request: &Request, _arg: &fuse_lk_in, _sleep: bool) -> lx::Result<()> {
        Err(lx::Error::ENOSYS)
    }

    /// Check file access permissions.
    #[cfg_attr(verus_keep_ghost, verus_verify(external_body))]
    #[cfg_attr(verus_keep_ghost, verus_spec(r =>
        with Tracked(g): Tracked<&G>
        requires
            self.owns(g),
            g.wf(),
        ensures
            // Succeeds only on a node the guest holds.
            r is Ok ==> g@.holds_node(_request@.node_id),
    ))]
    fn access(&self, _request: &Request, _mask: u32) -> lx::Result<()> {
        Err(lx::Error::ENOSYS)
    }

    /// Create and open a file.
    ///
    /// The request retains its wire representation, including Linux `O_*` flags
    /// from [`lx`], using the platform's Linux ABI.
    ///
    /// If not implemented, the client will use `mknod` followed by `open`.
    #[cfg_attr(verus_keep_ghost, verus_verify(external_body))]
    #[cfg_attr(verus_keep_ghost, verus_spec(r =>
        with Tracked(g): Tracked<&mut G>
        requires
            // The ghost state is well formed, and is this file system's.
            self.owns(old(g)),
            old(g).wf(),
        ensures
            self.owns(final(g)),
            final(g).wf(),
            match r {
                // Succeeds only where the guest may add a name; hands out the
                // file's node (an existing one, if the file was there and the
                // create was not exclusive; its kind is the reply's, since that
                // file may be a FIFO, say) and a new handle on it.
                Ok(out) => {
                    let s = old(g)@;
                    let n = _request@.node_id;
                    let m = out.entry.nodeid;
                    &&& s.may_add_name(n, _name@)
                    &&& !s.holds_file(out.open.fh)
                    &&& final(g)@ == s.after_reach(n, s.child_path(n, _name@), m, out.entry.attr.mode & lx::S_IFMT)
                            .after_open(out.open.fh, m, G::V::opens_for_writing(_arg.flags))
                },
                Err(_) => final(g)@ == old(g)@,
            },
            // Fails where one of its rules applies.
            G::V::fails_with(r, old(g)@.create_errors(_request@.node_id, _name@, _arg.flags)),
    ))]
    fn create(
        &self,
        _request: &Request,
        _name: &LxStr,
        _arg: &fuse_create_in,
    ) -> lx::Result<CreateOut> {
        Err(lx::Error::ENOSYS)
    }

    /// Map a file block index to a device block index.
    ///
    /// This method is only relevant for file systems mounted using `fuseblk`.
    #[cfg_attr(verus_keep_ghost, verus_verify(external_body))]
    #[cfg_attr(verus_keep_ghost, verus_spec(r =>
        with Tracked(g): Tracked<&G>
        requires
            self.owns(g),
            g.wf(),
        ensures
            // Succeeds only on a node the guest holds.
            r is Ok ==> g@.holds_node(_request@.node_id),
    ))]
    fn block_map(&self, _request: &Request, _block: u64, _block_size: u32) -> lx::Result<u64> {
        Err(lx::Error::ENOSYS)
    }

    /// Clean up the file system.
    ///
    /// For regular FUSE, the client only calls this for file systems mounted using `fuseblk`, but
    /// for other file systems `Connection` will call it when the `/dev/fuse` connection is closed.
    ///
    /// For virtio-fs, the client will call this when the file system is unmounted. After receiving
    /// destroy, another `init` call can be received if the file system is mounted again.
    #[cfg_attr(verus_keep_ghost, verus_verify(external_body))]
    #[cfg_attr(verus_keep_ghost, verus_spec(
        with Tracked(g): Tracked<&mut G>
        requires
            // The ghost state is well formed, and is this file system's.
            self.owns(old(g)),
            old(g).wf(),
        ensures
            self.owns(final(g)),
            final(g).wf(),
            // Every handle and every node but the root is gone (the guest
            // unmounted; a new mount starts from the root).
            final(g)@ == old(g)@.after_destroy(),
    ))]
    fn destroy(&self) {}

    /// Submit an ioctl.
    ///
    /// # Note
    ///
    /// This is a somewhat limited subset of the ioctl functionality of libfuse; the additional
    /// functionality seems to only apply to CUSE, however.
    #[cfg_attr(verus_keep_ghost, verus_verify(external_body))]
    #[cfg_attr(verus_keep_ghost, verus_spec(r =>
        with Tracked(g): Tracked<&G>
        requires
            self.owns(g),
            g.wf(),
        ensures
            // Succeeds only on an open file the guest may change: an ioctl can
            // change a file (its flags, say), and the spec does not tell them
            // apart.
            r is Ok ==> g@.uses_handle(_request@.node_id, _arg.fh) && g@.file_can_write(_arg.fh),
    ))]
    fn ioctl(
        &self,
        _request: &Request,
        _arg: &fuse_ioctl_in,
        _data: &[u8],
    ) -> lx::Result<(i32, Vec<u8>)> {
        Err(lx::Error::ENOSYS)
    }

    /// Allocate requested space.
    #[cfg_attr(verus_keep_ghost, verus_verify(external_body))]
    #[cfg_attr(verus_keep_ghost, verus_spec(r =>
        with Tracked(g): Tracked<&G>
        requires
            self.owns(g),
            g.wf(),
        ensures
            // Succeeds only on a file the guest opened for writing, and may
            // change.
            r is Ok ==> g@.uses_handle(_request@.node_id, _arg.fh) && g@.opened_for_writing(_arg.fh) && g@.file_can_write(_arg.fh),
    ))]
    fn fallocate(&self, _request: &Request, _arg: &fuse_fallocate_in) -> lx::Result<()> {
        Err(lx::Error::ENOSYS)
    }

    /// Reads the contents of a directory, and performs a lookup on each entry.
    ///
    /// This function increases the lookup count of each entry in the directory by one.
    ///
    /// If you implement this, you must set `FUSE_DO_READDIRPLUS` in `init`. If you implement both
    /// read_dir_plus and read_dir, also set `FUSE_READDIRPLUS_AUTO`.
    #[cfg_attr(verus_keep_ghost, verus_verify(external_body))]
    #[cfg_attr(verus_keep_ghost, verus_spec(r =>
        with Tracked(g): Tracked<&mut G>
        requires
            // The ghost state is well formed, and is this file system's.
            self.owns(old(g)),
            old(g).wf(),
        ensures
            self.owns(final(g)),
            final(g).wf(),
            // Succeeds only on a directory the guest holds open.
            r is Ok ==> old(g)@.uses_handle(_request@.node_id, _arg.fh),
            // Each entry listed with its node is looked up, as by `lookup`,
            // in the open directory: the state after is the result of such
            // lookups, each allowed by `lookup`'s rules (also on an error,
            // which may come after some of them).
            old(g)@.looked_up_in(old(g)@.file_node(_arg.fh), final(g)@),
            // Fails where one of its rules applies.
            G::V::fails_with(r, old(g)@.read_dir_errors(_request@.node_id, _arg.fh, _arg.size as nat)),
    ))]
    fn read_dir_plus(&self, _request: &Request, _arg: &fuse_read_in) -> lx::Result<Vec<u8>> {
        Err(lx::Error::ENOSYS)
    }

    /// Find data holes in a sparse file.
    #[cfg_attr(verus_keep_ghost, verus_verify(external_body))]
    #[cfg_attr(verus_keep_ghost, verus_spec(r =>
        with Tracked(g): Tracked<&G>
        requires
            self.owns(g),
            g.wf(),
        ensures
            // Succeeds only on a file the guest holds open.
            r is Ok ==> g@.uses_handle(_request@.node_id, _fh),
    ))]
    fn lseek(&self, _request: &Request, _fh: u64, _offset: u64, _whence: u32) -> lx::Result<u64> {
        Err(lx::Error::ENOSYS)
    }

    /// Copy data from one file to another without needing to send data through the FUSE kernel
    /// module.
    #[cfg_attr(verus_keep_ghost, verus_verify(external_body))]
    #[cfg_attr(verus_keep_ghost, verus_spec(r =>
        with Tracked(g): Tracked<&G>
        requires
            self.owns(g),
            g.wf(),
        ensures
            // Succeeds only into a file the guest opened for writing, and may
            // change (the destination, `fh_out`).
            r is Ok ==> g@.uses_handle(_request@.node_id, _arg.fh_in)
                && g@.uses_handle(_arg.nodeid_out, _arg.fh_out)
                && g@.opened_for_writing(_arg.fh_out) && g@.file_can_write(_arg.fh_out),
    ))]
    fn copy_file_range(
        &self,
        _request: &Request,
        _arg: &fuse_copy_file_range_in,
    ) -> lx::Result<usize> {
        Err(lx::Error::ENOSYS)
    }

    /// Create a DAX memory mapping.
    #[cfg_attr(verus_keep_ghost, verus_verify(external_body))]
    #[cfg_attr(verus_keep_ghost, verus_spec(r =>
        with Tracked(g): Tracked<&G>
        requires
            self.owns(g),
            g.wf(),
        ensures
            // Succeeds only on a file the guest holds open; a writable mapping
            // (the guest then changes the file through its memory) only on one
            // opened for writing, that the guest may change.
            r is Ok ==> g@.uses_handle(request@.node_id, arg.fh)
                && (arg.flags & FUSE_SETUPMAPPING_FLAG_WRITE != 0
                    ==> g@.opened_for_writing(arg.fh) && g@.file_can_write(arg.fh)),
    ))]
    fn setup_mapping(
        &self,
        request: &Request,
        mapper: &dyn Mapper,
        arg: &fuse_setupmapping_in,
    ) -> lx::Result<()> {
        let _ = (request, mapper, arg);
        Err(lx::Error::ENOSYS)
    }

    /// Remove a DAX memory mapping.
    #[cfg_attr(verus_keep_ghost, verus_verify(external_body))]
    fn remove_mapping(
        &self,
        request: &Request,
        mapper: &dyn Mapper,
        moffset: u64,
        len: u64,
    ) -> lx::Result<()> {
        let _ = (request, mapper, moffset, len);
        Err(lx::Error::ENOSYS)
    }

    /// Retrieves the statx details of a file.
    ///
    /// # Note
    ///
    /// If information is retrieved through an open file descriptor (i.e. using `fstat`), the
    /// `fh` parameter will be set to the file handle returned by the `open` call.
    #[cfg_attr(verus_keep_ghost, verus_verify(external_body))]
    #[cfg_attr(verus_keep_ghost, verus_spec(r =>
        with Tracked(g): Tracked<&G>
        requires
            self.owns(g),
            g.wf(),
        ensures
            // Succeeds only on the open file `fh` (with `FUSE_GETATTR_FH`), or on a node the guest holds.
            r is Ok ==> if _getattr_flags & FUSE_GETATTR_FH != 0 { g@.uses_handle(_request@.node_id, _fh) } else { g@.holds_node(_request@.node_id) },
    ))]
    fn get_statx(
        &self,
        _request: &Request,
        _fh: u64,
        _getattr_flags: u32,
        _flags: StatxFlags,
        _mask: lx::StatExMask,
    ) -> lx::Result<fuse_statx_out> {
        Err(lx::Error::ENOSYS)
    }
}

#[cfg(windows)]
pub type FileRef<'a> = std::os::windows::io::BorrowedHandle<'a>;
#[cfg(unix)]
pub type FileRef<'a> = std::os::unix::io::BorrowedFd<'a>;

/// Trait for mapping files into a shared memory region.
///
/// This is used to support DAX with virtio-fs.
#[cfg_attr(verus_keep_ghost, verus_verify)]
pub trait Mapper {
    /// Map memory into the region at `offset`.
    fn map(
        &self,
        offset: u64,
        file: FileRef<'_>,
        file_offset: u64,
        len: u64,
        writable: bool,
    ) -> lx::Result<()>;

    /// Unmaps any memory in the given range.
    fn unmap(&self, offset: u64, len: u64) -> lx::Result<()>;

    /// Clears any mappings in the range.
    fn clear(&self);
}

impl fuse_entry_out {
    /// Create a new `fuse_entry_out`.
    pub fn new(node_id: u64, entry_valid: Duration, attr_valid: Duration, attr: fuse_attr) -> Self {
        Self {
            nodeid: node_id,
            generation: 0,
            entry_valid: entry_valid.as_secs(),
            entry_valid_nsec: entry_valid.subsec_nanos(),
            attr_valid: attr_valid.as_secs(),
            attr_valid_nsec: attr_valid.subsec_nanos(),
            attr,
        }
    }

    pub fn new_dot(ino: u64, mode: u32) -> Self {
        let mut entry = Self::new_zeroed();
        entry.attr.ino = ino;
        entry.attr.mode = mode;
        entry
    }
}

impl fuse_attr_out {
    /// Create a new `fuse_attr_out`.
    pub fn new(valid: Duration, attr: fuse_attr) -> Self {
        Self {
            attr_valid: valid.as_secs(),
            attr_valid_nsec: valid.subsec_nanos(),
            dummy: 0,
            attr,
        }
    }
}

impl fuse_statx_out {
    /// Create a new `fuse_statx_out`.
    pub fn new(valid: Duration, flags: StatxFlags, statx: fuse_statx) -> Self {
        Self {
            attr_valid: valid.as_secs(),
            attr_valid_nsec: valid.subsec_nanos(),
            flags,
            statx,
            _rsvd: [0; 2],
        }
    }
}

impl fuse_open_out {
    /// Create a new `fuse_open_out`.
    pub fn new(fh: u64, open_flags: u32) -> Self {
        Self {
            fh,
            open_flags,
            padding: 0,
        }
    }
}

impl fuse_kstatfs {
    /// Create a new `fuse_kstatfs`.
    pub fn new(
        blocks: u64,
        bfree: u64,
        bavail: u64,
        files: u64,
        ffree: u64,
        bsize: u32,
        namelen: u32,
        frsize: u32,
    ) -> Self {
        Self {
            blocks,
            bfree,
            bavail,
            files,
            ffree,
            bsize,
            namelen,
            frsize,
            padding: 0,
            spare: Default::default(),
        }
    }
}
