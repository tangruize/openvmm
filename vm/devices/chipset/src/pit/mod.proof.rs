// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

// Declarations supporting the PIT's spec; no human review needed.

use super::super::ControlWord;
use super::super::PitDevice;
use super::super::CountState;
use super::super::Mode;
use super::super::Timer;
use super::super::TimerState;
use super::super::save_restore::state::SavedCountState;
use super::super::save_restore::state::SavedState;
use super::super::save_restore::state::SavedTimerState;
use closeable_mutex::CloseableMutex;
use closeable_mutex::LockInvariant;
use vmcore::device_state::spec::DeviceGhost;
use std::sync::Arc;
use vmcore::save_restore::RestoreError;
use vmcore::save_restore::SaveError;
use vmcore::save_restore::spec::SavedStateBlobView;
use vmcore::line_interrupt::LineInterrupt;
use vmcore::save_restore::spec::UnitSnapshot;
use vmcore::vmtime::VmTime;
use vmcore::vmtime::VmTimeAccess;
use vstd::prelude::*;
use vstd::resource::frac::FracGhost;

verus! {

// Make the PIT's types visible to Verus. Verus checks that their fields
// match the real definitions.

#[verifier::external_type_specification]
pub(crate) struct ExTimer(Timer);

#[verifier::external_type_specification]
pub(crate) struct ExTimerState(TimerState);

#[verifier::external_type_specification]
pub(crate) struct ExCountState(CountState);

#[verifier::external_type_specification]
pub(crate) struct ExMode(Mode);

#[verifier::external_type_specification]
pub(crate) struct ExControlWord(ControlWord);

#[verifier::external_type_specification]
pub struct ExSavedState(SavedState);

#[verifier::external_type_specification]
pub struct ExSavedTimerState(SavedTimerState);

#[verifier::external_type_specification]
pub struct ExSavedCountState(SavedCountState);

// Runtime glue and time, as opaque types.

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExVmTime(VmTime);

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExVmTimeAccess(VmTimeAccess);

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExLineInterrupt(LineInterrupt);

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExContext<'a>(std::task::Context<'a>);

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExIoResult(chipset_device::io::IoResult);

// Examples: the PIT's state unit answering requests by the request
// protocol, through the device's `CloseableMutex`, as the unit loop would
// (step 5): the ghost work is in the unit's own proof blocks, through
// `DeviceGhost::ghost_mut`. Not production code.

/// `Start`: the unit gives `StateUnits`' half to the device and starts it.
pub(in crate::pit) fn example_start(
    m: Arc<CloseableMutex<PitDevice>>,
    half: Tracked<FracGhost<UnitSnapshot>>,
) -> (r: Arc<CloseableMutex<PitDevice>>)
    requires
        half@.valid((*m).key(), 0.5real),
    ensures
        (*r).key() == (*m).key(),
{
    let Tracked(half) = half;
    let mut g = CloseableMutex::close_verified(m);
    let d = g.borrow_mut();
    let ghost_field = d.ghost_mut();
    proof {
        // The whole FracGhost: the device was stopped.
        ghost_field.borrow_mut().frac.combine(half);
        ghost_field.borrow_mut().frac.bounded();
        ghost_field.borrow_mut().running = true;
    }
    d.start_device();
    g.release()
}

/// `Stop`: the device's FracGhost records its snapshot; the unit returns a
/// half.
pub(in crate::pit) fn example_stop(m: Arc<CloseableMutex<PitDevice>>) -> (r: (
    Arc<CloseableMutex<PitDevice>>,
    Tracked<FracGhost<UnitSnapshot>>,
))
    ensures
        (*r.0).key() == (*m).key(),
        r.1@.valid((*m).key(), 0.5real),
        !(r.1@@ is Unknown),
{
    let mut g = CloseableMutex::close_verified(m);
    let d = g.borrow_mut();
    // TRUSTED: a unit receives `Stop` only while it runs ("`StateUnits`'
    // unit states are the units' real states").
    proof {
        assume(d.ghost().running);
    }
    let ghost now = d.snapshot();
    let ghost_field = d.ghost_mut();
    let tracked half;
    proof {
        ghost_field.borrow_mut().frac.update(now);
        half = ghost_field.borrow_mut().frac.split();
        ghost_field.borrow_mut().running = false;
    }
    (g.release(), Tracked(half))
}

/// `Save`: with the lent half, which records a known state, the saved state
/// matches it.
pub(in crate::pit) fn example_save(
    m: Arc<CloseableMutex<PitDevice>>,
    lent: Tracked<FracGhost<UnitSnapshot>>,
) -> (r: (
    Arc<CloseableMutex<PitDevice>>,
    Result<SavedState, SaveError>,
    Tracked<FracGhost<UnitSnapshot>>,
))
    requires
        lent@.valid((*m).key(), 0.5real),
        !(lent@@ is Unknown),
    ensures
        (*r.0).key() == (*m).key(),
        r.2 == lent,
        r.1 matches Ok(saved) ==> lent@@ == UnitSnapshot::Saved(SavedStateBlobView::of(saved)),
{
    let Tracked(lent) = lent;
    let mut g = CloseableMutex::close_verified(m);
    let d = g.borrow_mut();
    let ghost_field = d.ghost_mut();
    proof {
        // The lent half agrees with the device's, which records its state.
        ghost_field.borrow().frac.agree(&lent);
    }
    let result = d.save_device();
    (g.release(), result, Tracked(lent))
}

/// `Restore`: with the lent half the stopped device holds the whole, is
/// restored, and the unit returns a half recording the restored state.
pub(in crate::pit) fn example_restore(
    m: Arc<CloseableMutex<PitDevice>>,
    state: SavedState,
    lent: Tracked<FracGhost<UnitSnapshot>>,
) -> (r: (
    Arc<CloseableMutex<PitDevice>>,
    Result<(), RestoreError>,
    Tracked<FracGhost<UnitSnapshot>>,
))
    requires
        lent@.valid((*m).key(), 0.5real),
    ensures
        (*r.0).key() == (*m).key(),
        r.2@.valid((*m).key(), 0.5real),
        r.1 is Ok ==> r.2@@ == UnitSnapshot::Saved(SavedStateBlobView::of(state)),
{
    let Tracked(lent) = lent;
    let mut g = CloseableMutex::close_verified(m);
    let d = g.borrow_mut();
    let ghost_field = d.ghost_mut();
    proof {
        // The whole FracGhost: the device is stopped.
        ghost_field.borrow_mut().frac.combine(lent);
        ghost_field.borrow_mut().frac.bounded();
    }
    let result = d.restore_device(state);
    let ghost now = d.snapshot();
    let ghost_field = d.ghost_mut();
    let tracked half;
    proof {
        ghost_field.borrow_mut().frac.update(now);
        half = ghost_field.borrow_mut().frac.split();
    }
    (g.release(), result, Tracked(half))
}

} // verus!
