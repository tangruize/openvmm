// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

// Declarations supporting `chipset_unit.spec.rs`; no human review needed.

use super::ChipsetUnit;
use vstd::prelude::*;

verus! {

/// Declares `ChipsetUnit` to Verus, as an opaque type.
#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExChipsetUnit<'a>(ChipsetUnit<'a>);

} // verus!
