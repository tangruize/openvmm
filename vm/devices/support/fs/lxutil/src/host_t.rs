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

/// One object on the host disk: its inode number and its file type (the
/// `S_IFMT` bits).
pub struct HostObject {
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

    /// Nothing lies strictly below path `p` (an empty directory, or not a
    /// directory).
    pub open spec fn nothing_below(self, p: PathView) -> bool {
        forall|x: PathView| #[trigger] self.objects.contains_key(x) && p.is_prefix_of(x) ==> x == p
    }

    /// `after` is this disk, or this disk with one new object at path `p`,
    /// where there was none: what a call that may have made `p` before it
    /// failed leaves.
    pub open spec fn unchanged_or_made(self, p: PathView, after: HostDisk) -> bool {
        ||| after == self
        ||| !self.objects.contains_key(p) && exists|o: HostObject| after.objects == #[trigger] self.objects.insert(p, o)
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
}

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
