// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

// Declarations supporting `worker.spec.rs`; no human review needed.

use crate::worker::SavedState;
use vstd::prelude::*;

verus! {

// Make the fields of `SavedState` visible to Verus. Verus checks that they
// match the real definition. (`state_unit::SavedStateUnit` is declared in
// `state_unit`.)

#[verifier::external_type_specification]
pub struct ExSavedState(SavedState);

// Types in the snapshot RPC's contracts.

#[verifier::external_type_specification]
pub struct ExSnapshotQuiesceError(crate::rpc::SnapshotQuiesceError);

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExTimeCapture(crate::time_abi::TimeCapture);

} // verus!
