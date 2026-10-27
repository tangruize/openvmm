// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

// High-level spec of the PIT, for human review.

use super::PitDevice;
use super::TimerState;
use super::save_restore::state::SavedCountState;
use super::save_restore::state::SavedState;
use super::save_restore::state::SavedTimerState;
use super::CountState;
use closeable_mutex::LockInvariant;
use vmcore::device_state::spec::DeviceGhost;
use vmcore::device_state::spec::DeviceGhostState;
use vmcore::save_restore::spec::SaveRestoreSpecImpl;
use vmcore::save_restore::spec::SavedStateBlobView;
use vmcore::save_restore::spec::UnitSnapshot;
use vmcore::vmtime::VmTime;
use vstd::prelude::*;
use vstd::resource::Loc;

verus! {

/// The view of a `PitDevice`: its snapshotable state and its configuration.
/// (Not in the view: the runtime glue, `vmtime` and the interrupt line;
/// `dram_refresh`, which is not saved; and the ghost field, given by
/// `DeviceGhost::ghost`.)
pub struct PitDeviceView {
    /// The states of the three timers.
    pub(super) timers: Seq<TimerState>,
    /// The time up to which the timers have been evaluated.
    pub(super) last: VmTime,
    /// Whether the NVX time ABI applies (static configuration).
    pub(super) time_abi: bool,
}

impl PitDeviceView {
    /// Three timers.
    pub(super) open spec fn inv(self) -> bool {
        self.timers.len() == 3
    }

    /// The saved state of a PIT in this state: what its `save` returns.
    pub(super) open spec fn saved_state(self) -> SavedState {
        SavedState {
            timers: [self.timers[0].saved(), self.timers[1].saved(), self.timers[2].saved()],
            last: self.last,
        }
    }

    /// What the FracGhost of the PIT's state unit records for a PIT in this
    /// state: its saved state, encoded.
    pub(super) open spec fn snapshot(self) -> UnitSnapshot {
        UnitSnapshot::Saved(SavedStateBlobView::of(self.saved_state()))
    }
}

impl TimerState {
    /// The saved form of a timer's state, field by field.
    pub(super) open spec fn saved(self) -> SavedTimerState {
        SavedTimerState {
            ce: self.ce,
            cr: self.cr,
            ol: self.ol,
            sl: self.sl,
            state: match self.state {
                CountState::Inactive => SavedCountState::Inactive,
                CountState::WaitingForGate => SavedCountState::WaitingForGate,
                CountState::Reloading => SavedCountState::Reloading,
                CountState::Active => SavedCountState::Active,
                CountState::Counting => SavedCountState::Counting,
            },
            control: self.control.0,
            out: self.out,
            gate: self.gate,
            null: self.null,
            read_high: self.read_high,
            cr_low: self.cr_low,
        }
    }
}

impl View for PitDevice {
    type V = PitDeviceView;

    closed spec fn view(&self) -> PitDeviceView {
        PitDeviceView {
            timers: self.timers@.map_values(|t: super::Timer| t.state),
            last: self.last,
            time_abi: self.time_abi,
        }
    }
}

impl PitDevice {
    /// Well formed: the view's invariant holds, and the ghost field follows
    /// the rule (`DeviceGhostState::wf`) for the device's snapshot.
    pub closed spec fn wf(&self) -> bool {
        &&& self@.inv()
        &&& self.ghost@.wf(self.id@, self@.snapshot())
    }
}

/// The PIT's saved state is its view's.
impl SaveRestoreSpecImpl for PitDevice {
    closed spec fn saved_state(&self) -> SavedState {
        self@.saved_state()
    }
}

/// The PIT's ghost field, for its state unit.
impl DeviceGhost for PitDevice {
    closed spec fn ghost(&self) -> DeviceGhostState {
        self.ghost@
    }

    closed spec fn id(&self) -> Loc {
        self.id@
    }

    closed spec fn snapshot(&self) -> UnitSnapshot {
        self@.snapshot()
    }

    closed spec fn device_inv(&self) -> bool {
        self@.inv()
    }

    proof fn lemma_snapshot_known(&self) {
    }

    fn ghost_mut(&mut self) -> (r: &mut Tracked<DeviceGhostState>) {
        &mut self.ghost
    }
}

/// The invariant of the PIT's `CloseableMutex`: the device is well formed,
/// and keeps the id of its FracGhost.
impl LockInvariant for PitDevice {
    type Key = Loc;

    closed spec fn key(&self) -> Loc {
        self.id@
    }

    closed spec fn inv(&self) -> bool {
        self.wf()
    }
}

} // verus!
