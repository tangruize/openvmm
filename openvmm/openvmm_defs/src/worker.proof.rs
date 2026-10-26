// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

// Declarations supporting `worker.spec.rs`; no human review needed.

use super::SavedState;
use state_unit::SavedStateUnit;
use vstd::prelude::*;

verus! {

// Make the fields of these types visible to Verus. Verus checks that they
// match the real definitions.

#[verifier::external_type_specification]
pub struct ExSavedState(SavedState);

#[verifier::external_type_specification]
pub struct ExSavedStateUnit(SavedStateUnit);

} // verus!
