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
//!
//! AXIOM (exact, `lock_registry_exact`): inside an operation that holds
//! `&mut StateUnits` and has collected since it began, the registry holds
//! exactly the collected units.
//!
//! Also assumed specifications for registry code Verus cannot see into:
//! `State`'s equality, `ReadySet`'s clone, and `Inner::ready_set`.

use crate::Inner;
use crate::InnerPred;
use crate::ReadySet;
use crate::State;
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

    /// Locks the registry, like `lock_registry`, inside an operation that
    /// holds `&mut StateUnits` and has collected the halves since it began.
    /// AXIOM (exact): the registry holds exactly the collected units.
    /// Registration needs `&StateUnits`, which cannot exist while the
    /// operation holds `&mut StateUnits`; removal is excluded by the axiom
    /// above. For use by such operations, after `collect_halves`.
    #[verifier::external_body]
    pub(crate) fn lock_registry_exact(&self) -> (ret: (Inner, MutexHandle<'_, Inner, InnerPred>))
        ensures
            InnerPred::inv(ret.0),
            StateUnits::agrees_exactly(self.collected(), ret.0),
    {
        self.inner.acquire()
    }
}

/// Trusted: `State`'s derived equality is structural.
pub assume_specification[ <State as PartialEq>::eq ](a: &State, b: &State) -> (result: bool)
    ensures
        result == (*a == *b),
;

/// Trusted: a clone of a `ReadySet` holds the same units.
pub assume_specification[ <ReadySet as Clone>::clone ](set: &ReadySet) -> (result: ReadySet)
    ensures
        result.ids() == set.ids(),
;

/// Trusted: with `unit_ids` `None`, the ready set holds every registered
/// unit. (`Inner::ready_set` builds it with an iterator chain.)
pub assume_specification[ Inner::ready_set ](inner: &Inner, unit_ids: Option<&[u64]>) -> (result:
    ReadySet)
    ensures
        unit_ids is None ==> inner.unit_ids() <= result.ids(),
;

} // verus!
