// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

// Declarations and the registry's lock invariant; no human review needed.

use super::Inner;
use super::InnerPred;
use super::mutex_t::MutexPredicate;
use vstd::prelude::*;

verus! {

#[verifier::external_type_specification]
#[verifier::external_body]
pub(super) struct ExInner(Inner);

#[verifier::external_type_specification]
#[verifier::external_body]
pub(super) struct ExInnerPred(InnerPred);

impl MutexPredicate<Inner> for InnerPred {
    /// No invariant yet; the FracGhost halves waiting in the registry will
    /// add one.
    open spec fn inv(v: Inner) -> bool {
        true
    }
}

} // verus!
