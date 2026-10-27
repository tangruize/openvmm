// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

//! Trusted: assumed specifications for registry code.
//!
//! Everything in this file is TRUSTED. It is listed in
//! `SNAPSHOT-SPEC-TRUSTED.md`.
//!
//! Also assumed specifications for registry code Verus cannot see into:
//! `State`'s equality, `ReadySet`'s clone, and `Inner::ready_set`.

use crate::Inner;
use crate::ReadySet;
use crate::State;
use vstd::prelude::*;

verus! {

/// Trusted: `State`'s derived equality is structural.
pub assume_specification[ <State as PartialEq>::eq ](a: &State, b: &State) -> (result: bool)
    ensures
        result == (*a == *b),
;

/// Trusted: a clone of a `ReadySet` holds the same units.
pub assume_specification[ <ReadySet as Clone>::clone ](set: &ReadySet) -> (result: ReadySet)
    ensures
        result@ == set@,
;

/// Trusted: with `unit_ids` `None`, the ready set holds every registered
/// unit. (`Inner::ready_set` builds it with an iterator chain.)
pub(crate) assume_specification[ Inner::ready_set ](inner: &Inner, unit_ids: Option<&[u64]>) -> (result:
    ReadySet)
    ensures
        unit_ids is None ==> inner.unit_ids() <= result@,
;

} // verus!
