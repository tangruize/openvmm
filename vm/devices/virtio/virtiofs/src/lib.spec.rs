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
use vstd::std_specs::path::PathView;

verus! {

/// Types that the inode and the volume hold, declared to Verus by name only,
/// so that their views can read their other fields (`parking_lot`'s lock
/// types are declared in the `fuse` crate, which also needs them).
#[cfg(verus_keep_ghost)]
#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExLxVolume(lxutil::LxVolume);

/// The view of a `VirtioFsInode`: its plain fields, which never change (its
/// path and names are behind locks, in `VirtioFsGhostView`'s `paths` and
/// `aliases`).
pub struct VirtioFsInodeView {
    /// Its host object, as the inode table identifies it (`DedupKey::Ino`):
    /// its volume's id and its inode number.
    pub object: (u32, u64),
    /// Its file type, the `S_IFMT` bits (`file_type`).
    pub kind: lx::mode_t,
    /// Its volume's read-only flag.
    pub readonly: bool,
    /// Its volume's access policy.
    pub policy: SubtreePolicyView,
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
}

/// The value of the inode table's ghost piece: the inodes' views, and the ids
/// of the current `paths` and `aliases` maps (recorded here, in the inode
/// table's lock, so that `DESTROY`, which starts fresh maps, updates them
/// under that lock's write lock: `CODE-OVERVIEW.md`, "How the ghost state
/// keeps matching").
pub struct InodeTable {
    pub inodes: Map<u64, VirtioFsInodeView>,
    pub paths_id: Loc,
    pub aliases_id: Loc,
}

/// The ghost state, lent to each request: the four whole-map pieces (the
/// other pieces are in the locks).
pub tracked struct VirtioFsGhost {
    pub tracked inodes: FracGhost<InodeTable>,
    pub tracked paths: GhostMapAuth<u64, PathView>,
    pub tracked aliases: GhostMapAuth<u64, Set<PathView>>,
    pub tracked files: FracGhost<Map<u64, VirtioFsFileView>>,
}

impl View for VirtioFsGhost {
    type V = VirtioFsGhostView;

    closed spec fn view(&self) -> VirtioFsGhostView {
        VirtioFsGhostView {
            inodes: self.inodes@.inodes,
            paths: self.paths@,
            aliases: self.aliases@,
            files: self.files@,
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
    /// Node `m` is in the inode table, in volume `v`.
    closed spec fn in_volume(self, m: u64, v: u32) -> bool {
        self.inodes.contains_key(m) && self.inodes[m].object.0 == v
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
impl FuseGhost for VirtioFsGhost {
    closed spec fn wf(&self) -> bool {
        &&& self@.inv()
        &&& self.inodes@.paths_id == self.paths.id()
        &&& self.inodes@.aliases_id == self.aliases.id()
    }
}

/// `g` is this file system's: `g`'s pieces are the ones its locks hold.
///
/// PLACEHOLDER (`true`): the locks do not hold ghost pieces yet (they are
/// `parking_lot` locks); the body comes with the locks' rules, next: the id
/// of `g`'s inode-table half is the one that this file system's inode-table
/// lock pins, and likewise for the file table. Open item in
/// `virtiofs-spec-docs/CODE-OVERVIEW.md`.
impl FuseOwns<VirtioFsGhost> for crate::VirtioFs {
    closed spec fn owns(&self, g: &VirtioFsGhost) -> bool {
        true
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
    /// removed keeps its last main path); every open handle's node has
    /// names (an open file keeps its inode, and its entries, after `FORGET`).
    closed spec fn inv(&self) -> bool {
        &&& forall|m: u64| #[trigger] self.inodes.contains_key(m) && m != fuse::protocol::FUSE_ROOT_ID ==> {
            &&& self.paths.contains_key(m)
            &&& self.aliases.contains_key(m)
            &&& !self.aliases[m].is_empty() ==> self.aliases[m].contains(self.paths[m])
        }
        &&& forall|fh: u64| #[trigger] self.files.contains_key(fh) ==> {
            let m = self.files[fh].node;
            m == fuse::protocol::FUSE_ROOT_ID || self.aliases.contains_key(m)
        }
    }

    closed spec fn holds_node(&self, n: u64) -> bool {
        self.inodes.contains_key(n)
    }

    closed spec fn is_dir(&self, n: u64) -> bool {
        self.inodes.contains_key(n) && self.inodes[n].kind == lx::S_IFDIR
    }

    closed spec fn child_path(&self, parent: u64, name: Seq<u8>) -> PathView {
        self.path_of(parent).push(name)
    }

    /// The inode's volume is not read-only, and its policy lets the guest
    /// change `p`.
    closed spec fn can_write(&self, n: u64, p: PathView) -> bool {
        !self.inodes[n].readonly && self.inodes[n].policy.can_write(p)
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
    closed spec fn may_reach(&self, parent: u64, p: PathView, kind: lx::mode_t) -> bool {
        let policy = self.inodes[parent].policy;
        &&& policy.visibility(p) != crate::microvm::policy::PathVisibility::Hidden
        &&& policy.visibility(p) == crate::microvm::policy::PathVisibility::TraverseOnly ==> kind == lx::S_IFDIR
    }

    closed spec fn after_reach(&self, parent: u64, p: PathView, m: u64, kind: lx::mode_t) -> Self {
        if self.inodes.contains_key(m) {
            VirtioFsGhostView {
                paths: self.paths.insert(m, p),
                aliases: self.aliases.insert(m, self.aliases[m].insert(p)),
                ..*self
            }
        } else {
            let parent_inode = self.inodes[parent];
            VirtioFsGhostView {
                // The new inode is in the parent's volume; its object is
                // what the host reported (left open in this draft).
                inodes: self.inodes.insert(m, VirtioFsInodeView { kind, ..parent_inode }),
                paths: self.paths.insert(m, p),
                aliases: self.aliases.insert(m, set![p]),
                ..*self
            }
        }
    }

    /// The two inodes' volumes are the same (an inode's object is its
    /// volume's id and its inode number).
    closed spec fn same_share(&self, a: u64, b: u64) -> bool {
        self.inodes[a].object.0 == self.inodes[b].object.0
    }

    closed spec fn after_link(&self, m: u64, p: PathView) -> Self {
        VirtioFsGhostView { aliases: self.aliases.insert(m, self.aliases[m].insert(p)), ..*self }
    }

    closed spec fn unlinked(&self, n: u64, p: PathView, after: Self) -> bool {
        let v = self.inodes[n].object.0;
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
        let v = self.inodes[n].object.0;
        if exchange {
            after == self.map_names(v, |x: PathView| Self::exchanged(x, p, q))
        } else {
            exists|mid: Self| #[trigger] self.unlinked(n, q, mid) && after == mid.map_names(v, |x: PathView| Self::moved(x, p, q))
        }
    }

    closed spec fn holds_file(&self, fh: u64) -> bool {
        self.files.contains_key(fh)
    }

    /// The node leaves the inode table. Its entries in `paths` and
    /// `aliases` stay (an open file may still hold its inode), and so do its
    /// open files.
    closed spec fn after_forget(&self, n: u64) -> Self {
        VirtioFsGhostView { inodes: self.inodes.remove(n), ..*self }
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
        }
    }

    closed spec fn after_release(&self, fh: u64) -> Self {
        VirtioFsGhostView { files: self.files.remove(fh), ..*self }
    }

    closed spec fn file_node(&self, fh: u64) -> u64 {
        self.files[fh].node
    }

    /// The open file's flags were for writing (`open_flags`).
    closed spec fn opened_for_writing(&self, fh: u64) -> bool {
        self.files[fh].writable
    }

    /// Every name of the file's node is writable under its inode's volume
    /// (the file's own copy of the inode's view, which holds even when the
    /// node is no longer in the table).
    closed spec fn file_can_write(&self, fh: u64) -> bool {
        let f = self.files[fh];
        &&& !f.inode.readonly && f.inode.policy.can_write(self.path_of(f.node))
        &&& forall|q: PathView| #[trigger] self.names_of(f.node).contains(q) ==> !f.inode.readonly && f.inode.policy.can_write(q)
    }

    closed spec fn after_open(&self, fh: u64, m: u64, writable: bool) -> Self {
        VirtioFsGhostView {
            files: self.files.insert(fh, VirtioFsFileView { node: m, inode: self.inodes[m], writable }),
            ..*self
        }
    }
}

} // verus!
