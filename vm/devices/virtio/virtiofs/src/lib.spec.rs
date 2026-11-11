// High-level spec of virtio-fs's state, as the guest sees it, for human
// review.
//
// The filesystem's ghost state (`VirtioFsGhost`, lent to each request), its
// view (`VirtioFsGhostView`), and virtio-fs's answers to the `fuse` crate's
// questions (`fuse::FuseState`), from virtio-fs's own types: an inode's view,
// and its volume's access policy (`microvm/policy.spec.rs`). How the ghost
// state is tied to the locks: `virtiofs-spec-docs/CODE-OVERVIEW.md`, "The
// guest's state in the spec".

use crate::microvm::policy::spec::SubtreePolicyView;
use fuse::spec::FuseGhost;
use fuse::spec::FuseOwns;
use fuse::spec::FuseState;
use vstd::prelude::*;
use vstd::resource::Loc;
use vstd::resource::frac::FracGhost;
use vstd::resource::map::GhostMapAuth;
use vstd::rwlock::RwLockPredicate;
#[cfg(verus_keep_ghost)]
use std::sync::Arc;
use super::FileTableLocked;
use super::InodeTableLocked;
#[cfg(verus_keep_ghost)]
use crate::file::VirtioFsFile;
#[cfg(verus_keep_ghost)]
use crate::inode::VirtioFsInode;
use vstd::std_specs::path::PathView;

