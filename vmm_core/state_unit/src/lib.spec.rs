// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

// High-level spec of state units, for human review.

use super::SavedStateUnit;
use super::quiesce::QuiesceForSaveError;
use super::StateUnits;
use vmcore::save_restore::spec::SavedStateBlobView;
pub use vmcore::save_restore::spec::UnitSnapshot;
use vstd::prelude::*;

verus! {

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
    /// The names of the units that advance guest-visible time
    /// (`UnitBuilder::advances_time`; they receive `AdvanceTime`).
    pub advances_time: Set<Seq<char>>,
    /// Whether the units are running.
    pub running: bool,
}

impl StateUnitsView {
    /// The inventory lists each unit once, and the units that advance time
    /// are among them.
    pub open spec fn inv(self) -> bool {
        &&& self.inventory.no_duplicates()
        &&& self.inventory.to_set() == self.units.dom()
        &&& self.advances_time <= self.units.dom()
    }

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

impl StateUnits {
    /// Well formed: the view's invariant holds; `StateUnits` holds halves of
    /// collected units' FracGhosts only, each belonging to that unit's
    /// channel, and while stopped it holds one for every collected unit;
    /// collected units have distinct names; and its half of the record of
    /// the collected units, shared with its registry, records them.
    ///
    /// (While running, a unit holds its whole FracGhost, so `StateUnits`
    /// holds no half for it; it holds halves for units added while running,
    /// which are stopped until `start_stopped_units`.)
    pub closed spec fn wf(&self) -> bool {
        &&& self@.inv()
        &&& self.units_ghost@.halves.dom() <= self.units_ghost@.registry.dom()
        &&& !self.running ==> self.units_ghost@.halves.dom() == self.units_ghost@.registry.dom()
        &&& forall|id: u64| #[trigger] self.units_ghost@.halves.contains_key(id)
            ==> self.units_ghost@.halves[id].valid(self.units_ghost@.registry[id].1, 0.5real)
        // `StateUnits`' half of the shared record of the collected units
        // belongs to its registry, and records the collected units.
        &&& self.units_ghost@.collected.valid((*self.inner).key(), 0.5real)
        &&& self.units_ghost@.collected@ == self.units_ghost@.registry
        &&& forall|a: u64, b: u64|
            #[trigger] self.units_ghost@.registry.contains_key(a)
                && #[trigger] self.units_ghost@.registry.contains_key(b) && a != b
                ==> self.units_ghost@.registry[a].0 != self.units_ghost@.registry[b].0
    }
}

impl View for StateUnits {
    type V = StateUnitsView;

    /// `units`: for each collected unit, by name, what the half held for it
    /// records, or `Unknown` if none is held (the unit is running).
    /// (Collected units have distinct names; see `wf`.)
    closed spec fn view(&self) -> StateUnitsView {
        StateUnitsView {
            units: Map::new(
                self.units_ghost@.registry.values().map(
                    |entry: (Seq<char>, vstd::resource::Loc, bool)| entry.0,
                ),
                |name: Seq<char>| {
                    let id = choose|id: u64| #[trigger] self.units_ghost@.registry.contains_key(id)
                        && self.units_ghost@.registry[id].0 == name;
                    if self.units_ghost@.halves.contains_key(id) {
                        self.units_ghost@.halves[id]@
                    } else {
                        UnitSnapshot::Unknown
                    }
                },
            ),
            // Unit ids increase with registration, so id order is
            // registration order.
            inventory: self.units_ghost@.registry.dom().to_sorted_seq(|a: u64, b: u64| a <= b).map_values(
                |id: u64| self.units_ghost@.registry[id].0,
            ),
            advances_time: self.units_ghost@.registry.values().filter(
                |entry: (Seq<char>, vstd::resource::Loc, bool)| entry.2,
            ).map(|entry: (Seq<char>, vstd::resource::Loc, bool)| entry.0),
            running: self.running,
        }
    }
}

/// The view of a `QuiesceForSaveError`.
pub struct QuiesceForSaveErrorView {
    /// Whether some unit may have partly changed its state or ownership:
    /// the VM cannot be resumed and must be terminated.
    pub uncertain: bool,
}

impl View for QuiesceForSaveError {
    type V = QuiesceForSaveErrorView;

    /// Trusted: `uncertain` is the error's `uncertain` field (the type is
    /// opaque to Verus; `has_uncertain_state` returns the field).
    uninterp spec fn view(&self) -> QuiesceForSaveErrorView;
}

} // verus!

