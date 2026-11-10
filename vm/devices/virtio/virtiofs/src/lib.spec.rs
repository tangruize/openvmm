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
use fuse::spec::FuseState;
use vstd::prelude::*;
use vstd::resource::frac::FracGhost;
use vstd::resource::map::GhostMapAuth;
use vstd::std_specs::path::PathView;

verus! {

/// Types that the inode and the volume hold, declared to Verus by name only,
/// so that their views can read their other fields.
#[cfg(verus_keep_ghost)]
#[verifier::external_type_specification]
#[verifier::external_body]
#[verifier::reject_recursive_types(R)]
#[verifier::reject_recursive_types(T)]
pub struct ExRwLock<R, T: ?Sized>(parking_lot::lock_api::RwLock<R, T>);

#[cfg(verus_keep_ghost)]
#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExLxVolume(lxutil::LxVolume);

#[cfg(verus_keep_ghost)]
#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExRawRwLock(parking_lot::RawRwLock);

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
/// opened; `VirtioFsFile` keeps its inode, not the node id), and whether it
/// was opened for writing (`open_flags`).
pub struct VirtioFsFileView {
    pub node: u64,
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

/// The ghost state, lent to each request: the four whole-map pieces (the
/// other pieces are in the locks).
pub tracked struct VirtioFsGhost {
    pub tracked inodes: FracGhost<Map<u64, VirtioFsInodeView>>,
    pub tracked paths: GhostMapAuth<u64, PathView>,
    pub tracked aliases: GhostMapAuth<u64, Set<PathView>>,
    pub tracked files: FracGhost<Map<u64, VirtioFsFileView>>,
}

impl View for VirtioFsGhost {
    type V = VirtioFsGhostView;

    closed spec fn view(&self) -> VirtioFsGhostView {
        VirtioFsGhostView {
            inodes: self.inodes@,
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

impl FuseState for VirtioFsGhostView {
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

    closed spec fn node_can_write(&self, n: u64) -> bool {
        forall|q: PathView| #[trigger] self.names_of(n).contains(q) ==> self.can_write(n, q)
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

    closed spec fn holds_file(&self, fh: u64) -> bool {
        self.files.contains_key(fh)
    }

    closed spec fn after_open(&self, fh: u64, m: u64, writable: bool) -> Self {
        VirtioFsGhostView {
            files: self.files.insert(fh, VirtioFsFileView { node: m, writable }),
            ..*self
        }
    }
}

} // verus!
