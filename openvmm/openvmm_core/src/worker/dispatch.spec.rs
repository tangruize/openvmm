// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

// High-level spec of a loaded VM, for human review.

use super::LoadedVm;
use super::restore::SnapshotRestore;
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
    /// The units whose state a time ABI clock restore changes: the
    /// partition (its TSCs and LAPIC timers are set) and the units that
    /// advance time (they receive `AdvanceTime`).
    pub time_units: Set<Seq<char>>,
    /// The units whose state a pending time ABI clock restore will change:
    /// `time_units` if the VM was loaded for a time ABI restore that has not
    /// yet run (`finish_snapshot_restore`); else empty.
    pub clock_units: Set<Seq<char>>,
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

/// The view of a `SnapshotRestore`.
pub struct SnapshotRestoreView {
    /// Whether a time ABI clock restore is pending (`time_abi` is set; it is
    /// taken by `finish_snapshot_restore`).
    pub clock_restore_pending: bool,
}

impl View for SnapshotRestore {
    type V = SnapshotRestoreView;

    /// Trusted: `clock_restore_pending` is `self.time_abi.is_some()`.
    /// (Uninterpreted: `SnapshotRestore` has private fields, so Verus cannot
    /// see `time_abi`.)
    uninterp spec fn view(&self) -> SnapshotRestoreView;
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
        let time_units = units.advances_time.insert("partition"@);
        LoadedVmView {
            snapshot: SavedStateView { units: units.saved_units(), inventory: units.inventory },
            recorded: units.all_recorded(),
            time_units,
            clock_units: if self.snapshot_restore@.clock_restore_pending {
                time_units
            } else {
                Set::empty()
            },
            runtime: VmRuntime { running: units.running },
        }
    }
}

} // verus!
