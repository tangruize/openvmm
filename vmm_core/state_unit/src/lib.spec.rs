// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

// High-level spec of state units, for human review.

use super::StateUnits;
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

/// The view of a `StateUnits`.
pub struct StateUnitsView {
    /// What each unit's FracGhost records, by unit name, for the units whose
    /// half `StateUnits` holds.
    pub units: Map<Seq<char>, UnitSnapshot>,
    /// The names of all state units, in registration order.
    pub inventory: Seq<Seq<char>>,
    /// Whether the units are running.
    pub running: bool,
}

impl View for StateUnits {
    type V = StateUnitsView;

    closed spec fn view(&self) -> StateUnitsView {
        StateUnitsView {
            units: self.halves@.map_values(|half: vstd::resource::frac::FracGhost<UnitSnapshot>| half@),
            inventory: self.spec_inventory(),
            running: self.running,
        }
    }
}

impl StateUnits {
    /// The names of all state units, in registration order.
    ///
    /// Uninterpreted for now: the names are behind the registry's mutex.
    pub uninterp spec fn spec_inventory(&self) -> Seq<Seq<char>>;
}

} // verus!
