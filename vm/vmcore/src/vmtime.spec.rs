// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

// The spec of the VM time keeper's snapshotable state, for human review.

use super::SavedState;
use super::VmTimeKeeper;
use crate::save_restore::spec::SavedStateBlobView;
use vstd::prelude::*;

verus! {

/// The view of a `VmTimeKeeper`.
///
/// Its saved state (`SavedState`: the VM time it stopped at and the host
/// time it stopped at) is all time, which is out of the spec's scope, so
/// the view has only the running flag, and every saved state the keeper
/// returns has the same blob view, `saved_blob()`.
pub struct VmTimeKeeperView {
    /// Whether VM time runs (between `start` and `stop`).
    pub running: bool,
}

impl VmTimeKeeperView {
    /// The view of the keeper's saved-state blob: the same for every
    /// `SavedState`, since a blob's view forgets time
    /// (`SavedState::axiom_all_time`, `vmtime_t.rs`).
    pub open spec fn saved_blob() -> SavedStateBlobView {
        SavedStateBlobView::of(arbitrary::<SavedState>())
    }
}

impl View for VmTimeKeeper {
    type V = VmTimeKeeperView;

    /// Trusted: `running` is `self.time.is_started()`. (Uninterpreted:
    /// `VmTimeKeeper` has private fields.)
    uninterp spec fn view(&self) -> VmTimeKeeperView;
}

} // verus!
