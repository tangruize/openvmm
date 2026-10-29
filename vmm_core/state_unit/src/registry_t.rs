// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

//! Assumed specifications for registry code.
//!
//! Everything in this file is ASSUMED, to prove (it is about our own code:
//! `State`'s derived equality, `ReadySet`'s clone, `Inner::ready_set`), not
//! part of the trusted base. It is listed in `SNAPSHOT-SPEC-TRUSTED.md`,
//! Part 3. PROOF TODO: replace each `assume_specification` by a verified
//! function.
//!
//! Also assumed specifications for registry code Verus cannot see into:
//! `State`'s equality, `ReadySet`'s clone, and `Inner::ready_set`.

use crate::Inner;
use crate::ReadySet;
use crate::State;
use vstd::prelude::*;

verus! {

/// Assumed (to prove): `State`'s derived equality is structural.
pub assume_specification[ <State as PartialEq>::eq ](a: &State, b: &State) -> (result: bool)
    ensures
        result == (*a == *b),
;

/// Assumed (to prove): a clone of a `ReadySet` holds the same units.
pub assume_specification[ <ReadySet as Clone>::clone ](set: &ReadySet) -> (result: ReadySet)
    ensures
        result@ == set@,
;

/// Assumed (to prove): with `unit_ids` `None`, the ready set holds every registered
/// unit. (`Inner::ready_set` builds it with an iterator chain.)
pub(crate) assume_specification[ Inner::ready_set ](inner: &Inner, unit_ids: Option<&[u64]>) -> (result:
    ReadySet)
    ensures
        unit_ids is None ==> inner.unit_ids() <= result@,
;

} // verus!
