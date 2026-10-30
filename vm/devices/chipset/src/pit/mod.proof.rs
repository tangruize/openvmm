// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

// Declarations supporting the PIT's spec; no human review needed.

use super::super::ControlWord;
use super::super::CountState;
use super::super::Mode;
use super::super::PitDevice;
use super::super::Timer;
use super::super::TimerState;
use super::super::save_restore::state::SavedCountState;
use super::super::save_restore::state::SavedState;
use super::super::save_restore::state::SavedTimerState;
use vmcore::line_interrupt::LineInterrupt;
use vmcore::vmtime::VmTime;
use vmcore::vmtime::VmTimeAccess;
use vstd::prelude::*;

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

// Runtime glue and time, by name only (fields hidden).

/// Declares `std::task::Context` to Verus, by name only (fields hidden; the PIT's
/// `poll` takes one).
#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExContext<'a>(std::task::Context<'a>);

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
pub struct ExIoResult(chipset_device::io::IoResult);

} // verus!
