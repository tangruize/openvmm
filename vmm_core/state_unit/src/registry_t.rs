// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

//! Trusted: locking `StateUnits`' registry, with the registry axiom.
//!
//! Everything in this file is TRUSTED. It is listed in
//! `SNAPSHOT-SPEC-TRUSTED.md`.
//!
//! Units are added through `&StateUnits` (`UnitBuilder::build`) and removed
//! through `Weak` handles (`UnitHandle`), both unverified, so no lock
//! invariant can tie the registry behind the mutex to `StateUnits`' own
//! ghost fields. This file states that tie as an axiom, made only about the
//! value just taken out of `StateUnits`' own registry mutex:
//!
//! AXIOM (registry unchanged since collection): since `StateUnits` last
//! collected halves, no collected unit was removed or changed its name or
//! channel; and every unit added since still has its half attached to its
//! sender (its sender came from `unit_channel`).

use crate::Inner;
use crate::InnerPred;
use crate::StateUnits;
use crate::mutex_t::MutexHandle;
use crate::mutex_t::MutexPredicate;
use vstd::prelude::*;

verus! {

impl StateUnits {
    /// Locks the registry and takes it out, like `Mutex::acquire`, with the
    /// registry axiom.
    #[verifier::external_body]
    pub(crate) fn lock_registry(&self) -> (ret: (Inner, MutexHandle<'_, Inner, InnerPred>))
        ensures
            InnerPred::inv(ret.0),
            StateUnits::agrees_with(self.collected(), ret.0),
    {
        self.inner.acquire()
    }
}

} // verus!
