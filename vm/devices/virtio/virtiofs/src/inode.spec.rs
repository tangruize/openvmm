// Spec of `VirtioFsInode`: its view, for human review.

use super::VirtioFsInode;
use crate::spec::VirtioFsInodeView;
use vstd::prelude::*;

verus! {

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
