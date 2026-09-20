// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

use vstd::prelude::*;

verus! {

#[verifier::external_trait_specification]
pub trait ExError: core::fmt::Debug + core::fmt::Display {
    type ExternalTraitSpecificationFor: core::error::Error;
}

}
