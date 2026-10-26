// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

//! The state-unit operations that send requests to the units: `run_op` and
//! its helpers, and `stop`, `reset`, `save`, and `restore`.
//!
//! Written in `verus!` style, because each unit's ghost half comes back
//! through a future awaited later (`join_all`), which annotation style cannot
//! express (`SNAPSHOT-SPEC-DESIGN.md`, "Conventions"). Trusted helpers are in
//! `ops_t.rs`.

use crate::ReadySet;
use crate::SavedStateUnit;
use crate::State;
use crate::StateRequest;
use crate::StateTransitionError;
use crate::StateUnits;
use crate::Unit;
use crate::UnitRecvError;
use crate::check;
use crate::extract;
use crate::mesh_rpc_t::*;
#[cfg(verus_keep_ghost)]
use crate::mutex_t::MutexPredicate;
use crate::ops_t::*;
#[cfg(verus_keep_ghost)]
use crate::proof::ArcStrView;
#[cfg(verus_keep_ghost)]
use crate::spec::UnitSnapshot;
use mesh::Sender;
use mesh::rpc::Rpc;
use std::collections::HashMap;
use std::future::Future;
use std::sync::Arc;
use thiserror::Error;
#[cfg(verus_keep_ghost)]
use vstd::future::FutureAdditionalSpecFns;
#[cfg(verus_keep_ghost)]
use vstd::std_specs::iter::IteratorSpec;
use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
use vstd::resource::frac::FracGhost;

/// In a normal build, ghosts are erased.
#[cfg(not(verus_keep_ghost))]
pub(crate) type UnitGhost = ();

