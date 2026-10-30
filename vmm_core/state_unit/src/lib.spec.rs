// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

// High-level spec of state units, for human review.

use super::SavedStateUnit;
use super::StateUnits;
use super::quiesce::QuiesceForSaveError;
use vmcore::save_restore::spec::SavedStateBlobView;
pub use vmcore::save_restore::spec::UnitSnapshot;
use vstd::prelude::*;
use vstd::resource::Loc;
use vstd::resource::frac::FracGhost;

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
    /// The collected units whose half `StateUnits` does not hold, by unit
    /// id, with their entries (name, the id of the unit's FracGhost, whether
    /// it advances time). While running: the running units. While stopped:
    /// none, unless the halves are lent out (`StateUnits::lend_halves`).
    pub missing_halves: Map<u64, (Seq<char>, Loc, bool)>,
}

impl StateUnitsView {
    /// `StateUnits` holds every collected unit's half (`missing_halves` is
    /// empty), as `reset`, `restore`, `advance_time`,
    /// `resume_after_failed_save`, and `lend_halves` need.
    pub open spec fn holds_halves(self) -> bool {
        self.missing_halves.dom().is_empty()
    }

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

/// Spec functions of a list of saved units (the view of the `Vec` that
/// `StateUnits::save` returns, `StateUnits::restore` takes, and
/// `SavedState` holds).
pub trait SavedStateUnits {
    /// The saved states in the list, by unit name. If two entries have the
    /// same name, the map holds one of them.
    spec fn by_name(&self) -> Map<Seq<char>, SavedStateBlobView>;

    /// The list's entries have distinct names.
    spec fn distinct_names(&self) -> bool;
}

impl SavedStateUnits for Seq<SavedStateUnit> {
    open spec fn by_name(&self) -> Map<Seq<char>, SavedStateBlobView> {
        Map::new(
            self.map_values(|unit: SavedStateUnit| unit.name@).to_set(),
            |name: Seq<char>|
                self[choose|i: int| 0 <= i < self.len() && #[trigger] self[i].name@ == name].state@,
        )
    }

    open spec fn distinct_names(&self) -> bool {
        forall|i: int, j: int|
            0 <= i < self.len() && 0 <= j < self.len() && i != j ==> #[trigger] self[i].name@
                != #[trigger] self[j].name@
    }
}

impl StateUnits {
    /// Well formed: the view's invariant holds; `StateUnits` holds halves of
    /// collected units' FracGhosts only, each belonging to that unit's
    /// channel; collected units have distinct names; and its half of the
    /// record of the collected units, shared with its registry, records them.
    ///
    /// Which halves it holds is not part of `wf` but of the view
    /// (`missing_halves`): the functions that need them all require
    /// `holds_halves()`, and those that collect them (`stop`,
    /// `quiesce_for_save`) ensure it. (While running, a unit holds its whole
    /// FracGhost, so `StateUnits` holds no half for it; it holds halves for
    /// units added while running, which are stopped until
    /// `start_stopped_units`. While stopped, it holds them all, unless they
    /// are lent out: `lend_halves`, `return_halves`.)
    pub closed spec fn wf(&self) -> bool {
        &&& self@.inv()
        &&& self.units_ghost@.halves.dom() <= self.units_ghost@.registry.dom()
        &&& forall|id: u64| #[trigger] self.units_ghost@.halves.contains_key(id)
            ==> self.units_ghost@.halves[id].valid(self.units_ghost@.registry[id].1, 0.5real)
        // `StateUnits`' half of the shared record of the collected units
        // belongs to its registry, and records the collected units.
        &&& self.units_ghost@.collected.valid((*self.inner).param(), 0.5real)
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
                    |entry: (Seq<char>, Loc, bool)| entry.0,
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
                |entry: (Seq<char>, Loc, bool)| entry.2,
            ).map(|entry: (Seq<char>, Loc, bool)| entry.0),
            running: self.running,
            missing_halves: self.units_ghost@.registry.remove_keys(self.units_ghost@.halves.dom()),
        }
    }
}

/// The halves of a stopped `StateUnits`, lent out (`StateUnits::lend_halves`)
/// to whoever must know that no unit changes its state until they come back
/// (`StateUnits::return_halves`): the VM worker lends them to the VM
/// controller with a snapshot (`VmRpc`'s protocol, `openvmm_defs`).
pub tracked struct LentHalves {
    /// The lent units, by unit id: name, the id of the unit's FracGhost,
    /// whether it advances time (their entries in `StateUnits`).
    pub ghost units: Map<u64, (Seq<char>, Loc, bool)>,
    /// The half of each lent unit, by unit id.
    pub tracked halves: Map<u64, FracGhost<UnitSnapshot>>,
}

impl LentHalves {
    /// Well formed: one half per lent unit, each belonging to that unit's
    /// FracGhost; the lent units have distinct names.
    pub open spec fn wf(&self) -> bool {
        &&& self.halves.dom() == self.units.dom()
        &&& forall|id: u64| #[trigger] self.halves.contains_key(id)
            ==> self.halves[id].valid(self.units[id].1, 0.5real)
        &&& forall|a: u64, b: u64|
            #[trigger] self.units.contains_key(a) && #[trigger] self.units.contains_key(b) && a != b
                ==> self.units[a].0 != self.units[b].0
    }

    /// What each lent unit's half records, by unit name (as
    /// `StateUnitsView::units`).
    pub open spec fn records(&self) -> Map<Seq<char>, UnitSnapshot> {
        Map::new(
            self.units.values().map(|entry: (Seq<char>, Loc, bool)| entry.0),
            |name: Seq<char>| {
                let id = choose|id: u64| #[trigger] self.units.contains_key(id)
                    && self.units[id].0 == name;
                self.halves[id]@
            },
        )
    }

    /// The lent units that record a saved state, with that state, by name
    /// (as `StateUnitsView::saved_units`).
    pub open spec fn saved_units(&self) -> Map<Seq<char>, SavedStateBlobView> {
        Map::new(
            self.records().dom().filter(|name: Seq<char>| self.records()[name] is Saved),
            |name: Seq<char>| self.records()[name]->Saved_0,
        )
    }
}

/// The view of a `QuiesceForSaveError`.
pub struct QuiesceForSaveErrorView {
    /// Whether some unit may have partly changed its state or ownership:
    /// the VM cannot be resumed and must be terminated.
    pub uncertain: bool,
}

// `View for QuiesceForSaveError` is in `quiesce.rs`, next to the type (its
// fields are private): `uncertain` is the error's `uncertain` field.

/// What `StateUnit`'s contract needs to know about a unit (a supertrait of
/// `StateUnit` in Verus builds; empty in normal builds). The contract itself
/// is on `StateUnit`'s methods, in `lib.rs`: the request protocol ("Request
/// protocol" in `SNAPSHOT-SPEC-DESIGN.md`), with the FracGhost halves passed
/// through each method as a ghost input.
///
/// Each unit keeps its own share of its FracGhost where its state is (a
/// field; inside its device's `CloseableMutex`; behind interior
/// mutability), and its `wf()` ties that share to its state: the share,
/// unless still `Unknown`, records what the unit's save returns; the whole
/// while running, one half while stopped.
pub trait StateUnitSpec {
    /// The unit's invariant.
    spec fn wf(&self) -> bool;

    /// The id of the unit's FracGhost (its channel's constant).
    spec fn id(&self) -> Loc;

    /// Whether the unit runs (between `start` and `stop`).
    spec fn running(&self) -> bool;
}

} // verus!
