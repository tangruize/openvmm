// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

// Declarations supporting `line_sets.spec.rs`; no human review needed.

use super::LineSetUnit;
use vstd::prelude::*;

verus! {

/// Declares `LineSetUnit` to Verus, as an opaque type.
#[verifier::external_type_specification]
#[verifier::external_body]
pub(super) struct ExLineSetUnit<'a>(LineSetUnit<'a>);

} // verus!
