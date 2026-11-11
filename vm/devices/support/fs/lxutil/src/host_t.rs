// The host model: what a share holds on the host disk, and the disk value
// that each volume's calls read and change.
//
// TRUSTED, do not attempt to prove anything in this file. Verus cannot see
// the host's disk, so the disk value is declared only (`VolumeDisk`: no
// fields, nothing constructs it but `LxVolumeOptions::new_volume`), and its
// view is trusted to match the real disk. That rests on the host
// assumptions (`lib.spec.rs`): no other host process changes the share, and
// no two volumes share a host directory, so only the volume's own calls
// change what its disk holds, and each call's contract (in `lib.rs`) says
// how; and an object's inode number is the same on every call (always on
// Linux; on Windows, a share's filesystem must report stable file ids,
// `supports_stable_file_id`). `lxutil`'s method bodies are trusted to meet those contracts (they
// are not verified).
//
// Paths are share paths in the host's form (`LxVolume::is_share_path`), the
// same values the volume's methods take; the empty path is the share's root.

use super::LxFile;
use super::LxVolume;
use vstd::prelude::*;
use vstd::std_specs::path::PathView;

verus! {

/// One object on the host disk: its device and inode numbers (its identity)
/// and its file type (the `S_IFMT` bits).
pub struct HostObject {
    /// Its device number.
    pub dev: u64,
    /// Its inode number.
    pub ino: u64,
    /// Its file type (`S_IFREG`, `S_IFDIR`, `S_IFLNK`, ...).
    pub kind: lx::mode_t,
}

/// What a share holds on the host disk: the object at each path where there
/// is one. Two paths may hold the same object (hard links).
pub struct HostDisk {
    /// The object at each path.
    pub objects: IMap<PathView, HostObject>,
}

impl HostDisk {
    /// The disk after a rename of path `p` to path `q` (not an exchange):
    /// what was at or below `q` is gone, and what was at or below `p` is at
    /// the same place below `q`. (With `p == q`, the disk as it is.)
    pub open spec fn renamed(self, p: PathView, q: PathView) -> HostDisk {
        HostDisk {
            objects: IMap::new(
                |x: PathView|
                    if q.is_prefix_of(x) {
                        self.objects.contains_key(p + x.subrange(q.len() as int, x.len() as int))
                    } else {
                        !p.is_prefix_of(x) && self.objects.contains_key(x)
                    },
                |x: PathView|
                    if q.is_prefix_of(x) {
                        self.objects[p + x.subrange(q.len() as int, x.len() as int)]
                    } else {
                        self.objects[x]
                    },
            ),
        }
    }

    /// The disk after an exchange of paths `p` and `q`: what was at or below
    /// each is at the same place below the other.
    pub open spec fn exchanged(self, p: PathView, q: PathView) -> HostDisk {
        HostDisk {
            objects: IMap::new(
                |x: PathView|
                    if q.is_prefix_of(x) {
                        self.objects.contains_key(p + x.subrange(q.len() as int, x.len() as int))
                    } else if p.is_prefix_of(x) {
                        self.objects.contains_key(q + x.subrange(p.len() as int, x.len() as int))
                    } else {
                        self.objects.contains_key(x)
                    },
                |x: PathView|
                    if q.is_prefix_of(x) {
                        self.objects[p + x.subrange(q.len() as int, x.len() as int)]
                    } else if p.is_prefix_of(x) {
                        self.objects[q + x.subrange(p.len() as int, x.len() as int)]
                    } else {
                        self.objects[x]
                    },
            ),
        }
    }

    /// No object on the disk has inode number `ino`: what a newly made
    /// object's number is (the host gives a new object a number no other
    /// object has).
    pub open spec fn is_new_ino(self, ino: u64) -> bool {
        forall|x: PathView| #[trigger] self.objects.contains_key(x) ==> self.objects[x].ino != ino
    }

    /// Nothing lies strictly below path `p` (an empty directory, or not a
    /// directory).
    pub open spec fn nothing_below(self, p: PathView) -> bool {
        forall|x: PathView| #[trigger] self.objects.contains_key(x) && p.is_prefix_of(x) ==> x == p
    }

    /// No object has both a path where `writable` holds and a path where
    /// `visible` holds but `writable` does not: a host hard link never joins
    /// a writable path and a visible read-only one.
    pub open spec fn no_mixed_links(self, writable: spec_fn(PathView) -> bool, visible: spec_fn(PathView) -> bool) -> bool {
        forall|x: PathView, y: PathView|
            #[trigger] self.objects.contains_key(x) && #[trigger] self.objects.contains_key(y)
            && self.objects[x].dev == self.objects[y].dev && self.objects[x].ino == self.objects[y].ino
            && writable(x) && visible(y) ==> writable(y)
    }

    /// A directory has one path: no two paths hold the same directory.
    pub open spec fn directories_have_one_path(self) -> bool {
        forall|x: PathView, y: PathView|
            #[trigger] self.objects.contains_key(x) && #[trigger] self.objects.contains_key(y)
            && self.objects[x].dev == self.objects[y].dev && self.objects[x].ino == self.objects[y].ino
            && self.objects[x].kind == lx::S_IFDIR ==> x == y
    }

    /// Path `p`'s parent is a directory on the disk (the share's root has
    /// no parent).
    pub open spec fn parent_is_dir(self, p: PathView) -> bool {
        p.len() > 0 ==> {
            &&& self.objects.contains_key(p.drop_last())
            &&& self.objects[p.drop_last()].kind == lx::S_IFDIR
        }
    }

    /// The disk is a tree: every object's parent is a directory on the disk.
    /// So every step but the last of an object's path is a directory, and no
    /// path to an object passes through a symbolic link.
    pub open spec fn is_tree(self) -> bool {
        forall|x: PathView| #[trigger] self.objects.contains_key(x) ==> self.parent_is_dir(x)
    }

    /// The host assumptions about the share as attached (`attached_disk`).
    pub open spec fn as_attached(self, writable: spec_fn(PathView) -> bool, visible: spec_fn(PathView) -> bool) -> bool {
        &&& self.no_mixed_links(writable, visible)
        &&& self.directories_have_one_path()
        &&& self.is_tree()
    }

    /// `after` is this disk, or this disk with one new object at path `p`,
    /// where there was none: what a call that may have made `p` before it
    /// failed leaves.
    pub open spec fn unchanged_or_made(self, p: PathView, after: HostDisk) -> bool {
        ||| after == self
        ||| !self.objects.contains_key(p) && self.parent_is_dir(p)
            && exists|o: HostObject| after.objects == #[trigger] self.objects.insert(p, o)
    }
}

/// The disk behind one volume. TRUSTED: its view is what the volume's host
/// directory holds now. Declared only: Verus cannot see the disk, and only
/// `LxVolumeOptions::new_volume` hands one out, with its volume.
#[verifier::external_body]
pub struct VolumeDisk {
    _private: (),
}

impl VolumeDisk {
    /// The volume whose disk this is (`LxVolume::id`).
    pub uninterp spec fn volume(&self) -> int;

    /// No call has changed this disk yet: it is the share as it was
    /// attached. `new_volume` hands its disk out untouched; every call that
    /// may change the disk leaves it touched.
    pub uninterp spec fn untouched(&self) -> bool;
}

/// HOST ASSUMPTIONS about the share as attached, as an axiom. Each is a
/// fact about what the disk held when the share was attached (`as_attached`):
///
/// 1. No host hard link joins a path the guest may write and a visible path
///    it may not (`no_mixed_links`). Pedro's guide states the rule for
///    setting up a share ("A host hard link ... that makes a read-only
///    object reachable inside a writable path also makes that object
///    writable ... Do not place such aliases inside an export").
/// 2. A directory has one path (`directories_have_one_path`): neither Linux
///    nor NTFS lets a directory have hard links, and a share contains no
///    mount point (the host assumption in `lib.spec.rs`), so no bind mount
///    gives one a second path either.
/// 3. The share is a tree (`is_tree`): every object's parent is a directory
///    on the disk, as a file system's names are.
///
/// WHY AN AXIOM. The other host assumptions (`lib.spec.rs`) are about how
/// the host behaves, and are already built into the trusted contracts of
/// `lxutil`'s methods (`lstat` reports the object the disk has only because
/// no other process changed it). These are about what the disk held when
/// the share was attached, before any call: no contract says what is on the
/// disk then, and no code checks it (it would mean walking the whole share).
/// So they can only be stated, and are trusted. `untouched` keeps them to
/// the disk as attached: after a call, the disk is what the calls made it,
/// and the code's checks and `lxutil`'s contracts are what keep the facts,
/// which the proof shows (`VirtioFsGhost::wf`).
///
/// USED once per volume, by the proof that a new file system's ghost state
/// is well formed (`VirtioFs::new_microvm`), with the share's policy.
pub axiom fn attached_disk(
    disk: &VolumeDisk,
    writable: spec_fn(PathView) -> bool,
    visible: spec_fn(PathView) -> bool,
)
    requires
        disk.untouched(),
    ensures
        disk@.as_attached(writable, visible),
;

impl View for VolumeDisk {
    type V = HostDisk;

    uninterp spec fn view(&self) -> HostDisk;
}

impl LxVolume {
    /// The volume's identity, which its disk records.
    pub uninterp spec fn id(&self) -> int;
}

impl LxFile {
    /// The host object that the file is open on.
    pub uninterp spec fn object(&self) -> HostObject;
}

} // verus!