verus! {

/// `anyhow::Error` (the error of `VirtioFs::new_microvm`), by name only.
#[cfg(verus_keep_ghost)]
#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExAnyhowError(anyhow::Error);

/// The view of a `VirtioFsInode`: its plain fields, which never change (its
/// path and names are behind locks, in `VirtioFsGhostView`'s `paths` and
/// `aliases`).
pub struct VirtioFsInodeView {
    /// Its volume's id: its share. (Not its host inode number: which host
    /// object an inode is waits for a model of the host.)
    pub volume: u32,
    /// Its file type, the `S_IFMT` bits (`file_type`).
    pub kind: lx::mode_t,
    /// Its volume's read-only flag.
    pub readonly: bool,
    /// Its volume's access policy.
    pub policy: SubtreePolicyView,
    /// Its volume's exposed file, for a single-file share (the only path the
    /// guest may reach in it: `exposed_file`), else `None`.
    pub exposed: Option<PathView>,
}


/// An open file, as the guest holds it: its node (recorded when it was
/// opened; `VirtioFsFile` keeps its inode, not the node id), its inode's view
/// (`VirtioFsFile::inode`: an open file keeps its inode, even after `FORGET`
/// removes the node from the table), and whether it was opened for writing
/// (`open_flags`).
pub struct VirtioFsFileView {
    pub node: u64,
    pub inode: VirtioFsInodeView,
    pub writable: bool,
}

/// A share of an aggregate, as its root inode will be: its volume's id,
/// read-only flag and policy, and the root's path in it (`root_entry`: empty
/// for a folder, the file's name for a single-file share).
pub struct VirtioFsShareView {
    pub volume: u32,
    pub readonly: bool,
    pub policy: SubtreePolicyView,
    pub root: PathView,
    /// The exposed file, for a single-file share (then also its root).
    pub exposed: Option<PathView>,
}

/// virtio-fs's state, as the guest sees it.
pub struct VirtioFsGhostView {
    /// Each node id in the inode table, and its inode's view.
    pub inodes: Map<u64, VirtioFsInodeView>,
    /// Each node's main path in its share (not the root's: see `path_of`).
    pub paths: Map<u64, PathView>,
    /// Each node's names: every path the code knows for it, the main path
    /// among them.
    pub aliases: Map<u64, Set<PathView>>,
    /// Each open handle.
    pub files: Map<u64, VirtioFsFileView>,
    /// This file system is an aggregate: node 1 is its synthetic root, not
    /// in the inode table (`InodeMap::aggregate`). Fixed at creation.
    pub aggregate: bool,
    /// This file system is a microVM's (it has the microVM profile, which
    /// adds checks). Fixed at creation.
    pub microvm: bool,
    /// An aggregate's shares, by name: the entries of its synthetic root.
    /// Fixed at creation in the microVM.
    pub shares: Map<Seq<u8>, VirtioFsShareView>,
}

/// The value of the inode table's ghost piece: the inodes' views, and the ids
/// of the current `paths` and `aliases` maps (recorded here, in the inode
/// table's lock, so that `DESTROY`, which starts fresh maps, updates them
/// under that lock's write lock: `CODE-OVERVIEW.md`, "How the ghost state
/// keeps matching").
pub struct InodeTable {
    pub inodes: Map<u64, VirtioFsInodeView>,
    /// Each node's inode's ghost key (the key in its path lock's rule): its
    /// entry in the `paths` and `aliases` ghost maps. Hidden from the view,
    /// which is keyed by node id.
    pub keys: Map<u64, int>,
    pub paths_id: Loc,
    pub aliases_id: Loc,
    /// The table is an aggregate's (`InodeMap::aggregate`).
    pub aggregate: bool,
}

/// The ghost state, lent to each request: the four whole-map pieces (the
/// other pieces are in the locks).
pub tracked struct VirtioFsGhost {
    pub tracked inodes: FracGhost<InodeTable>,
    /// Each inode's main path, by its ghost key.
    pub tracked paths: GhostMapAuth<int, PathView>,
    /// Each inode's names, by its ghost key.
    pub tracked aliases: GhostMapAuth<int, Set<PathView>>,
    pub tracked files: FracGhost<Map<u64, VirtioFsFileView>>,
    /// The ghost key that the next inode created gets.
    pub ghost next_key: int,
    /// The file system is a microVM's (`owns` ties it to the profile).
    pub ghost microvm: bool,
    /// An aggregate's shares (`owns` ties them to its registry's lock).
    pub ghost shares: Map<Seq<u8>, VirtioFsShareView>,
}

impl View for VirtioFsGhost {
    type V = VirtioFsGhostView;

    /// The maps by node id: each node in the inode table but the root, with
    /// the entries of its inode's ghost key. Entries of no node's key (an
    /// inode built and then dropped, as `InodeMap::insert` does when the
    /// table already has the host file; or a forgotten inode) are not seen.
    closed spec fn view(&self) -> VirtioFsGhostView {
        let t = self.inodes@;
        VirtioFsGhostView {
            inodes: t.inodes,
            paths: Map::new(t.inodes.dom().remove(fuse::protocol::FUSE_ROOT_ID), |n: u64| self.paths@[t.keys[n]]),
            aliases: Map::new(t.inodes.dom().remove(fuse::protocol::FUSE_ROOT_ID), |n: u64| self.aliases@[t.keys[n]]),
            files: self.files@,
            aggregate: t.aggregate,
            microvm: self.microvm,
            shares: self.shares,
        }
    }
}

impl VirtioFsGhostView {
    /// Node `n`'s main path: the empty path for the root.
    closed spec fn path_of(self, n: u64) -> PathView {
        if n == fuse::protocol::FUSE_ROOT_ID {
            seq![]
        } else {
            self.paths[n]
        }
    }

    /// Node `n`'s names: only the empty path for the root.
    closed spec fn names_of(self, n: u64) -> Set<PathView> {
        if n == fuse::protocol::FUSE_ROOT_ID {
            set![seq![]]
        } else {
            self.aliases[n]
        }
    }
}

impl VirtioFsGhostView {
    /// The state of a new file system: the root is the only node in the inode
    /// table (an aggregate's root is synthetic: then there is none), and
    /// there are no names and no open handles.
    pub closed spec fn is_new(self) -> bool {
        &&& self.inodes.dom().subset_of(set![fuse::protocol::FUSE_ROOT_ID])
        &&& self.paths.dom() == Set::<u64>::empty()
        &&& self.aliases.dom() == Set::<u64>::empty()
        &&& self.files.dom() == Set::<u64>::empty()
    }

    /// A relative path's length in bytes on Linux: its names, and a `/`
    /// between each two.
    closed spec fn encoded_len(p: PathView) -> nat
        decreases p.len(),
    {
        if p.len() == 0 {
            0
        } else if p.len() == 1 {
            p[0].len()
        } else {
            Self::encoded_len(p.drop_last()) + 1 + p.last().len()
        }
    }

    /// Node `m` is in the inode table, in volume `v`.
    closed spec fn in_volume(self, m: u64, v: u32) -> bool {
        self.inodes.contains_key(m) && self.inodes[m].volume == v
    }

    /// Path `x` after `p` is renamed to `q`: at or below `p`, the same place
    /// below `q` (`renamed_path`, `inode.rs`).
    closed spec fn moved(x: PathView, p: PathView, q: PathView) -> PathView {
        if p.is_prefix_of(x) {
            q + x.subrange(p.len() as int, x.len() as int)
        } else {
            x
        }
    }

    /// Path `x` after `p` and `q` are swapped (`exchanged_path`, `inode.rs`).
    closed spec fn exchanged(x: PathView, p: PathView, q: PathView) -> PathView {
        if p.is_prefix_of(x) {
            Self::moved(x, p, q)
        } else if q.is_prefix_of(x) {
            Self::moved(x, q, p)
        } else {
            x
        }
    }

    /// Every name and main path of the nodes of volume `v`, changed by `f`.
    closed spec fn map_names(self, v: u32, f: spec_fn(PathView) -> PathView) -> Self {
        VirtioFsGhostView {
            paths: self.paths.map_entries(|m: u64, x: PathView| if self.in_volume(m, v) { f(x) } else { x }),
            aliases: self.aliases.map_entries(|m: u64, xs: Set<PathView>| if self.in_volume(m, v) { xs.map(f) } else { xs }),
            ..self
        }
    }
}

/// Well formed: the view's invariant, and the ids of the `paths` and
/// `aliases` maps that the inode table records are those of this state's
/// `Auth`s.
/// The rule of the inode table's lock: what the lock's value always is,
/// given when the lock is created and never changed. The lock holds a half
/// of the ghost inode table (the one with id `id`; `g` holds the other half),
/// and the inodes it records, by node id, are the table's inodes (their
/// views).
///
/// SPEC: the core of the rule is part of the spec: the half is the ghost
/// inode table's (`id`), and its inodes are the real table's (`inodes`). It
/// is what makes the contracts' nodes the nodes the code holds.
/// TODO (proof agent): the rest (e.g. that the half is a half, that each
/// table inode's locks' rules have the recorded key and the maps' ids, and
/// the keys, `InodeTable.keys`, against those rules); it may add facts or
/// fields, but must keep the core.
// Only Verus reads the rule's fields.
#[cfg_attr(not(verus_keep_ghost), allow(dead_code))]
pub ghost struct InodeTableRule {
    /// The id of the ghost inode table.
    pub id: Loc,
}

impl RwLockPredicate<InodeTableLocked> for InodeTableRule {
    closed spec fn inv(self, v: InodeTableLocked) -> bool {
        &&& v.half@.id() == self.id
        &&& v.half@@.inodes == v.table.inodes_by_node_id.values@.map_values(|i: Arc<VirtioFsInode>| i@)
        &&& v.half@@.aggregate == v.table.aggregate
    }
}

/// The rule of the file table's lock: the lock holds a half of the ghost
/// file table (the one with id `id`; `g` holds the other half), whose handles
/// are the table's, each with its open file's inode (its view) and whether
/// it was opened for writing (from its open flags).
///
/// SPEC: the core, as for `InodeTableRule` (`id`, the handles, and each
/// handle's inode and writability).
/// TODO (proof agent): the rest (e.g. that the half is a half, and that each
/// handle's recorded node, `VirtioFsFileView::node`, is a node whose inode
/// the file's is, while the node is in the table); it may add facts or
/// fields, but must keep the core.
// Only Verus reads the rule's fields.
#[cfg_attr(not(verus_keep_ghost), allow(dead_code))]
pub ghost struct FileTableRule {
    /// The id of the ghost file table.
    pub id: Loc,
}

impl RwLockPredicate<FileTableLocked> for FileTableRule {
    closed spec fn inv(self, v: FileTableLocked) -> bool {
        let files = v.table.values@;
        &&& v.half@.id() == self.id
        &&& v.half@@.dom() == files.dom()
        &&& forall|fh: u64| #[trigger] files.contains_key(fh) ==> {
            let f: VirtioFsFile = *files[fh];
            &&& v.half@@[fh].inode == f.inode@
            &&& v.half@@[fh].writable == <VirtioFsGhostView as FuseState>::opens_for_writing(f.open_flags)
        }
    }
}

