// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

// High-level spec of a loaded VM, for human review.

use super::LoadedVm;
use openvmm_defs::worker::spec::SavedStateView;
use vmcore::save_restore::spec::SavedStateBlobView;
use vstd::prelude::*;
use vstd::resource::frac::FracGhost;

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

    /// The snapshot's units are the values of the FracGhost halves `LoadedVm`
    /// holds (see "Views" in `SNAPSHOT-SPEC-DESIGN.md`).
    open(super) spec fn view(&self) -> LoadedVmView {
        LoadedVmView {
            snapshot: SavedStateView {
                units: self.halves@.map_values(|half: FracGhost<SavedStateBlobView>| half@),
                inventory: self.inventory(),
            },
            runtime: VmRuntime { running: self.units_running() },
        }
    }
}

impl LoadedVm {
    /// The names of all state units, in registration order.
    ///
    /// Uninterpreted until `StateUnits` has a view (`state_unit` is not yet
    /// wired for Verus).
    pub uninterp spec fn inventory(&self) -> Seq<Seq<char>>;

    /// Whether the state units are running.
    ///
    /// Uninterpreted until `StateUnits` has a view.
    pub uninterp spec fn units_running(&self) -> bool;
}

} // verus!
