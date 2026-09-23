// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

// Human-owned open specification vocabulary for `dispatch.rs` snapshot restore.
//
// This module contains no executable restore implementation.
//
// Snapshot state has a fixed meaning taken from real data: it is exactly what
// `LoadedVm::save` produces, a decoded `SavedState` (state-unit inventory plus
// one saved-state blob per stateful unit). The same View is used for the
// restore input and for the live VM, and the live VM's View is anchored by the
// `LoadedVm::save` contract, whose output is real data. No part of the snapshot
// View is an invented placeholder, so no later proof definition can make the
// restore property vacuous.
//
// Which fields of a component are snapshot state is therefore decided by what
// the component's own `save` serializes; configuration and host-bound runtime
// objects that `save` does not serialize are not snapshot state.

use openvmm_defs::worker::SavedState;
use state_unit::SavedStateUnit;
use std::time::Duration;
use vmcore::save_restore::SavedStateBlob;
#[cfg(verus_keep_ghost)]
use vmcore::vmtime::duration_observation::elapsed_nanoseconds;
use vstd::prelude::*;

verus! {

// The save/restore identity of a state unit.
pub type UnitName = Seq<char>;

// Snapshot-owned VM state in exactly the form `LoadedVm::save` produces.
pub struct VmSnapshotView {
    // Complete ordered state-unit inventory, including stateless units.
    pub inventory: Seq<UnitName>,
    // Saved state of every unit that has mutable state. Blobs are compared as
    // real values; their meaning is owned by each unit's saved-state schema.
    pub units: Map<UnitName, SavedStateBlob>,
}

pub struct RestoreRequestView {
    pub snapshot: VmSnapshotView,
    // No two saved units share a name (the saved form is a list).
    pub unit_names_unique: bool,
    pub has_time_adjustment: bool,
    pub downtime_ns: nat,
}

// The snapshot-owned state of a live `LoadedVm`: what `save` would return.
pub struct LoadedVmView {
    pub snapshot: VmSnapshotView,
}

impl VmSnapshotView {
    // The decoded form of a real `SavedState`.
    pub open spec fn of_saved_state(saved: &SavedState) -> VmSnapshotView {
        VmSnapshotView {
            inventory: saved.inventory@.map_values(|name: String| name@),
            units: Map::new(
                saved.units@.map_values(|unit: SavedStateUnit| unit.name@).to_set(),
                |name: UnitName|
                    saved.units@[choose|i: int| #![trigger saved.units@[i]]
                        0 <= i < saved.units@.len() && saved.units@[i].name@ == name].state,
            ),
        }
    }

    // Saved units replace the corresponding live unit state; units without
    // saved state keep theirs. Restore does not change the live inventory.
    pub open spec fn overlay(self, saved: VmSnapshotView) -> VmSnapshotView {
        VmSnapshotView {
            inventory: self.inventory,
            units: Map::new(
                self.units.dom().union(saved.units.dom()),
                |name: UnitName|
                    if saved.units.contains_key(name) {
                        saved.units[name]
                    } else {
                        self.units[name]
                    },
            ),
        }
    }

    // Downtime compensation: every unit advances its own guest-visible clocks
    // by the same downtime, as observed by its next save. A zero downtime
    // changes nothing.
    pub open spec fn after_downtime(self, downtime_ns: nat) -> VmSnapshotView {
        if downtime_ns == 0 {
            self
        } else {
            VmSnapshotView {
                inventory: self.inventory,
                units: Map::new(
                    self.units.dom(),
                    |name: UnitName|
                        super::restore_proof::unit_after_downtime(
                            name,
                            self.units[name],
                            downtime_ns,
                        ),
                ),
            }
        }
    }

    // The single definition of the snapshot state after a successful restore
    // of `request` onto the live snapshot state `self`.
    pub open spec fn restore_from(self, request: RestoreRequestView) -> VmSnapshotView {
        request.compensate(self.overlay(request.snapshot))
    }

    // Every saved unit holds exactly its (compensated) saved state. Units
    // without saved state are not constrained.
    pub open spec fn installs(self, request: RestoreRequestView) -> bool {
        let expected = request.compensate(request.snapshot);
        forall|name: UnitName|
            #[trigger] request.snapshot.units.contains_key(name) ==> self.units.contains_key(name)
                && self.units[name] == expected.units[name]
    }
}

impl RestoreRequestView {
    // The decoded form of the real restore inputs.
    pub open spec fn decode(
        saved: &SavedState,
        restore_time: &Option<(Duration, u64, Option<u64>)>,
    ) -> RestoreRequestView {
        RestoreRequestView {
            snapshot: VmSnapshotView::of_saved_state(saved),
            unit_names_unique: forall|i: int, j: int| #![trigger saved.units@[i], saved.units@[j]]
                0 <= i < j < saved.units@.len() ==> saved.units@[i].name@ != saved.units@[j].name@,
            has_time_adjustment: restore_time.is_some(),
            downtime_ns: match restore_time {
                Some(time) => elapsed_nanoseconds(&time.0),
                None => 0,
            },
        }
    }

    pub open spec fn compensate(self, snapshot: VmSnapshotView) -> VmSnapshotView {
        if self.has_time_adjustment {
            snapshot.after_downtime(self.downtime_ns)
        } else {
            snapshot
        }
    }

    // The checks restore performs before accepting a snapshot: unique saved
    // unit names that are all registered, and a matching inventory unless the
    // snapshot predates inventories.
    pub open spec fn is_accepted_by(self, live: VmSnapshotView) -> bool {
        self.unit_names_unique
        && (forall|name: UnitName|
            #[trigger] self.snapshot.units.contains_key(name) ==> live.inventory.contains(name))
        && (self.snapshot.inventory.len() == 0 || self.snapshot.inventory == live.inventory)
    }
}

impl LoadedVmView {
    pub open spec fn snapshot_restore_success(
        self,
        request: RestoreRequestView,
        restored: LoadedVmView,
    ) -> bool {
        request.is_accepted_by(self.snapshot)
        && restored.snapshot == self.snapshot.restore_from(request)
    }
}

} // verus!
