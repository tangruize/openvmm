// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

// Declarations supporting `line_sets.spec.rs`; no human review needed.

use super::LineSetUnit;
use vstd::prelude::*;

verus! {

/// Declares `LineSetUnit` (its fields) to Verus.
#[verifier::external_type_specification]
pub(super) struct ExLineSetUnit<'a>(LineSetUnit<'a>);

/// Declares `LineSet` to Verus, by name only (fields hidden).
#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExLineSet(vmcore::line_interrupt::LineSet);

} // verus!
