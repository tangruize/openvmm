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
/// the view has only the running flag, the saved state's view is `()`, and
/// every saved state the keeper returns has the same blob view,
/// `saved_blob()`.
pub struct VmTimeKeeperView {
    /// Whether VM time runs (between `start` and `stop`).
    pub running: bool,
}

impl VmTimeKeeperView {
    /// The view of the keeper's saved-state blob: the same for every
    /// `SavedState`, whose view is `()`.
    pub open spec fn saved_blob() -> SavedStateBlobView {
        SavedStateBlobView::of_view::<SavedState>(())
    }
}

/// vmtime's saved state is all time (`vmtime`, `stop_host_time`), which is
/// out of the spec's scope: its view is empty.
impl View for SavedState {
    type V = ();

    open spec fn view(&self) -> () {
        ()
    }
}

impl View for VmTimeKeeper {
    type V = VmTimeKeeperView;

    /// Trusted: `running` is `self.time.is_started()`. (Uninterpreted:
    /// `VmTimeKeeper` has private fields.)
    uninterp spec fn view(&self) -> VmTimeKeeperView;
}

} // verus!
