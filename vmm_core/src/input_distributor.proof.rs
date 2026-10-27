// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

// Declarations supporting `input_distributor.spec.rs`; no human review needed.

use super::Inner;
use vstd::prelude::*;

verus! {

/// Declares `Inner` to Verus, as an opaque type.
#[verifier::external_type_specification]
#[verifier::external_body]
pub(super) struct ExInner(Inner);

} // verus!
