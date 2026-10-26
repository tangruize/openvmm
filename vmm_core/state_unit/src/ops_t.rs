// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

//! Trusted helpers for the verified state-unit operations (`ops.rs`).
//!
//! Everything in this file is TRUSTED, and listed in
//! `SNAPSHOT-SPEC-TRUSTED.md`. Each item is code that Verus cannot see into
//! (tracing, timing, formatted panics, future combinators, iterator chains);
//! its contract states what the code plainly does.

use crate::Inner;
#[cfg(verus_keep_ghost)]
use crate::ReadySet;
use crate::State;
use crate::Unit;
use crate::UnitRecvError;
#[cfg(verus_keep_ghost)]
use crate::proof::ArcStrView;
use futures::future::join_all;
use std::future::Future;
use std::sync::Arc;
use std::time::Instant;
use tracing::Instrument;
#[cfg(verus_keep_ghost)]
use vstd::future::FutureAdditionalSpecFns;
use vstd::prelude::*;

verus! {

/// Awaits all of `futs`, logging how long it took. Trusted: each output is
/// its future's, in order.
#[verifier::external_body]
pub(crate) async fn join_all_timed<F: Future>(futs: Vec<F>) -> (result: Vec<F::Output>)
    ensures
        result@.len() == futs@.len(),
        forall|i: int| 0 <= i < result@.len() ==> #[trigger] futs@[i].awaited() && result@[i] == futs@[i]@,
{
    let start = Instant::now();
    let results = join_all(futs).await;
    tracing::info!(duration = ?Instant::now() - start, "state change complete");
    results
}

/// Runs `fut` inside `span`. Trusted: the output is `fut`'s.
#[verifier::external_body]
pub(crate) async fn instrumented<F: Future>(fut: F, span: tracing::Span) -> (result: F::Output)
    ensures
        fut.awaited(),
        result == fut@,
{
    fut.instrument(span).await
}

/// The span of a state change operation `op`.
#[verifier::external_body]
pub(crate) fn state_change_span(op: &str) -> tracing::Span {
    tracing::info_span!("state_change", operation = op)
}

/// The span of a state change of unit `name`.
#[verifier::external_body]
pub(crate) fn device_state_change_span(name: &Arc<str>) -> tracing::Span {
    tracing::info_span!("device_state_change", device = name.as_ref())
}

/// The current time, for logging.
#[verifier::external_body]
pub(crate) fn start_timer() -> Instant {
    Instant::now()
}

/// Logs that a unit's state change, started at `start`, is complete.
#[verifier::external_body]
pub(crate) fn log_device_state_change(start: Instant) {
    tracing::debug!(duration = ?Instant::now() - start, "device state change complete");
}

/// Panics unless `unit` is in state `expected` (the message names `other` as
/// the other allowed state). Trusted: returns only if it is.
#[verifier::external_body]
pub(crate) fn expect_state(unit: &Unit, other: State, expected: State)
    ensures
        unit.state == expected,
{
    assert_eq!(
        unit.state, expected,
        "unit {} in {:?} state, should be {:?} or {:?}",
        unit.name, unit.state, other, expected
    );
}

/// Panics: communication with a registered unit failed. Trusted: does not
/// return.
#[verifier::external_body]
pub(crate) fn unit_failed(err: UnitRecvError) -> ! {
    panic!("{:?}", err)
}

/// The dependency list that `deps` gives for `unit`. No contract.
#[verifier::external_body]
pub(crate) fn deps_of<D: FnMut(&Unit) -> &[u64]>(deps: &mut D, unit: &Unit) -> Vec<u64> {
    deps(unit).to_vec()
}

/// The registered units' ids, in increasing order. Trusted: exactly the
/// registry's keys, once each.
#[verifier::external_body]
pub(crate) fn unit_ids_of(inner: &Inner) -> (ids: Vec<u64>)
    ensures
        ids@.no_duplicates(),
        ids@.to_set() == inner.units@.dom(),
{
    inner.units.keys().copied().collect()
}

/// Clones a unit's name. Trusted: a clone of an `Arc<str>` has the same
/// contents.
#[verifier::external_body]
pub(crate) fn clone_name(name: &Arc<str>) -> (result: Arc<str>)
    ensures
        result.view_str() == name.view_str(),
{
    name.clone()
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
/// unit.
pub assume_specification[ Inner::ready_set ](inner: &Inner, unit_ids: Option<&[u64]>) -> (result:
    ReadySet)
    ensures
        unit_ids is None ==> inner.unit_ids() <= result.ids(),
;

} // verus!
