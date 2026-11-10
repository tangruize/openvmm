// High-level spec of a FUSE filesystem, as the guest sees it, for human
// review. DRAFT: not yet included from `lib.rs`, not yet checked by Verus.
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
use vstd::std_specs::path::PathView;

verus! {

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

    /// The guest may reach the host object `object` (`(device_nr,
    /// inode_nr)`, as `lstat` reports it), of file type `kind`, at path `p`
    /// of `parent`'s share.
    spec fn may_reach(&self, parent: u64, p: PathView, object: (u64, u64), kind: lx::mode_t) -> bool;

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

} // verus!

// The contracts, on the `Fuse` trait (`lib.rs`), shown here as they would be
// written there:
//
// pub trait Fuse<G: View>: Send + Sync where G::V: FuseState { ... }
//
// `g` is the ghost state that every handler is lent; `n` is
// `request.node_id()`; `name@` is the name's bytes.
//
// fn lookup(&self, request: &Request, name: &LxStr) -> (r: lx::Result<fuse_entry_out>)
//     with Tracked(g): Tracked<&mut G>
//     ensures
//         match r {
//             // Succeeds only for a name the guest may reach, and hands out
//             // that object's node. The object is only "some object": the
//             // spec has no model of the host yet, so the rule on pinned
//             // identities (which object is at `p`) is not enforced by this
//             // contract; the path rules are. The kind is the reply's.
//             Ok(entry) => exists|object: (u64, u64)| {
//                 let s = old(g)@;
//                 let p = s.child_path(n, name@);
//                 let kind = entry.attr.mode & lx::S_IFMT;
//                 &&& s.is_entry(n, name@)
//                 &&& s.may_reach(n, p, object, kind)
//                 &&& final(g)@ == s.after_reach(n, p, entry.nodeid, kind)
//             },
//             // An error changes nothing.
//             Err(_) => final(g)@ == old(g)@,
//         };
//
// fn mkdir(&self, request: &Request, name: &LxStr, arg: &fuse_mkdir_in) -> (r: lx::Result<fuse_entry_out>)
//     with Tracked(g): Tracked<&mut G>
//     ensures
//         match r {
//             // Succeeds only where the guest may add a name, and hands out
//             // a new node for the new directory.
//             Ok(entry) => {
//                 let s = old(g)@;
//                 &&& s.may_add_name(n, name@)
//                 &&& !s.holds_node(entry.nodeid)
//                 &&& final(g)@ == s.after_reach(n, s.child_path(n, name@), entry.nodeid, lx::S_IFDIR)
//             },
//             Err(_) => final(g)@ == old(g)@,
//         };
//
// fn create(&self, request: &Request, name: &LxStr, arg: &fuse_create_in) -> (r: lx::Result<CreateOut>)
//     with Tracked(g): Tracked<&mut G>
//     ensures
//         match r {
//             // Succeeds only where the guest may add a name; hands out the
//             // file's node (an existing one, if the file was there and the
//             // create was not exclusive) and a new handle on it.
//             Ok(out) => {
//                 let s = old(g)@;
//                 let m = out.entry.nodeid;
//                 &&& s.may_add_name(n, name@)
//                 &&& !s.holds_file(out.open.fh)
//                 &&& final(g)@ == s.after_reach(n, s.child_path(n, name@), m, lx::S_IFREG)
//                         .after_open(out.open.fh, m, opens_for_writing(arg.flags))
//             },
//             Err(_) => final(g)@ == old(g)@,
//         };