impl FuseGhost for VirtioFsGhost {
    /// The view's facts (`inv`); the recorded ids are the maps'; each node
    /// in the table but the root has a ghost key, with entries in both maps,
    /// no two such nodes share one, and every key in use is below `next_key`
    /// (so the next inode's key is fresh).
    closed spec fn wf(&self) -> bool {
        let t = self.inodes@;
        &&& self@.inv()
        &&& t.paths_id == self.paths.id()
        &&& t.aliases_id == self.aliases.id()
        &&& forall|n: u64| #[trigger] t.inodes.contains_key(n) && n != fuse::protocol::FUSE_ROOT_ID ==> {
            &&& t.keys.contains_key(n)
            &&& self.paths@.contains_key(t.keys[n])
            &&& self.aliases@.contains_key(t.keys[n])
        }
        &&& forall|n1: u64, n2: u64| #[trigger] t.inodes.contains_key(n1) && #[trigger] t.inodes.contains_key(n2)
            && n1 != n2 && n1 != fuse::protocol::FUSE_ROOT_ID && n2 != fuse::protocol::FUSE_ROOT_ID ==> t.keys[n1] != t.keys[n2]
        &&& forall|k: int| #[trigger] self.paths@.contains_key(k) ==> k < self.next_key
        &&& forall|k: int| #[trigger] self.aliases@.contains_key(k) ==> k < self.next_key
    }
}

