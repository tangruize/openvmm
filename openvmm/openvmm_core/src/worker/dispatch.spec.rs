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
    /// Whether every state unit's snapshotable state is known (each has been
    /// stopped or restored since it was created); `snapshot` is complete
    /// only then.
    pub recorded: bool,
    /// The rest, which a snapshot does not capture.
    pub runtime: VmRuntime,
}

impl LoadedVmView {
    /// The snapshot's invariant holds.
    pub open spec fn inv(self) -> bool {
        self.snapshot.inv()
    }
}

/// The part of a loaded VM's state that a snapshot does not capture.
///
/// More fields will be added as the spec grows.
pub struct VmRuntime {
    /// Whether the state units are running (`StateUnits::save` and
    /// `StateUnits::restore` require them stopped).
    pub running: bool,
}

impl LoadedVm {
    /// Well formed: the view's invariant holds, and the state units are well
    /// formed.
    pub closed spec fn wf(&self) -> bool {
        &&& self@.inv()
        &&& self.state_units.wf()
    }
}

impl View for LoadedVm {
    type V = LoadedVmView;

    /// From the state units' view: the snapshot holds the units whose
    /// FracGhost records a saved state.
    closed spec fn view(&self) -> LoadedVmView {
        let units = self.state_units@;
        LoadedVmView {
            snapshot: SavedStateView { units: units.saved_units(), inventory: units.inventory },
            recorded: units.all_recorded(),
            runtime: VmRuntime { running: units.running },
        }
    }
}

} // verus!
