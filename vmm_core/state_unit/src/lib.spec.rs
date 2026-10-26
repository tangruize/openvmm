// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

// High-level spec of state units, for human review.

use super::SavedStateUnit;
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
///
/// It covers the units `StateUnits` has collected (taken the FracGhost half
/// of). Collection happens before the VM is first used (`make_ids`), so these
/// are all registered units; units registered later must be collected before
/// the next `stop`, `save`, or `restore` (see `SNAPSHOT-SPEC-TRUSTED.md`).
pub struct StateUnitsView {
    /// What each unit's FracGhost records, by unit name.
    pub units: Map<Seq<char>, UnitSnapshot>,
    /// The names of the units, in registration order.
    pub inventory: Seq<Seq<char>>,
    /// Whether the units are running.
    pub running: bool,
}

impl StateUnitsView {
    /// No unit records `Unknown`: each has been stopped or restored since it
    /// was created.
    pub open spec fn all_recorded(self) -> bool {
        forall|name: Seq<char>| #[trigger] self.units.contains_key(name)
            ==> !(self.units[name] is Unknown)
    }

    /// The units that record a saved state, with that state, by name.
    pub open spec fn saved_units(self) -> Map<Seq<char>, SavedStateBlobView> {
        Map::new(
            self.units.dom().filter(|name: Seq<char>| self.units[name] is Saved),
            |name: Seq<char>| self.units[name]->Saved_0,
        )
    }
}

impl SavedStateUnit {
    /// The saved states in `units`, by unit name. If two entries have the
    /// same name, the map holds one of them.
    pub open spec fn map_of(units: Seq<SavedStateUnit>) -> Map<Seq<char>, SavedStateBlobView> {
        Map::new(
            units.map_values(|unit: SavedStateUnit| unit.name@).to_set(),
            |name: Seq<char>|
                units[choose|i: int| 0 <= i < units.len() && #[trigger] units[i].name@ == name].state@,
        )
    }

    /// The entries of `units` have distinct names.
    pub open spec fn distinct_names(units: Seq<SavedStateUnit>) -> bool {
        forall|i: int, j: int|
            0 <= i < units.len() && 0 <= j < units.len() && i != j ==> #[trigger] units[i].name@
                != #[trigger] units[j].name@
    }
}

impl View for StateUnits {
    type V = StateUnitsView;

    /// `units`: for each collected unit, by name, what the half held for it
    /// records. (Collected units have distinct names; see `wf`.)
    closed spec fn view(&self) -> StateUnitsView {
        StateUnitsView {
            units: Map::new(
                self.units_ghost@.registry.values().map(|entry: (Seq<char>, vstd::resource::Loc)| entry.0),
                |name: Seq<char>|
                    self.units_ghost@.halves[choose|id: u64| #[trigger] self.units_ghost@.registry.contains_key(id)
                        && self.units_ghost@.registry[id].0 == name]@,
            ),
            // Unit ids increase with registration, so id order is
            // registration order.
            inventory: self.units_ghost@.registry.dom().to_sorted_seq(|a: u64, b: u64| a <= b).map_values(
                |id: u64| self.units_ghost@.registry[id].0,
            ),
            running: self.running,
        }
    }
}

} // verus!