/// `g` is this file system's: `g`'s halves of the inode table and the file
/// table are the other halves of the ones this file system's two table locks
/// hold (their rules pin the ids). Through the inode table, this also
/// reaches each inode's path and names: the table records the ids of `g`'s
/// `paths` and `aliases` maps (`wf`), and each inode's locks hold its entries
/// of them (the locks' rules).
///
/// SPEC.
impl FuseOwns<VirtioFsGhost> for crate::VirtioFs {
    closed spec fn owns(&self, g: &VirtioFsGhost) -> bool {
        &&& self.inner.inodes.pred().id == g.inodes.id()
        &&& self.inner.files.pred().id == g.files.id()
        &&& g.microvm == (self.inner.microvm_profile is Some)
        // An aggregate's shares are its registry's: fixed (and recorded in
        // the registry lock's rule) in the microVM.
        &&& g@.aggregate == (self.inner.mode is Aggregate)
        &&& match self.inner.mode {
            crate::VirtioFsMode::Aggregate(state) => {
                &&& state.fixed() == g.microvm
                &&& g.microvm ==> state.shares() == g.shares
            },
            crate::VirtioFsMode::Direct => g.shares == Map::<Seq<u8>, VirtioFsShareView>::empty(),
        }
    }
}

/// A dormant device has no share, so no node, name or handle: its ghost
/// state is the empty one.
impl FuseOwns<VirtioFsGhost> for crate::microvm::device::DormantMicrovmFs {
    open spec fn owns(&self, g: &VirtioFsGhost) -> bool {
        g@ == VirtioFsGhostView {
            inodes: Map::empty(),
            paths: Map::empty(),
            aliases: Map::empty(),
            files: Map::empty(),
            aggregate: false,
            microvm: true,
            shares: Map::empty(),
        }
    }
}

/// `SectionFs` (Windows only) is not part of the microVM; its ghost state is
/// not tied to its data (no contract of its operations is proved).
#[cfg(windows)]
impl FuseOwns<VirtioFsGhost> for crate::section::SectionFs {
    open spec fn owns(&self, g: &VirtioFsGhost) -> bool {
        true
    }
}

