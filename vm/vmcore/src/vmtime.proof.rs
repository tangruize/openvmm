// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

// Declarations supporting `vmtime.spec.rs`; no human review needed.

use super::SavedState;
use super::VmTimeKeeper;
use vstd::prelude::*;

verus! {

/// Declares `VmTimeKeeper` to Verus, by name only (fields hidden).
#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExVmTimeKeeper(VmTimeKeeper);

/// Declares vmtime's `SavedState` to Verus, by name only (fields hidden).
#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExSavedState(SavedState);

} // verus!
