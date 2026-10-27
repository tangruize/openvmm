// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

// The identity MSR handler's snapshotable state and the time-ABI unit's
// share of its FracGhost, for human review.

use super::TimeAbiMsrs;
use super::TimeAbiSavedState;
use vmcore::device_state::spec::DeviceGhostState;
use vmcore::save_restore::spec::SavedStateBlobView;
use vmcore::save_restore::spec::UnitSnapshot;
use vstd::atomic_ghost::AtomicInvariantPredicate;
use vstd::prelude::*;
use vstd::resource::Loc;

verus! {

/// The invariant of `TimeAbiMsrs::tsc_invariant_control`'s atomic, whose
/// key `k` is the id of the time-ABI unit's FracGhost: the share kept with
/// the value follows the rule (`DeviceGhostState::wf`), so, once known, it
/// records the saved state the handler's `save` returns for that value.
pub struct TscControlPred;

impl AtomicInvariantPredicate<Loc, u64, DeviceGhostState> for TscControlPred {
    open spec fn atomic_inv(k: Loc, v: u64, g: DeviceGhostState) -> bool {
        g.wf(k, UnitSnapshot::Saved(SavedStateBlobView::of(TimeAbiSavedState { tsc_invariant_control: v })))
    }
}

impl TimeAbiMsrs {
    /// The id of the time-ABI unit's FracGhost, whose share is kept with the
    /// MSR value.
    pub closed spec fn key(&self) -> Loc {
        self.tsc_invariant_control.constant()
    }

    /// Well formed: the MSR value's atomic is set up.
    pub closed spec fn wf(&self) -> bool {
        self.tsc_invariant_control.well_formed()
    }
}

} // verus!
