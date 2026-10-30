// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

// Declarations supporting `time_abi.spec.rs`; no human review needed.

use super::TimeAbiUnit;
use vstd::prelude::*;

verus! {

/// Declares `TimeAbiUnit` (its fields) to Verus.
#[verifier::external_type_specification]
pub(in crate::worker::dispatch) struct ExTimeAbiUnit(TimeAbiUnit);

/// Declares `TimeAbiReport` to Verus, by name only (fields hidden).
#[verifier::external_type_specification]
#[verifier::external_body]
pub(in crate::worker::dispatch) struct ExTimeAbiReport(super::TimeAbiReport);

} // verus!
