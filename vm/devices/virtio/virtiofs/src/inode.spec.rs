// Spec of `VirtioFsInode`: its view, for human review.

use super::InodeAliases;
use super::InodePath;
use super::VirtioFsInode;
use super::VirtioFsVolume;
use crate::spec::VirtioFsInodeView;
#[cfg(verus_keep_ghost)]
use crate::spec::VirtioFsShareView;
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
///
/// SPEC: the core of the rule is part of the spec: the piece is this
/// inode's entry of the `paths` map (`id`, `key`), and its value is the real
/// path (`value`). It is what makes the contracts' paths the paths the code
/// hands to the host.
/// TODO (proof agent): the rest is the proof's to settle (e.g. how the key
/// is chosen); it may add facts or fields, but must keep the core.
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

/// The rule of an inode's aliases lock: like `PathRule`, for the `aliases`
/// map. For the root: its only name is the empty path, and there is no
/// ghost piece. For any other inode: the piece is this inode's entry of the
/// `aliases` map (the same key as its path lock's), and its value is the set
/// of the real names.
///
/// SPEC: the core, as for `PathRule` (`id`, `key`, `value`).
/// TODO (proof agent): the rest (e.g. that the key is the path lock's: a
/// fact of the inode, or of the inode table's rule); it may add facts or
/// fields, but must keep the core.
// Only Verus reads the rule's fields.
#[cfg_attr(not(verus_keep_ghost), allow(dead_code))]
pub ghost struct AliasesRule {
    /// The inode's ghost key: its entry in the `aliases` map.
    pub key: int,
    /// The id of the `aliases` map.
    pub map_id: Loc,
    /// Whether the inode is the root.
    pub is_root: bool,
}

impl RwLockPredicate<InodeAliases> for AliasesRule {
    closed spec fn inv(self, v: InodeAliases) -> bool {
        let names = v.aliases@.map(|p: std::path::PathBuf| p@);
        if self.is_root {
            &&& names == set![Seq::<Seq<u8>>::empty()]
            &&& v.piece@ is None
        } else {
            match v.piece@ {
                Some(piece) => {
                    &&& piece.id() == self.map_id
                    &&& piece.key() == self.key
                    &&& piece.value() == names
                },
                None => false,
            }
        }
    }
}

impl VirtioFsVolume {
    /// The volume as an aggregate's share: its id, read-only flag and
    /// policy, and its root's path (`root_entry`: the exposed file's name, or
    /// the empty path).
    pub(crate) closed spec fn share_view(&self) -> VirtioFsShareView {
        VirtioFsShareView {
            volume: self.id,
            readonly: self.readonly,
            policy: self.policy@,
            root: match self.exposed_file {
                Some(file) => file@,
                None => Seq::<Seq<u8>>::empty(),
            },
            exposed: match self.exposed_file {
                Some(file) => Some(file@),
                None => None,
            },
        }
    }
}

/// The inode's plain fields, which never change (its path and names are
/// behind locks, in `VirtioFsGhostView`'s `paths` and `aliases`).
impl View for VirtioFsInode {
    type V = VirtioFsInodeView;

    closed spec fn view(&self) -> VirtioFsInodeView {
        VirtioFsInodeView {
            volume: self.volume.id,
            kind: self.file_type,
            readonly: self.volume.readonly,
            policy: self.volume.policy@,
            exposed: match self.volume.exposed_file {
                Some(file) => Some(file@),
                None => None,
            },
        }
    }
}

} // verus!
