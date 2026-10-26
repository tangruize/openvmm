// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

// Declarations supporting `worker.spec.rs`; no human review needed.

use super::SavedState;
use vstd::prelude::*;

verus! {

// Make the fields of `SavedState` visible to Verus. Verus checks that they
// match the real definition. (`state_unit::SavedStateUnit` is declared in
// `state_unit`.)

#[verifier::external_type_specification]
pub struct ExSavedState(SavedState);

} // verus!
