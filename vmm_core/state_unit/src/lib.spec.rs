// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

// High-level spec of state units, for human review.

use vmcore::save_restore::spec::SavedStateBlobView;
use vstd::prelude::*;

verus! {

/// What a state unit's FracGhost records: the unit's snapshotable state,
/// that is, what its save returns.
pub enum UnitSnapshot {
    /// Not yet recorded: the unit has been neither stopped nor restored since
    /// it was created.
    Unknown,
    /// The unit has no saved state (its save returns `None`).
    NoSavedState,
    /// The unit's saved state.
    Saved(SavedStateBlobView),
}

} // verus!
