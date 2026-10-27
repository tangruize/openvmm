// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

// Declarations supporting `msr.spec.rs`; no human review needed. (`TimeAbiMsrs`
// itself is `verus_verify`.)

use super::TimeAbiSavedState;
use crate::time_abi::DeclaredRates;
use std::sync::OnceLock;
use vstd::prelude::*;

verus! {

/// Declares `TimeAbiSavedState` (its field) to Verus.
#[verifier::external_type_specification]
pub struct ExTimeAbiSavedState(TimeAbiSavedState);

/// Declares `DeclaredRates` to Verus, as an opaque type.
#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExDeclaredRates(DeclaredRates);

/// Declares `OnceLock` to Verus, as an opaque type.
#[verifier::external_type_specification]
#[verifier::external_body]
#[verifier::reject_recursive_types(T)]
pub struct ExOnceLock<T>(OnceLock<T>);

} // verus!
