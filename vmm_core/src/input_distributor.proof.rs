// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

// Declarations supporting `input_distributor.spec.rs`; no human review needed.

use super::Inner;
use vstd::prelude::*;

verus! {

/// Declares `Inner` (its fields) to Verus.
#[verifier::external_type_specification]
pub(super) struct ExInner(Inner);

/// Declares `Forwarder` to Verus, as an opaque type.
#[verifier::external_type_specification]
#[verifier::external_body]
#[verifier::reject_recursive_types(T)]
pub(super) struct ExForwarder<T>(super::Forwarder<T>);

/// Declares `KeyboardData` to Verus, as an opaque type.
#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExKeyboardData(input_core::KeyboardData);

/// Declares `MouseData` to Verus, as an opaque type.
#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExMouseData(input_core::MouseData);

/// Declares `InputDistributor` to Verus, as an opaque type.
#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExInputDistributor(super::InputDistributor);

} // verus!