verus! {

broadcast use vstd::laws_cmp::group_laws_cmp;

/// The ghost that travels with a unit's request or reply.
#[cfg(verus_keep_ghost)]
pub(crate) type UnitGhost = Option<FracGhost<UnitSnapshot>>;

/// What `state_change_ghost` returns: the reply, and the ghost that came back.
pub(crate) type ChangeOutput<R> = (Result<Option<R>, UnitRecvError>, Tracked<UnitGhost>);

impl StateUnits {
    /// Stops all the state units.
    #[verifier::external_body]
    pub async fn stop(&mut self) {
        assert!(self.running);
        // Stop units in reverse dependency order so that a dependency is not
        // stopped before its dependant.
        self.run_op(
            "stop",
            None,
            State::Running,
            State::Stopping,
            State::Stopped,
            StateRequest::Stop,
            |_, _| Some(()),
            |unit| &unit.dependents,
            Ghost::assume_new(),
            Tracked::assume_new(),
            Ghost::assume_new(),
        )
        .await;
        self.running = false;
    }

    /// Resets all the state units.
    ///
    /// Panics if running.
    #[verifier::external_body]
    pub async fn reset(&mut self) -> Result<(), StateTransitionError> {
        assert!(!self.running);
        // Reset in dependency order so that dependants observe their
        // dependencies' reset state.
        let (r, _, _) = self
            .run_op(
                "reset",
                None,
                State::Stopped,
                State::Resetting,
                State::Stopped,
                StateRequest::Reset,
                |_, _| Some(()),
                |unit| &unit.dependencies,
                Ghost::assume_new(),
                Tracked::assume_new(),
                Ghost::assume_new(),
            )
            .await;

        check("reset", r)?;
        Ok(())
    }

    /// Saves all the state units.
    ///
    /// Panics if running.
    #[verifier::external_body]
    pub async fn save(&mut self) -> Result<Vec<SavedStateUnit>, StateTransitionError> {
        assert!(!self.running);
        // Save can occur in any order since it will not observably mutate
        // state.
        let (r, _, _) = self
            .run_op(
                "save",
                None,
                State::Stopped,
                State::Saving,
                State::Stopped,
                StateRequest::Save,
                |_, _| Some(()),
                |_| &[],
                Ghost::assume_new(),
                Tracked::assume_new(),
                Ghost::assume_new(),
            )
            .await;

        let states = extract("save", r, |name, state| {
            state.map(|state| SavedStateUnit {
                name: name.to_string(),
                state,
            })
        })?;

        Ok(states)
    }

    /// Restores all the state units.
    ///
    /// Panics if running.
    #[verifier::external_body]
    pub async fn restore(
        &mut self,
        states: Vec<SavedStateUnit>,
    ) -> Result<(), StateTransitionError> {
        assert!(!self.running);

        #[derive(Debug, Error)]
        enum RestoreUnitError {
            #[error("unknown unit name")]
            Unknown,
            #[error("duplicate unit name")]
            Duplicate,
        }

        let mut states_by_id = HashMap::new();
        let mut r = Vec::new();
        {
            let inner = self.inner.lock();
            for state in states {
                match inner.names.get(state.name.as_str()) {
                    Some(&id) => {
                        // The same `Arc` as the key in `names` (see `build`).
                        let name = &inner.units[&id].name;
                        if states_by_id
                            .insert(id, (name.clone(), state.state))
                            .is_some()
                        {
                            r.push((name.clone(), Err(RestoreUnitError::Duplicate)));
                        }
                    }
                    None => {
                        r.push((state.name.into(), Err(RestoreUnitError::Unknown)));
                    }
                }
            }
        }

        check("restore", r)?;

        let (r, _, _) = self
            .run_op(
                "restore",
                None,
                State::Stopped,
                State::Restoring,
                State::Stopped,
                StateRequest::Restore,
                |id, _| states_by_id.remove(&id).map(|(_, blob)| blob),
                |unit| &unit.dependencies,
                Ghost::assume_new(),
                Tracked::assume_new(),
                Ghost::assume_new(),
            )
            .await;

        // Make sure all the saved state was consumed. This could hit if a unit
        // was removed concurrently with the restore.
        check(
            "restore",
            states_by_id
                .into_iter()
                .map(|(_, (name, _))| (name, Err(RestoreUnitError::Unknown))),
        )?;

        check("restore", r)?;

        Ok(())
    }

    /// Runs a state change operation on a set of units.
    ///
    /// `op` gives the name of the operation for tracing and error reporting
    /// purposes.
    ///
    /// `unit_ids` is the set of the units whose state should be changed. If
    /// `unit_ids` is `None`, all units change states.
    ///
    /// The old state for each unit must be `old_state`. During the operation,
    /// the unit is temporarily places in `interim_state`. When complete, the
    /// unit is placed in `new_state`.
    ///
    /// Each unit waits for its dependencies to complete their state change
    /// operation before proceeding with their own state change. The
    /// dependencies list is computed for a unit by calling `deps`.
    ///
    /// To perform the state change, the unit is sent a request generated using
    /// `request`, with input generated by `input`. If `input` returns `None`,
    /// then communication with the unit is skipped, but the unit still
    /// transitions through the interim and into the new state, and its
    /// dependencies are still waited on by its dependents.
    ///
    /// Verus: `input_of(id)` is what `input` returns for unit `id`; `gins`
    /// is the ghost sent with each collected unit's request, by unit id; and
    /// `post(id, input, reply, gout)` is what the caller expects of each
    /// reply. Returns the replies, the ghosts that came back (`gins`' own for
    /// units sent no request), and the ids of the units that replied, in the
    /// order of the replies.
    #[allow(unused_variables)] // ghost parameters are erased at runtime
    pub(crate) async fn run_op<I: 'static, R: 'static + Send, Req, In, D>(
        &self,
        op: &str,
        unit_ids: Option<&[u64]>,
        old_state: State,
        interim_state: State,
        new_state: State,
        request: Req,
        input: In,
        deps: D,
        input_of: Ghost<spec_fn(u64) -> Option<I>>,
        gins: Tracked<Map<u64, UnitGhost>>,
        post: Ghost<spec_fn(u64, I, R, UnitGhost) -> bool>,
    ) -> (result: (Vec<(Arc<str>, R)>, Tracked<Map<u64, UnitGhost>>, Ghost<Seq<u64>>)) where
        Req: Copy + FnOnce(Rpc<I, R>) -> StateRequest,
        In: FnMut(u64, &Unit) -> Option<I>,
        D: FnMut(&Unit) -> &[u64],

        requires
            unit_ids is None,
            gins@.dom() == self.collected().dom(),
            forall|rpc: Rpc<I, R>| #[trigger] request.requires((rpc,)),
            forall|id: u64, unit: &Unit| #[trigger] input.requires((id, unit)),
            forall|id: u64, unit: &Unit, i: Option<I>|
                #[trigger] input.ensures((id, unit), i) ==> i == input_of@(id),
            // Each unit's request may be sent with its ghost.
            forall|id: u64, i: I|
                self.collected().contains_key(id) ==> #[trigger] StateRequest::may_send(
                    request,
                    self.collected()[id].1,
                    gins@[id],
                    id,
                    i,
                    post@,
                ),
        ensures
            ({
                let (replies, gouts, replied) = result;
                &&& gouts@.dom() == gins@.dom()
                &&& replies@.len() == replied@.len()
                &&& replied@.no_duplicates()
                &&& forall|k: int| 0 <= k < replies@.len() ==> {
                    let id = #[trigger] replied@[k];
                    &&& self.collected().contains_key(id)
                    &&& replies@[k].0.view_str() == self.collected()[id].0
                    &&& input_of@(id) matches Some(i) && post@(id, i, replies@[k].1, gouts@[id])
                }
                // Units that did not reply were sent no request: no input, or
                // not in `old_state` (possible only if that is not `new_state`).
                &&& forall|id: u64| #[trigger] self.collected().contains_key(id) && !replied@.contains(id)
                    ==> gouts@[id] == gins@[id] && (input_of@(id) is None || old_state != new_state)
            }),
    {
        // (`mut` on the parameters is lost in Verus's async desugaring.)
        let mut input = input;
        let mut deps = deps;
        let Tracked(mut gins) = gins;
        let ghost gins_at_start = gins;
        let ghost collected = self.collected();
        let mut done = Vec::new();
        let ghost mut sent: Seq<u64> = Seq::empty();
        let ready_set;
        {
            let (mut inner, handle) = self.lock_registry_exact();
            ready_set = inner.ready_set(unit_ids);
            let ids = unit_ids_of(&inner);
            let mut i: usize = 0;
            while i < ids.len()
                invariant
                    0 <= i <= ids@.len(),
                    ids@.no_duplicates(),
                    ids@.to_set() == inner.units@.dom(),
                    StateUnits::agrees_exactly(collected, inner),
                    crate::InnerPred::inv(inner),
                    inner.unit_ids() <= ready_set.ids(),
                    // The units sent a request so far, in order: some of
                    // ids[..i]. The others keep their ghost.
                    done@.len() == sent.len(),
                    sent.no_duplicates(),
                    forall|k: int| 0 <= k < sent.len() ==> ids@.subrange(0, i as int).contains(#[trigger] sent[k]),
                    forall|id: u64| #[trigger] gins.contains_key(id)
                        <==> (collected.contains_key(id) && !sent.contains(id)),
                    forall|id: u64| #[trigger] gins.contains_key(id) ==> gins[id] == gins_at_start[id],
                    forall|j: int| 0 <= j < i && !sent.contains(#[trigger] ids@[j])
                        ==> input_of@(ids@[j]) is None || old_state != new_state,
                    forall|k: int| 0 <= k < done@.len() ==> {
                        let id = #[trigger] sent[k];
                        FutureAdditionalSpecFns::<(Arc<str>, u64, ChangeOutput<R>)>::awaited(&done@[k])
                            ==> StateUnits::change_done(
                            collected,
                            gins_at_start,
                            input_of@,
                            post@,
                            id,
                            FutureAdditionalSpecFns::<(Arc<str>, u64, ChangeOutput<R>)>::view(&done@[k]),
                        )
                    },
                decreases ids@.len() - i,
            {
                let id = ids[i];
                if !ready_set.contains(id) {
                    i += 1;
                    continue;
                }
                let unit = inner.units.get_mut(&id).unwrap();
                if unit.state != old_state {
                    expect_state(unit, old_state, new_state);
                    ready_set.done(id, true);
                } else {
                    let name = clone_name(&unit.name);
                    let input = (input)(id, &*unit);
                    let deps = deps_of(&mut deps, unit);
                    let tracked gin = gins.tracked_remove(id);
                    let fut = state_change_ghost(
                        clone_name(&name),
                        clone_sender(&unit.send),
                        request,
                        input,
                        Tracked(gin),
                        Ghost(id),
                        post,
                    );
                    let recv = wait_then_change(ready_set.clone(), op, id, deps, name, fut);
                    done.push(recv);
                    proof {
                        sent = sent.push(id);
                    }
                    unit.state = interim_state;
                }
                i += 1;
            }
            handle.release(inner);
        }
        let ghost futs = done@;

        let results = instrumented(join_all_timed(done), state_change_span(op)).await;
        proof {
            assert forall|j: int| 0 <= j < results@.len() implies StateUnits::change_done(
                collected,
                gins_at_start,
                input_of@,
                post@,
                sent[j],
                #[trigger] results@[j],
            ) by {
                assert(futs[j].awaited());
            }
            assert(sent.skip(0) =~= sent);
        }

        let (mut inner, handle) = self.lock_registry_exact();
        let mut r: Vec<(Arc<str>, R)> = Vec::new();
        let ghost mut replied: Seq<u64> = Seq::empty();
        // For each reply, the index (in `sent`) of its unit.
        let ghost mut reply_at: Seq<int> = Seq::empty();
        let tracked mut gouts = gins;
        let ghost all = results@;
        let mut iter = results.into_iter();
        let ghost mut k: int = 0;
        loop
            invariant
                0 <= k <= all.len(),
                all.len() == sent.len(),
                IteratorSpec::remaining(&iter) == all.skip(k),
                IteratorSpec::obeys_prophetic_iter_laws(&iter),
                StateUnits::agrees_exactly(collected, inner),
                crate::InnerPred::inv(inner),
                forall|j: int| 0 <= j < all.len() ==> StateUnits::change_done(
                    collected,
                    gins_at_start,
                    input_of@,
                    post@,
                    sent[j],
                    #[trigger] all[j],
                ),
                sent.no_duplicates(),
                forall|id: u64| #[trigger] collected.contains_key(id) && !sent.contains(id)
                    ==> gins_at_start.contains_key(id),
                // gouts: the units not sent a request, and those whose reply is
                // processed (sent[..k]).
                forall|id: u64| #[trigger] gouts.contains_key(id)
                    <==> (collected.contains_key(id) && !sent.skip(k).contains(id)),
                forall|id: u64| #[trigger] gouts.contains_key(id) && !sent.contains(id)
                    ==> gouts[id] == gins_at_start[id],
                forall|j: int| 0 <= j < k ==> gouts[#[trigger] sent[j]] == all[j].2.1@,
                r@.len() == replied.len(),
                replied.no_duplicates(),
                forall|m: int| 0 <= m < replied.len() ==> sent.subrange(0, k).contains(#[trigger] replied[m]),
                reply_at.len() == r@.len(),
                forall|m: int| 0 <= m < r@.len() ==> {
                    let j = #[trigger] reply_at[m];
                    &&& 0 <= j < k
                    &&& sent[j] == replied[m]
                    &&& all[j].2.0 == Ok::<Option<R>, UnitRecvError>(Some(r@[m].1))
                    &&& r@[m].0 == all[j].0
                },
                forall|j: int| 0 <= j < k && !replied.contains(#[trigger] sent[j])
                    ==> all[j].2.0 == Ok::<Option<R>, UnitRecvError>(None),
            decreases all.len() - k,
        {
            match iter.next() {
                Some((name, id, (result, gout))) => {
                    proof {
                        assert(all.skip(k)[0] == all[k]);
                        assert(id == sent[k]);
                        // sent[k..] is sent[k] followed by sent[k+1..], which
                        // does not hold sent[k].
                        assert forall|x: u64| #[trigger] sent.skip(k).contains(x)
                            <==> (x == sent[k] || sent.skip(k + 1).contains(x)) by {
                            if sent.skip(k).contains(x) {
                                let t = choose|t: int| 0 <= t < sent.skip(k).len() && sent.skip(k)[t] == x;
                                if t > 0 {
                                    assert(sent.skip(k + 1)[t - 1] == x);
                                }
                            }
                            if sent.skip(k + 1).contains(x) {
                                let t = choose|t: int| 0 <= t < sent.skip(k + 1).len() && sent.skip(k + 1)[t] == x;
                                assert(sent.skip(k)[t + 1] == x);
                            }
                            if x == sent[k] {
                                assert(sent.skip(k)[0] == x);
                            }
                        }
                        assert(!sent.skip(k + 1).contains(sent[k])) by {
                            if sent.skip(k + 1).contains(sent[k]) {
                                let t = choose|t: int| 0 <= t < sent.skip(k + 1).len() && sent.skip(k + 1)[t] == sent[k];
                                assert(sent[k + 1 + t] == sent[k]);
                            }
                        }
                        assert(collected.contains_key(id)) by {
                            assert(sent.skip(k).contains(id)) by {
                                assert(sent.skip(k)[0] == id);
                            }
                        }
                    }
                    let Tracked(gout) = gout;
                    proof {
                        gouts.tracked_insert(id, gout);
                    }
                    match result {
                        Ok(Some(result)) => {
                            r.push((name, result));
                            proof {
                                replied = replied.push(id);
                                reply_at = reply_at.push(k);
                            }
                        },
                        Ok(None) => {},
                        Err(err) => {
                            // If the unit was removed during the operation, then
                            // ignore the failure. Otherwise, panic because unit
                            // failure is not recoverable. FUTURE: reconsider this
                            // position.
                            if inner.units.contains_key(&id) {
                                unit_failed(err);
                            }
                        },
                    }
                    proof {
                        k = k + 1;
                    }
                },
                None => break,
            }
        }
        let ids = unit_ids_of(&inner);
        let mut i: usize = 0;
        while i < ids.len()
            invariant
                StateUnits::agrees_exactly(collected, inner),
                crate::InnerPred::inv(inner),
                inner.unit_ids() <= ready_set.ids(),
            decreases ids@.len() - i,
        {
            let id = ids[i];
            if !ready_set.contains(id) {
                i += 1;
                continue;
            }
            let ghost before = inner.units@;
            let unit = inner.units.get_mut(&id).unwrap();
            let ghost old_unit = *unit;
            if unit.state == interim_state {
                unit.state = new_state;
            } else {
                expect_state(unit, interim_state, new_state);
            }
            proof {
                assert(inner.units@ == before.insert(id, *unit));
                assert(unit.name == old_unit.name && unit.send == old_unit.send);
            }
            i += 1;
        }
        handle.release(inner);
        (r, Tracked(gouts), Ghost(replied))
    }

    /// The output `out` of unit `id`'s state change (`wait_then_change` of
    /// `state_change_ghost`) is as promised: it names the unit, and its reply
    /// and returned ghost satisfy `state_change_ghost`'s postcondition.
    pub(crate) open spec fn change_done<I, R>(
        collected: Map<u64, (Seq<char>, vstd::resource::Loc)>,
        gins: Map<u64, UnitGhost>,
        input_of: spec_fn(u64) -> Option<I>,
        post: spec_fn(u64, I, R, UnitGhost) -> bool,
        id: u64,
        out: (Arc<str>, u64, ChangeOutput<R>),
    ) -> bool {
        &&& out.0.view_str() == collected[id].0
        &&& out.1 == id
        &&& input_of(id) is None ==> out.2.0 == Ok::<Option<R>, UnitRecvError>(None) && out.2.1@ == gins[id]
        &&& input_of(id) matches Some(i) ==> !(out.2.0 matches Ok(None)) && (out.2.0 matches Ok(Some(r))
            ==> post(id, i, r, out.2.1@))
    }
}

impl StateRequest {
    /// `request`, with input `input` and the reply expectation
    /// `post(id, input, ..)`, may be sent to the unit whose ghost id is `k`,
    /// with ghost `gin`: whatever message `request` builds satisfies the
    /// protocol.
    pub(crate) open spec fn may_send<I, R, Req: FnOnce(Rpc<I, R>) -> StateRequest>(
        request: Req,
        k: vstd::resource::Loc,
        gin: UnitGhost,
        id: u64,
        input: I,
        post: spec_fn(u64, I, R, UnitGhost) -> bool,
    ) -> bool {
        forall|rpc: Rpc<I, R>, msg: StateRequest|
            rpc.spec_input() == input && (forall|r: R, g: UnitGhost|
                #[trigger] rpc.reply_ok(r, g) == post(id, input, r, g)) && #[trigger] request.ensures(
                (rpc,),
                msg,
            ) ==> StateRequest::request_ok(k, msg, gin)
    }
}

/// Sends state change `request` to the unit at `send` with `input`, wrapping
/// the call with a span, and wrapping its error with something more
/// informative. Unverified wrapper for callers that carry no ghost
/// (`quiesce.rs`, `start.rs`).
///
/// `name` is used in tracing and error construction.
#[verifier::external_body]
pub(crate) async fn state_change<I: 'static, R: 'static + Send, Req: FnOnce(Rpc<I, R>) -> StateRequest>(
    name: Arc<str>,
    send: Sender<StateRequest>,
    request: Req,
    input: Option<I>,
) -> Result<Option<R>, UnitRecvError> {
    state_change_ghost(
        name,
        send,
        request,
        input,
        Tracked::assume_new(),
        Ghost::assume_new(),
        Ghost::assume_new(),
    ).await.0
}

/// Sends state change `request` to the unit at `send` with `input`, lending
/// `gin`, wrapping the call with a span, and wrapping its error with
/// something more informative. With no input, sends nothing and returns
/// `gin`. `id` and `post` give the expected reply: `post(id, input, ..)`.
///
/// `name` is used in tracing and error construction.
#[allow(unused_variables)] // ghost parameters are erased at runtime
pub(crate) async fn state_change_ghost<
    I: 'static,
    R: 'static + Send,
    Req: FnOnce(Rpc<I, R>) -> StateRequest,
>(
    name: Arc<str>,
    send: Sender<StateRequest>,
    request: Req,
    input: Option<I>,
    gin: Tracked<UnitGhost>,
    id: Ghost<u64>,
    post: Ghost<spec_fn(u64, I, R, UnitGhost) -> bool>,
) -> (result: ChangeOutput<R>)
    requires
        input matches Some(i) ==> {
            &&& forall|rpc: Rpc<I, R>| #[trigger] request.requires((rpc,))
            &&& StateRequest::may_send(request, send.constant(), gin@, id@, i, post@)
        },
    ensures
        input is None ==> result.0 == Ok::<Option<R>, UnitRecvError>(None) && result.1@ == gin@,
        input matches Some(i) ==> !(result.0 matches Ok(None)) && (result.0 matches Ok(Some(r))
            ==> post@(id@, i, r, result.1@)),
{
    let Some(input) = input else { return (Ok(None), gin) };
    let span = device_state_change_span(&name);
    let (r, gout) = instrumented(call_unit(name, send, request, input, gin, id, post), span).await;
    match r {
        Ok(r) => (Ok(Some(r)), gout),
        Err(err) => (Err(err), gout),
    }
}

/// Sends `request` with `input` to the unit at `send`, lending `gin`, and
/// waits for the reply, logging how long it took. On success, the reply and
/// the ghost that came back satisfy `post(id, input, ..)`.
#[allow(unused_variables)] // ghost parameters are erased at runtime
async fn call_unit<I: 'static, R: 'static + Send, Req: FnOnce(Rpc<I, R>) -> StateRequest>(
    name: Arc<str>,
    send: Sender<StateRequest>,
    request: Req,
    input: I,
    gin: Tracked<UnitGhost>,
    id: Ghost<u64>,
    post: Ghost<spec_fn(u64, I, R, UnitGhost) -> bool>,
) -> (result: (Result<R, UnitRecvError>, Tracked<UnitGhost>))
    requires
        forall|rpc: Rpc<I, R>| #[trigger] request.requires((rpc,)),
        StateRequest::may_send(request, send.constant(), gin@, id@, input, post@),
    ensures
        result.0 matches Ok(r) ==> post@(id@, input, r, result.1@),
{
    let ghost i = input;
    let start = start_timer();
    let r = match call(
        &send,
        request,
        input,
        gin,
        Ghost(|r: R, g: UnitGhost| post@(id@, i, r, g)),
    ).recv().await {
        Ok((r, gout)) => (Ok(r), gout),
        Err(err) => (Err(UnitRecvError { name, source: err }), Tracked(None)),
    };
    log_device_state_change(start);
    r
}

/// Waits for the dependencies `deps` of unit `id` to finish, runs `fut` (the
/// unit's state change), and marks unit `id` done.
async fn wait_then_change<F: Future>(
    ready_set: ReadySet,
    op: &str,
    id: u64,
    deps: Vec<u64>,
    name: Arc<str>,
    fut: F,
) -> (result: (Arc<str>, u64, F::Output))
    ensures
        result.0 == name,
        result.1 == id,
        fut.awaited(),
        result.2 == fut@,
{
    ready_set.wait(op, id, &deps).await;
    let r = fut.await;
    ready_set.done(id, true);
    (name, id, r)
}

} // verus!
