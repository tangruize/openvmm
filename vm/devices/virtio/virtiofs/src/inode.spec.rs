// Spec of `VirtioFsInode`: its view, for human review.

use super::InodePath;
use super::VirtioFsInode;
use crate::spec::VirtioFsInodeView;
use vstd::prelude::*;
use vstd::resource::Loc;
use vstd::rwlock::RwLockPredicate;

verus! {

/// The rule of an inode's path lock: what the lock's value always is, given
/// when the lock is created (`RwLock::new(value, Ghost(rule))`), and never
/// changed. For the root: the path is empty, and there is no ghost piece
/// (its path never changes, so it needs none). For any other inode: the
/// ghost piece is this inode's entry of the ghost `paths` map (the map with
/// id `map_id`, at the inode's key `key`), and its value is the path in the
/// lock; so the path in the lock always equals `paths[key]` in the map's
/// `Auth`, which the request holds.
// Only Verus reads the rule's fields.
#[cfg_attr(not(verus_keep_ghost), allow(dead_code))]
pub ghost struct PathRule {
    /// The inode's ghost key: its entry in the `paths` and `aliases` maps.
    pub key: int,
    /// The id of the `paths` map.
    pub map_id: Loc,
    /// Whether the inode is the root.
    pub is_root: bool,
}

impl RwLockPredicate<InodePath> for PathRule {
    closed spec fn inv(self, v: InodePath) -> bool {
        if self.is_root {
            &&& v.path@ == Seq::<Seq<u8>>::empty()
            &&& v.piece@ is None
        } else {
            match v.piece@ {
                Some(piece) => {
                    &&& piece.id() == self.map_id
                    &&& piece.key() == self.key
                    &&& piece.value() == v.path@
                },
                None => false,
            }
        }
    }
}

/// The inode's plain fields, which never change (its path and names are
/// behind locks, in `VirtioFsGhostView`'s `paths` and `aliases`).
impl View for VirtioFsInode {
    type V = VirtioFsInodeView;

    closed spec fn view(&self) -> VirtioFsInodeView {
        VirtioFsInodeView {
            object: (self.volume.id, self.inode_nr),
            kind: self.file_type,
            readonly: self.volume.readonly,
            policy: self.volume.policy@,
        }
    }
}

} // verus!
