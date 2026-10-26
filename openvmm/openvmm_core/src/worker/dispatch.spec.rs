// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

// High-level spec of a loaded VM, for human review.

use super::LoadedVm;
use openvmm_defs::worker::spec::SavedStateView;
use vstd::prelude::*;

verus! {

/// The view of a `LoadedVm`.
pub struct LoadedVmView {
    /// The snapshotable state: what `save` captures and `restore` sets.
    /// Meaningful while the state units are stopped.
    pub snapshot: SavedStateView,
    /// The rest, which a snapshot does not capture.
    pub runtime: VmRuntime,
}

/// The part of a loaded VM's state that a snapshot does not capture.
///
/// More fields will be added as the spec grows.
pub struct VmRuntime {
    /// Whether the state units are running (`StateUnits::save` and
    /// `StateUnits::restore` require them stopped).
    pub running: bool,
}

impl View for LoadedVm {
    type V = LoadedVmView;

    /// From the state units' view: the snapshot holds the units whose
    /// FracGhost records a saved state.
    open(super) spec fn view(&self) -> LoadedVmView {
        let units = self.state_units@;
        LoadedVmView {
            snapshot: SavedStateView {
                units: Map::new(
                    units.units.dom().filter(|name: Seq<char>| units.units[name] is Saved),
                    |name: Seq<char>| units.units[name]->Saved_0,
                ),
                inventory: units.inventory,
            },
            runtime: VmRuntime { running: units.running },
        }
    }
}

impl LoadedVm {
    /// Every state unit's state is recorded: `LoadedVm` holds a half for each
    /// unit in the inventory, and none records `Unknown` (each unit has been
    /// stopped or restored since it was created).
    pub open(super) spec fn snapshot_recorded(&self) -> bool {
        let units = self.state_units@;
        forall|i: int|
            0 <= i < units.inventory.len() ==> #[trigger] units.units.contains_key(units.inventory[i])
                && !(units.units[units.inventory[i]] is Unknown)
    }
}

} // verus!
