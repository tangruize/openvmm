// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

// Declarations supporting the RTC's spec; no human review needed.

use super::super::super::clock::UtcClockSource;
use super::super::super::mode::RtcMode;
use super::super::super::spec::CmosReg;
use super::super::state::SavedState;
use local_clock::LocalClockTime;
use vmcore::vmtime::VmTimerPeriodic;
use vstd::prelude::*;

verus! {

// The RTC's saved state, visible to Verus. (`Rtc`, `RtcState`, and
// `CmosData` are `verus_verify` themselves: their fields are private to
// `cmos_rtc`.)

#[verifier::external_type_specification]
pub struct ExSavedState(SavedState);

// The types of the RTC's other fields, by name only (fields hidden; `LineInterrupt`
// and `VmTimeAccess` are declared with the PIT).

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExCmosReg(CmosReg);

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExRtcMode(RtcMode);

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExVmTimerPeriodic(VmTimerPeriodic);

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExLocalClockTime(LocalClockTime);

#[verifier::external_trait_specification]
pub trait ExUtcClockSource: inspect::Inspect + Send {
    type ExternalTraitSpecificationFor: UtcClockSource;
}

} // verus!
