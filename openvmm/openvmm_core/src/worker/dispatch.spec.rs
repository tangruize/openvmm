// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

// High-level spec of a loaded VM, for human review.

use super::LoadedVm;
use super::microvm::SnapshotBoundary;
use super::restore::SnapshotRestore;
use openvmm_defs::worker::spec::SavedStateView;
use vstd::prelude::*;
use vstd::resource::Loc;

verus! {

/// The view of a `LoadedVm`.
pub struct LoadedVmView {
    /// The snapshotable state: what `save` captures and `restore` sets.
    /// Meaningful while the state units are stopped.
    pub snapshot: SavedStateView,
    /// The units whose save fails, by name: their state is one they refuse
    /// to save (e.g. the partition unit after a partial restore). `save`
    /// fails if there is one; `snapshot` still holds their state.
    pub save_fails: Set<Seq<char>>,
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
    /// Whether a guest-requested snapshot boundary is active: the vCPUs are
    /// held stopped at an I/O boundary and host input is gated, until the
    /// boundary is released.
    pub snapshot_boundary: bool,
    /// The state units whose half the state units do not hold
    /// (`StateUnitsView::missing_halves`): while running, the running
    /// units; while stopped, none, unless they are lent to the controller
    /// with a snapshot (`VmRpc`'s protocol).
    pub missing_halves: Map<u64, (Seq<char>, Loc, bool)>,
    /// Whether the VM has the microVM machine profile (a guest-requested
    /// snapshot needs it).
    pub microvm: bool,
    /// Whether the VM boots Linux directly with an MP table, so that a
    /// snapshot can record its effective command line (a guest-requested
    /// snapshot needs it).
    pub linux_mptable: bool,
}

impl VmRuntime {
    /// The state units hold every unit's half (none missing).
    pub open spec fn holds_halves(self) -> bool {
        self.missing_halves.dom().is_empty()
    }
}

/// The view of a `SnapshotRestore`.
pub struct SnapshotRestoreView {
    /// Whether a time ABI clock restore is pending (`time_abi` is set; it is
    /// taken by `finish_snapshot_restore`).
    pub clock_restore_pending: bool,
}

/// The view of a `SnapshotBoundary`.
pub struct SnapshotBoundaryView {
    /// Whether a boundary is active.
    pub active: bool,
}

// The views of `SnapshotBoundary` and `SnapshotRestore` are defined next to
// the types, from their fields (`dispatch/microvm.rs`, `dispatch/restore.rs`).

impl LoadedVm {
    /// Well formed: the view's invariant holds, the state units are well
    /// formed, the VM's running flag is the state units' (the view's
    /// `runtime.running`), and an active snapshot boundary and a restore's
    /// guard hold this VM's own partition unit's VPs stopped (their
    /// `StopGuard`s send to `inner.partition_unit`). Every function that changes one changes the other
    /// before it returns (`resume`, `pause`, `quiesce_and_save`,
    /// `resume_after_failed_snapshot`, and the error paths that stop the
    /// units).
    pub closed spec fn wf(&self) -> bool {
        &&& self@.inv()
        &&& self.state_units.wf()
        &&& self.running == self.state_units@.running
        &&& self.snapshot_boundary.wf(self.inner.partition_unit.id())
        &&& self.snapshot_restore.wf(self.inner.partition_unit.id())
    }
}

impl LoadedVm {
    /// Whether the VM's load mode is Linux direct boot with an MP table
    /// (`LoadMode::Linux` with `boot_mode: LinuxDirectBootMode::MpTable`).
    /// PROOF TODO: define it from `inner.load_mode`'s fields, once
    /// `LoadMode`'s field types are declared to Verus (it is declared by
    /// name only, fields hidden). Our code, not trusted.
    pub uninterp spec fn boots_linux_mptable(&self) -> bool;

    /// The time ABI capture failed (`capture_time_abi`: no time ABI, the
    /// LAPIC timers' check, the guest's declared rates, the hypervisor's or
    /// the host's clock). OUT OF SCOPE (guest time): established only by
    /// `capture_time_abi`'s contract, on its errors.
    pub uninterp spec fn time_capture_failed() -> bool;
}

impl View for LoadedVm {
    type V = LoadedVmView;

    /// From the state units' view: the snapshot holds the units whose
    /// FracGhost records a saved state, or a state whose save fails.
    closed spec fn view(&self) -> LoadedVmView {
        let units = self.state_units@;
        LoadedVmView {
            snapshot: SavedStateView { units: units.state_blobs(), inventory: units.inventory },
            save_fails: units.save_fails(),
            recorded: units.all_recorded(),
            runtime: VmRuntime {
                running: units.running,
                snapshot_boundary: self.snapshot_boundary@.active,
                missing_halves: units.missing_halves,
                microvm: self.inner.machine_profile
                    == openvmm_defs::microvm::MachineProfile::Microvm,
                linux_mptable: self.boots_linux_mptable(),
            },
        }
    }
}

} // verus!
