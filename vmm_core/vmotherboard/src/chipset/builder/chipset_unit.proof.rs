// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

// Declarations supporting `chipset_unit.spec.rs`; no human review needed.
// (`ChipsetUnit` itself is `verus_verify`.)

use vstd::prelude::*;

verus! {

/// Declares `Chipset` to Verus, as an opaque type.
#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExChipset(crate::Chipset);

} // verus!