impl FuseState for VirtioFsGhostView {
    /// The structural facts that every operation keeps: every node in the
    /// inode table but the root has a main path and names, and its main path
    /// is among its names when it has any (a node whose names were all
    /// removed keeps its last main path); and an aggregate's shares have
    /// plain names (the profile accepts only letters, digits, `.`, `_` and
    /// `-`, and not `.` or `..`: `MicroVmAggregateChild::new`).
    closed spec fn inv(&self) -> bool {
        &&& forall|m: u64| #[trigger] self.inodes.contains_key(m) && m != fuse::protocol::FUSE_ROOT_ID ==> {
            &&& self.paths.contains_key(m)
            &&& self.aliases.contains_key(m)
            &&& !self.aliases[m].is_empty() ==> self.aliases[m].contains(self.paths[m])
        }
        &&& forall|k: Seq<u8>| #[trigger] self.shares.contains_key(k) ==> Self::is_plain_name(k)
    }

    /// A node of the inode table, or an aggregate's root (synthetic, not in
    /// the table, and always there).
    closed spec fn holds_node(&self, n: u64) -> bool {
        self.inodes.contains_key(n) || self.is_synthetic(n)
    }

    closed spec fn is_dir(&self, n: u64) -> bool {
        ||| self.is_synthetic(n)
        ||| self.inodes.contains_key(n) && self.inodes[n].kind == lx::S_IFDIR
    }

    closed spec fn child_path(&self, parent: u64, name: Seq<u8>) -> PathView {
        self.path_of(parent).push(name)
    }

    /// The inode's volume is not read-only, and its policy lets the guest
    /// change `p`.
    /// (Never under an aggregate's root, which is read-only.)
    closed spec fn can_write(&self, n: u64, p: PathView) -> bool {
        !self.is_synthetic(n) && !self.inodes[n].readonly && self.inodes[n].policy.can_write(p)
    }

    /// Through its main path and every name of it (`readonly()`:
    /// `policy_permits_writes` checks the primary path and every alias).
    closed spec fn node_can_write(&self, n: u64) -> bool {
        &&& self.can_write(n, self.path_of(n))
        &&& forall|q: PathView| #[trigger] self.names_of(n).contains(q) ==> self.can_write(n, q)
    }

    /// `p` is not hidden under the parent's policy, and a traverse-only
    /// path is a directory. The volume's pinned identities are left out
    /// until the spec has a model of the host (decided 2026-11-10): that rule
    /// is about which object is at `p`, which only the host knows.
    /// Under an aggregate's root: `p` is one share's name
    /// (`lookup_synthetic_root`).
    closed spec fn may_reach(&self, parent: u64, p: PathView, kind: lx::mode_t) -> bool {
        if self.is_synthetic(parent) {
            &&& p.len() == 1
            &&& self.shares.contains_key(p[0])
            // A single-file share's root is a regular file.
            &&& self.shares[p[0]].exposed is Some ==> kind == lx::S_IFREG
        } else {
            let inode = self.inodes[parent];
            &&& !self.is_hidden(parent, p)
            &&& inode.policy.visibility(p) == crate::microvm::policy::PathVisibility::TraverseOnly ==> kind == lx::S_IFDIR
            // A single-file share's one object is a regular file
            // (`ensure_object_allowed`).
            &&& inode.exposed is Some ==> kind == lx::S_IFREG
        }
    }

    /// Under an aggregate's root, the node reached at `p` (a share's name)
    /// is that share's root (`insert_child_root_entry`): in its share, with
    /// its root's path there, not `p`.
    closed spec fn after_reach(&self, parent: u64, p: PathView, m: u64, kind: lx::mode_t) -> Self {
        let q = if self.is_synthetic(parent) { self.shares[p[0]].root } else { p };
        if self.inodes.contains_key(m) {
            VirtioFsGhostView {
                paths: self.paths.insert(m, q),
                aliases: self.aliases.insert(m, self.aliases[m].insert(q)),
                ..*self
            }
        } else {
            // The new inode's view: in the parent's volume, or the share's
            // for a share's root.
            let inode = if self.is_synthetic(parent) {
                let share = self.shares[p[0]];
                VirtioFsInodeView {
                    volume: share.volume,
                    kind,
                    readonly: share.readonly,
                    policy: share.policy,
                    exposed: share.exposed,
                }
            } else {
                VirtioFsInodeView { kind, ..self.inodes[parent] }
            };
            VirtioFsGhostView {
                inodes: self.inodes.insert(m, inode),
                paths: self.paths.insert(m, q),
                aliases: self.aliases.insert(m, set![q]),
                ..*self
            }
        }
    }

    /// The two inodes' volumes are the same.
    closed spec fn same_share(&self, a: u64, b: u64) -> bool {
        self.inodes[a].volume == self.inodes[b].volume
    }

    closed spec fn after_link(&self, m: u64, p: PathView) -> Self {
        VirtioFsGhostView { aliases: self.aliases.insert(m, self.aliases[m].insert(p)), ..*self }
    }

    closed spec fn unlinked(&self, n: u64, p: PathView, after: Self) -> bool {
        let v = self.inodes[n].volume;
        &&& after.inodes == self.inodes
        &&& after.files == self.files
        &&& after.aliases.dom() == self.aliases.dom()
        &&& after.paths.dom() == self.paths.dom()
        // Each node of the share loses the names at or below `p`.
        &&& forall|m: u64| #[trigger] self.aliases.contains_key(m) ==> after.aliases[m] == if self.in_volume(m, v) {
            self.aliases[m].filter(|x: PathView| !p.is_prefix_of(x))
        } else {
            self.aliases[m]
        }
        // One whose main path was among them takes a remaining name, if any.
        &&& forall|m: u64| #[trigger] self.paths.contains_key(m) ==> if self.in_volume(m, v) && p.is_prefix_of(self.paths[m]) && !after.aliases[m].is_empty() {
            after.aliases[m].contains(after.paths[m])
        } else {
            after.paths[m] == self.paths[m]
        }
    }

    closed spec fn renamed(&self, n: u64, p: PathView, q: PathView, exchange: bool, after: Self) -> bool {
        let v = self.inodes[n].volume;
        if p == q {
            // A rename of a path onto itself changes nothing (POSIX `rename(2)`).
            after == *self
        } else if exchange {
            after == self.map_names(v, |x: PathView| Self::exchanged(x, p, q))
        } else {
            exists|mid: Self| #[trigger] self.unlinked(n, q, mid) && after == mid.map_names(v, |x: PathView| Self::moved(x, p, q))
        }
    }

    /// A handle of the file table, or an aggregate root's own handle
    /// (`open_dir` hands it out, not in the file table, and it is always
    /// there).
    closed spec fn holds_file(&self, fh: u64) -> bool {
        self.files.contains_key(fh) || self.is_synthetic_handle(fh)
    }

    /// The node leaves the table, with its main path and names (an open
    /// file of it keeps its inode, but the guest can no longer use the
    /// handle: `get_file` needs the node).
    closed spec fn after_forget(&self, n: u64) -> Self {
        VirtioFsGhostView {
            inodes: self.inodes.remove(n),
            paths: self.paths.remove(n),
            aliases: self.aliases.remove(n),
            ..*self
        }
    }

    /// The file table is empty, the inode table keeps only the root (if it
    /// is there: an aggregate's root is synthetic, not in the table), and
    /// the `paths` and `aliases` maps start afresh (`DESTROY` replaces their
    /// `Auth`s, and node ids start over).
    closed spec fn after_destroy(&self) -> Self {
        VirtioFsGhostView {
            inodes: self.inodes.restrict(set![fuse::protocol::FUSE_ROOT_ID]),
            paths: Map::empty(),
            aliases: Map::empty(),
            files: Map::empty(),
            aggregate: self.aggregate,
            microvm: self.microvm,
            shares: self.shares,
        }
    }

    closed spec fn after_release(&self, fh: u64) -> Self {
        VirtioFsGhostView { files: self.files.remove(fh), ..*self }
    }

    /// (An aggregate root's own handle: the root's.)
    closed spec fn file_node(&self, fh: u64) -> u64 {
        if self.is_synthetic_handle(fh) { fuse::protocol::FUSE_ROOT_ID } else { self.files[fh].node }
    }

    /// The open file's flags were for writing (`open_flags`; not an
    /// aggregate root's own handle).
    closed spec fn opened_for_writing(&self, fh: u64) -> bool {
        !self.is_synthetic_handle(fh) && self.files[fh].writable
    }

    /// The file's node's main path and every name of it are writable under
    /// its inode's volume (the file's own copy of the inode's view). Used
    /// with `uses_handle`, so the node is in the table.
    closed spec fn file_can_write(&self, fh: u64) -> bool {
        let f = self.files[fh];
        &&& !self.is_synthetic_handle(fh)
        &&& !f.inode.readonly && f.inode.policy.can_write(self.path_of(f.node))
        &&& forall|q: PathView| #[trigger] self.names_of(f.node).contains(q) ==> !f.inode.readonly && f.inode.policy.can_write(q)
    }

    closed spec fn after_open(&self, fh: u64, m: u64, writable: bool) -> Self {
        VirtioFsGhostView {
            files: self.files.insert(fh, VirtioFsFileView { node: m, inode: self.inodes[m], writable }),
            ..*self
        }
    }

    /// An aggregate's root (node 1, `VirtioFs::is_synthetic_root`).
    closed spec fn is_synthetic(&self, n: u64) -> bool {
        self.aggregate && n == fuse::protocol::FUSE_ROOT_ID
    }

    /// An aggregate root's own handle (`SYNTHETIC_ROOT_FH`, `u64::MAX`:
    /// `VirtioFs::is_synthetic_root_handle`).
    closed spec fn is_synthetic_handle(&self, fh: u64) -> bool {
        self.aggregate && fh == u64::MAX
    }

    /// Any code: without a model of the host, the spec cannot say which
    /// codes the host fails with (lxutil hands on the host's own). A host
    /// model may narrow it; it would then also need, as rules, the failures
    /// the code checks that the rules leave out (see `CODE-OVERVIEW.md`,
    /// "When an operation fails").
    closed spec fn host_error(code: i32) -> bool {
        true
    }

    /// A flag outside those `util::create_flags` accepts.
    closed spec fn bad_create_flags(flags: u32) -> bool {
        let supported = (lx::O_ACCESS_MASK | lx::O_CREAT | lx::O_EXCL | lx::O_TRUNC | lx::O_APPEND
            | lx::O_NONBLOCK | lx::O_DSYNC | lx::O_DIRECT | lx::O_DIRECTORY | lx::O_NOATIME
            | lx::O_SYNC | lx::__FMODE_EXEC | lx::O_LARGEFILE | lx::O_NOFOLLOW) as u32;
        flags & !supported != 0u32
    }

    /// By the share's policy, or, in a single-file share, any path but its
    /// file (`ensure_path_allowed`, which `child_path` and the re-check at
    /// each operation apply; an aggregate's root has no policy).
    closed spec fn is_hidden(&self, n: u64, p: PathView) -> bool {
        let inode = self.inodes[n];
        &&& !self.is_synthetic(n)
        &&& {
            ||| inode.policy.visibility(p) == crate::microvm::policy::PathVisibility::Hidden
            // In a single-file share, every path but the exposed file.
            ||| inode.exposed matches Some(file) && p != file
        }
    }

    /// In the microVM (strict paths: `child_name_disallowed`, which
    /// `child_path` applies to the guest's name before anything else): a
    /// name with `\` or `:` (refused, so that saved names are valid on any
    /// host; `lxutil` escapes both on Windows, so neither is a separator
    /// there).
    closed spec fn refuses_name(&self, n: u64, name: Seq<u8>) -> bool {
        self.microvm && (name.contains('\\' as u8) || name.contains(':' as u8))
    }

    /// In the microVM, on Linux: longer than `MAX_PATH_BYTES` (4096) bytes,
    /// as the code measures it there (the path's bytes, separators included:
    /// `relative_path_encoded_len`, which `validate_child_path` applies). On
    /// Windows the code measures UTF-16 bytes, which may be fewer: no rule
    /// there.
    #[cfg(unix)]
    closed spec fn path_too_long(&self, n: u64, p: PathView) -> bool {
        self.microvm && Self::encoded_len(p) > 4096
    }

    /// (Windows: see the Linux version.)
    #[cfg(not(unix))]
    closed spec fn path_too_long(&self, n: u64, p: PathView) -> bool {
        false
    }

    /// Over `MAX_GUEST_BUFFER_SIZE` (1 MiB: `guest_buffer`, `write`).
    closed spec fn buffer_too_large(size: nat) -> bool {
        size > 1024 * 1024
    }

    /// The listing rule (`entry_listable`): the entry's path under `dir`'s
    /// path now is visible, or traverse-only and a directory (an entry whose
    /// type the host reported as unknown, `DT_UNK`, is listed only if the
    /// host says it is a directory: left to the host model). In an
    /// aggregate's root: a share's name.
    closed spec fn may_list(&self, dir: u64, name: Seq<u8>, kind: u32) -> bool {
        if self.is_synthetic(dir) {
            self.shares.contains_key(name)
        } else {
            let visibility = self.inodes[dir].policy.visibility(self.child_path(dir, name));
            ||| visibility == crate::microvm::policy::PathVisibility::Visible
            ||| visibility == crate::microvm::policy::PathVisibility::TraverseOnly
                && (kind == lx::DT_DIR as u32 || kind == lx::DT_UNK as u32)
        }
    }

    /// In the microVM (whose volumes have strict paths): none of the node's
    /// names, nor its main path, is hidden (`get_inode` ->
    /// `validate_confined_object`, `microvm/inode.rs`). Always, outside the
    /// microVM, and for an aggregate's root (neither is checked).
    closed spec fn names_visible(&self, n: u64) -> bool {
        self.microvm && !self.is_synthetic(n) ==> {
            &&& !self.is_hidden(n, self.path_of(n))
            &&& forall|q: PathView| #[trigger] self.names_of(n).contains(q) ==> !self.is_hidden(n, q)
        }
    }

    /// The node's names are not all gone (`link` checks its aliases: its
    /// main path outlives them, and may name another object by then).
    closed spec fn has_names(&self, n: u64) -> bool {
        !self.names_of(n).is_empty()
    }

    /// In the microVM (`microvm::fs::validate_mknod_mode`): any file type
    /// but a regular file (or 0, which is one), a FIFO or a socket: never a
    /// device node.
    closed spec fn refuses_mknod(&self, mode: u32) -> bool {
        let kind = mode & lx::S_IFMT;
        &&& self.microvm
        &&& !(kind == 0 || kind == lx::S_IFREG || kind == lx::S_IFIFO || kind == lx::S_IFSOCK)
    }

    /// `O_CREAT` or `O_EXCL` (`util::open_flags`: `open` does not create), or
    /// a flag `create` does not support either.
    closed spec fn bad_open_flags(flags: u32) -> bool {
        ||| flags & ((lx::O_CREAT | lx::O_EXCL) as u32) != 0u32
        ||| Self::bad_create_flags(flags)
    }

    /// In the microVM (`microvm::fs::validate_symlink_target`): an empty
    /// target (`ENOENT`), or one longer than 4095 bytes (`ENAMETOOLONG`,
    /// `MAX_SYMLINK_TARGET_BYTES`), as Linux does.
    closed spec fn bad_symlink_target(&self, target: Seq<u8>, code: i32) -> bool {
        &&& self.microvm
        &&& {
            ||| target.len() == 0 && code == lx::ENOENT
            ||| target.len() > 4095 && code == lx::ENAMETOOLONG
        }
    }
}

} // verus!
