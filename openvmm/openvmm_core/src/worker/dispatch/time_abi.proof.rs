// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

// Declarations supporting `time_abi.spec.rs`; no human review needed.

use super::TimeAbiUnit;
use vstd::prelude::*;

verus! {

/// Declares `TimeAbiUnit` to Verus, as an opaque type.
#[verifier::external_type_specification]
#[verifier::external_body]
pub(in crate::worker::dispatch) struct ExTimeAbiUnit(TimeAbiUnit);

} // verus!
