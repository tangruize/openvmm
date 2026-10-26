// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

// Declarations and the registry's lock invariant; no human review needed.

use crate::Inner;
use crate::InnerPred;
use crate::State;
use crate::StateRequest;
use crate::StateUnits;
use crate::Unit;
use crate::mesh_rpc_t::*;
use crate::mutex_t::MutexPredicate;
use crate::UnitSnapshot;
use mesh::error::RemoteError;
use mesh::rpc::Rpc;
use std::sync::Arc;
use vmcore::save_restore::SavedStateBlob;
use vstd::prelude::*;
use vstd::resource::Loc;
use vstd::resource::frac::FracGhost;

verus! {

// Make the registry's fields visible to Verus. Verus checks that they match
// the real definitions.

#[verifier::external_type_specification]
pub(crate) struct ExInner(Inner);

#[verifier::external_type_specification]
pub(crate) struct ExUnit(Unit);

#[verifier::external_type_specification]
pub(crate) struct ExState(State);

#[verifier::external_type_specification]
pub(crate) struct ExReadySet(crate::ReadySet);

#[verifier::external_type_specification]
#[verifier::external_body]
pub(crate) struct ExReadyState(crate::ReadyState);

#[verifier::external_type_specification]
pub(crate) struct ExUnitRecvError(crate::UnitRecvError);

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExStateTransitionError(crate::StateTransitionError);

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExAnyhowError(anyhow::Error);

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExQuiesceForSaveError(crate::quiesce::QuiesceForSaveError);

#[verifier::external_type_specification]
pub struct ExSavedStateUnit(crate::SavedStateUnit);

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExSensitivityLevel(inspect::SensitivityLevel);

/// A spec view of `Arc<str>` (vstd has views only for `Arc` of sized types).
pub trait ArcStrView {
    spec fn view_str(&self) -> Seq<char>;
}

impl ArcStrView for Arc<str> {
    uninterp spec fn view_str(&self) -> Seq<char>;
}

#[verifier::external_type_specification]
#[verifier::external_body]
pub(crate) struct ExInnerPred(InnerPred);

#[verifier::external_type_specification]
pub struct ExStateRequest(StateRequest);

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExRemoteError(RemoteError);

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExDeferred(inspect::Deferred);

/// The ghosts that travel on a state unit's channel. The channel's constant is
/// the id of the unit's FracGhost.
impl MeshProtocol for StateRequest {
    type Const = Loc;
    /// A half given or lent by the caller.
    type GhostIn = Option<FracGhost<UnitSnapshot>>;
    /// A half returned to the caller.
    type GhostOut = Option<FracGhost<UnitSnapshot>>;
    /// `StateUnits`' half, attached at creation (`unit_channel`).
    type SenderGhost = FracGhost<UnitSnapshot>;
    /// The unit's half, attached at creation.
    type ReceiverGhost = FracGhost<UnitSnapshot>;

    /// The request protocol. `k` is the id of the unit's FracGhost. While a
    /// unit is running it holds the whole FracGhost; while it is stopped, the
    /// caller (`StateUnits`) and the unit each hold one half.
    ///
    /// - `Start`: the caller gives its half; the unit, now holding the whole,
    ///   runs. No ghost comes back, also on `Err` (a unit that fails to
    ///   start ends in a terminal state).
    /// - `Stop`: no ghost is sent; the unit records its state and returns a
    ///   half recording it (not `Unknown`).
    /// - `Save`: the caller lends its half, which must record a known state;
    ///   the unit returns the same half, with a blob that matches it.
    /// - `Restore(blob)`: the caller sends its half; on success the unit
    ///   returns a half recording `Saved(blob@)`.
    /// - `Reset`, `AdvanceTime`: the caller sends its half; the unit returns a
    ///   half (its state may have changed).
    /// - `QuiesceInput`, `ResumeInput`, `Inspect`: no ghost.
    ///
    /// Every reply to `Save`, `Restore`, `Reset`, or `AdvanceTime` returns a
    /// half, also on `Err`, so the caller keeps it.
    open spec fn request_ok(k: Loc, msg: StateRequest, gin: Self::GhostIn) -> bool {
        match msg {
            // The caller's half goes to the unit; nothing comes back.
            StateRequest::Start(rpc) => {
                &&& gin matches Some(h) && h.valid(k, 0.5real)
                &&& forall|r: Result<(), RemoteError>|
                    #[trigger] rpc.reply_ok(r, None::<FracGhost<UnitSnapshot>>)
            },
            // No ghost either way.
            StateRequest::QuiesceInput(rpc) => gin is None && forall|r: Result<(), RemoteError>|
                #[trigger] rpc.reply_ok(r, None::<FracGhost<UnitSnapshot>>),
            StateRequest::ResumeInput(rpc) => gin is None && forall|r: Result<(), RemoteError>|
                #[trigger] rpc.reply_ok(r, None::<FracGhost<UnitSnapshot>>),
            StateRequest::Inspect(_) => gin is None,
            // The unit returns a half of the whole it held while running.
            StateRequest::Stop(rpc) => {
                &&& gin is None
                &&& forall|r: (), gout: Self::GhostOut|
                    gout matches Some(h) && h.valid(k, 0.5real) && !(h@ is Unknown)
                        ==> #[trigger] rpc.reply_ok(r, gout)
            },
            StateRequest::Save(rpc) => {
                &&& gin matches Some(h) && h.valid(k, 0.5real)
                &&& !(gin->Some_0@ is Unknown)
                &&& forall|r: Result<Option<SavedStateBlob>, RemoteError>, gout: Self::GhostOut|
                    gout == gin && match r {
                        Ok(Some(blob)) => gin->Some_0@ == UnitSnapshot::Saved(blob@),
                        Ok(None) => gin->Some_0@ == UnitSnapshot::NoSavedState,
                        Err(_) => true,
                    } ==> #[trigger] rpc.reply_ok(r, gout)
            },
            StateRequest::Restore(rpc) => {
                &&& gin matches Some(h) && h.valid(k, 0.5real)
                &&& forall|r: Result<(), RemoteError>, gout: Self::GhostOut|
                    gout matches Some(h) && h.valid(k, 0.5real) && (r is Ok ==> h@
                        == UnitSnapshot::Saved(rpc.spec_input()@)) ==> #[trigger] rpc.reply_ok(r, gout)
            },
            // A half back, with any value.
            StateRequest::Reset(rpc) => {
                &&& gin matches Some(h) && h.valid(k, 0.5real)
                &&& forall|r: Result<(), RemoteError>, gout: Self::GhostOut|
                    gout matches Some(h) && h.valid(k, 0.5real) ==> #[trigger] rpc.reply_ok(r, gout)
            },
            StateRequest::AdvanceTime(rpc) => {
                &&& gin matches Some(h) && h.valid(k, 0.5real)
                &&& forall|r: Result<(), RemoteError>, gout: Self::GhostOut|
                    gout matches Some(h) && h.valid(k, 0.5real) ==> #[trigger] rpc.reply_ok(r, gout)
            },
        }
    }
}

impl View for crate::ReadySet {
    /// The units in the set.
    type V = Set<u64>;

    closed spec fn view(&self) -> Set<u64> {
        self.0@.dom()
    }
}

impl Inner {
    /// The registered units. (Proof plumbing for `Inner::ready_set`'s
    /// assumed specification.)
    pub(crate) closed spec fn unit_ids(&self) -> Set<u64> {
        self.units@.dom()
    }
}

impl MutexPredicate<Inner> for InnerPred {
    /// Each registered unit's sender, until `StateUnits` collects its half,
    /// carries a half of that unit's FracGhost (the channel's constant is its
    /// id), still recording `Unknown`; and registered units have distinct
    /// names.
    ///
    /// `UnitBuilder::build` (unverified) keeps this, given that its caller
    /// passes a sender from `unit_channel` (ASSUMPTION).
    open(crate) spec fn inv(v: Inner) -> bool {
        &&& forall|id: u64| #[trigger] v.units@.contains_key(id) ==> {
            let send = v.units@[id].send;
            send.attached() matches Some(h) ==> h.valid(send.constant(), 0.5real) && h@
                == UnitSnapshot::Unknown
        }
        // Registered units have distinct names (`build` checks `names`).
        &&& forall|a: u64, b: u64|
            #[trigger] v.units@.contains_key(a) && #[trigger] v.units@.contains_key(b) && a != b
                ==> v.units@[a].name.view_str() != v.units@[b].name.view_str()
    }
}

/// `StateUnits`' ghost state: the units collected so far and the half held
/// for each.
pub tracked struct UnitHalves {
    /// The half held for each collected unit, by unit id.
    pub tracked halves: Map<u64, FracGhost<UnitSnapshot>>,
    /// The collected units, by id: name, the id of the unit's FracGhost (its
    /// channel's constant), and whether it advances time
    /// (`UnitBuilder::advances_time`).
    pub ghost registry: Map<u64, (Seq<char>, Loc, bool)>,
}

/// The ghost state of a new `StateUnits`: no unit collected.
pub(crate) fn no_halves() -> (result: Tracked<UnitHalves>)
    ensures
        result@.halves == Map::<u64, FracGhost<UnitSnapshot>>::empty(),
        result@.registry == Map::<u64, (Seq<char>, Loc, bool)>::empty(),
{
    Tracked(UnitHalves { halves: Map::tracked_empty(), registry: Map::empty() })
}

impl StateUnits {
    /// If the collected units have distinct names, the view's invariant
    /// holds (the inventory, built from the collected ids, lists each unit
    /// once).
    ///
    /// PROOF PENDING (`external_body`): to be proved from vstd's
    /// `to_sorted_seq` lemmas. Listed in `SNAPSHOT-SPEC-TRUSTED.md`.
    #[verifier::external_body]
    pub(crate) proof fn lemma_view_inv(&self)
        requires
            forall|a: u64, b: u64|
                #[trigger] self.units_ghost@.registry.contains_key(a)
                    && #[trigger] self.units_ghost@.registry.contains_key(b) && a != b
                    ==> self.units_ghost@.registry[a].0 != self.units_ghost@.registry[b].0,
        ensures
            self@.inv(),
    {
    }

    /// The collected units, by id: name and the id of the unit's FracGhost.
    /// (Proof plumbing for the registry axioms.)
    pub(crate) closed spec fn collected(&self) -> Map<u64, (Seq<char>, Loc, bool)> {
        self.units_ghost@.registry
    }

    /// The registry as seen under the lock agrees with the collected units:
    /// each collected unit is registered with the same name and channel, and
    /// each registered unit not yet collected still has its half attached.
    pub(crate) open spec fn agrees_with(collected: Map<u64, (Seq<char>, Loc, bool)>, inner: Inner) -> bool {
        &&& forall|id: u64| #[trigger] collected.contains_key(id) ==> {
            &&& inner.units@.contains_key(id)
            &&& inner.units@[id].name.view_str() == collected[id].0
            &&& inner.units@[id].send.constant() == collected[id].1
            &&& inner.units@[id].advances_time == collected[id].2
        }
        &&& forall|id: u64|
            #[trigger] inner.units@.contains_key(id) && !collected.contains_key(id)
                ==> inner.units@[id].send.attached() is Some
    }

    /// The registry as seen under the lock holds exactly the collected units,
    /// with the same names and channels.
    pub(crate) open spec fn agrees_exactly(collected: Map<u64, (Seq<char>, Loc, bool)>, inner: Inner) -> bool {
        &&& inner.units@.dom() == collected.dom()
        &&& forall|id: u64| #[trigger] collected.contains_key(id) ==> {
            &&& inner.units@[id].name.view_str() == collected[id].0
            &&& inner.units@[id].send.constant() == collected[id].1
            &&& inner.units@[id].advances_time == collected[id].2
        }
    }

    /// Collects the halves still attached to registered units' senders.
    /// Halves already held are kept. Ghost only: call sites are Verus-only.
    pub fn collect_halves(&mut self)
        requires
            old(self).wf(),
        ensures
            final(self).wf(),
    {
        let (mut inner, handle) = self.lock_registry();
        let mut ids: Vec<u64> = Vec::new();
        for id in inner.units.keys() {
            ids.push(*id);
        }
        // The units collected now, and their halves; merged in after the
        // registry is released (the lock borrows `self`).
        let ghost old_registry = self.units_ghost@.registry;
        let ghost mut registry = old_registry;
        let tracked mut new_halves = Map::<u64, FracGhost<UnitSnapshot>>::tracked_empty();
        let mut i: usize = 0;
        while i < ids.len()
            invariant
                InnerPred::inv(inner),
                StateUnits::agrees_with(registry, inner),
                old_registry <= registry,
                new_halves.dom() == registry.dom().difference(old_registry.dom()),
                forall|id: u64| #[trigger] new_halves.contains_key(id)
                    ==> new_halves[id].valid(registry[id].1, 0.5real),
                forall|a: u64, b: u64|
                    #[trigger] registry.contains_key(a) && #[trigger] registry.contains_key(b) && a != b
                        ==> registry[a].0 != registry[b].0,
                old_registry == self.units_ghost@.registry,
            decreases ids.len() - i,
        {
            let id = ids[i];
            if let Some(unit) = inner.units.get_mut(&id) {
                let ghost before = *unit;
                let Tracked(half) = take_sender_ghost(&mut unit.send);
                proof {
                    if !registry.contains_key(id) {
                        // Not yet collected: the half is still attached.
                        let tracked half = half.tracked_unwrap();
                        let entry = (
                            before.name.view_str(),
                            before.send.constant(),
                            before.advances_time,
                        );
                        assert forall|other: u64| #[trigger] registry.contains_key(other)
                            implies registry[other].0 != entry.0 by {
                            assert(inner.units@.contains_key(other));
                            assert(other != id);
                        }
                        let ghost prev = registry;
                        new_halves.tracked_insert(id, half);
                        registry = registry.insert(id, entry);
                        assert(old_registry <= registry) by {
                            assert forall|k: u64| #[trigger] old_registry.contains_key(k)
                                implies registry.contains_key(k) && registry[k] == old_registry[k] by {
                                assert(prev.contains_key(k));
                            }
                        }
                        assert(new_halves.dom() =~= registry.dom().difference(old_registry.dom()));
                    }
                }
            }
            i += 1;
        }
        handle.release(inner);
        proof {
            let ghost old_halves = self.units_ghost@.halves;
            self.units_ghost.borrow_mut().halves.tracked_union_prefer_right(new_halves);
            self.units_ghost.borrow_mut().registry = registry;
            let ghost halves = self.units_ghost@.halves;
            assert(halves.dom() =~= old_halves.dom().union(new_halves.dom()));
            assert(halves.dom() <= registry.dom());
            if !self.running {
                assert(halves.dom() =~= registry.dom());
            }
            assert forall|id: u64| #[trigger] halves.contains_key(id)
                implies halves[id].valid(registry[id].1, 0.5real) by {
                if new_halves.contains_key(id) {
                } else {
                    assert(old_halves.contains_key(id));
                    assert(old_registry.contains_key(id));
                    assert(halves[id] == old_halves[id]);
                }
            }
            assert forall|id: u64| #[trigger] old_halves.contains_key(id)
                implies halves[id] == old_halves[id] by {
                assert(!new_halves.contains_key(id));
            }
            self.lemma_view_inv();
        }
    }
}

// Examples showing that the request protocol can be met, by a caller and by
// a unit. Not production code.

/// A caller lends its half for `Save` and learns that the reply's blob
/// matches what the half records.
pub(crate) async fn example_save_call(
    send: &mesh::Sender<StateRequest>,
    half: Tracked<FracGhost<UnitSnapshot>>,
) -> (result: Option<(Option<SavedStateBlob>, Tracked<FracGhost<UnitSnapshot>>)>)
    requires
        half@.valid(send.constant(), 0.5real),
        !(half@@ is Unknown),
    ensures
        result matches Some((saved, h)) ==> h@ == half@ && match saved {
            Some(blob) => half@@ == UnitSnapshot::Saved(blob@),
            None => half@@ == UnitSnapshot::NoSavedState,
        },
{
    let ghost k = send.constant();
    let ghost lent = half@;
    let Tracked(half) = half;
    let pending = call(
        send,
        |rpc: Rpc<(), Result<Option<SavedStateBlob>, RemoteError>>| -> (msg: StateRequest)
            ensures
                msg == StateRequest::Save(rpc),
            { StateRequest::Save(rpc) },
        (),
        Tracked(Some(half)),
        Ghost(
            |r: Result<Option<SavedStateBlob>, RemoteError>, gout: Option<FracGhost<UnitSnapshot>>|
                gout == Some(lent) && match r {
                    Ok(Some(blob)) => lent@ == UnitSnapshot::Saved(blob@),
                    Ok(None) => lent@ == UnitSnapshot::NoSavedState,
                    Err(_) => true,
                },
        ),
    );
    match pending.recv().await {
        Ok((Ok(saved), gout)) => {
            let Tracked(gout) = gout;
            Some((saved, Tracked(gout.tracked_unwrap())))
        },
        _ => None,
    }
}

/// A unit whose save returns `saved`, following the rule: while running it
/// holds its whole FracGhost, and while stopped one half.
pub(crate) struct ExampleUnit {
    pub(crate) saved: Option<SavedStateBlob>,
    /// The unit's share of its FracGhost.
    pub(crate) frac: Tracked<FracGhost<UnitSnapshot>>,
    /// Whether the unit is running.
    pub(crate) running: Ghost<bool>,
    /// The id of the unit's FracGhost (its channel's constant).
    pub(crate) id: Ghost<Loc>,
}

impl View for ExampleUnit {
    /// What the unit's save returns.
    type V = UnitSnapshot;

    closed spec fn view(&self) -> UnitSnapshot {
        match self.saved {
            Some(blob) => UnitSnapshot::Saved(blob@),
            None => UnitSnapshot::NoSavedState,
        }
    }
}

impl ExampleUnit {
    /// The ghost belongs to the unit's channel; the unit holds the whole of
    /// it while running and one half while stopped; unless it is still
    /// `Unknown`, it records what the unit's save returns.
    pub(crate) closed spec fn wf(&self) -> bool {
        &&& self.frac@.valid(self.id@, if self.running@ { 1real } else { 0.5real })
        &&& !(self.frac@@ is Unknown) ==> self.frac@@ == self@
    }

    /// Changes the unit's state, as an I/O handler would. Allowed only while
    /// running, when the unit holds the whole ghost and can update it.
    pub(crate) fn set(&mut self, saved: Option<SavedStateBlob>)
        requires
            old(self).wf(),
            old(self).running@,
        ensures
            final(self).wf(),
            final(self).running == old(self).running,
            final(self).id == old(self).id,
    {
        self.saved = saved;
        proof {
            let ghost now = self@;
            self.frac.borrow_mut().update(now);
        }
    }

    /// Answers `Start`, `Stop`, `Save`, and `Restore` by the protocol.
    ///
    /// Requires that `Stop` arrives only while the unit runs. This is the
    /// trusted fact that `StateUnits`' recorded unit states are the units'
    /// real states (`SNAPSHOT-SPEC-DESIGN.md`); every other request proves
    /// the unit's running state from the ghost it carries.
    pub(crate) fn handle(&mut self, msg: StateRequest, gin: Tracked<Option<FracGhost<UnitSnapshot>>>)
        requires
            old(self).wf(),
            StateRequest::request_ok(old(self).id@, msg, gin@),
            msg is Stop ==> old(self).running@,
        ensures
            final(self).wf(),
            final(self).id == old(self).id,
    {
        let Tracked(gin) = gin;
        match msg {
            StateRequest::Start(rpc) => {
                let tracked given = gin.tracked_unwrap();
                proof {
                    // With the caller's half the unit holds the whole: it was
                    // stopped (running, it would now hold more than 1).
                    self.frac.borrow_mut().combine(given);
                    self.frac.borrow().bounded();
                    self.running = Ghost(true);
                }
                complete(rpc, Ok(()), Tracked(None::<FracGhost<UnitSnapshot>>));
            },
            StateRequest::Stop(rpc) => {
                let tracked mut half;
                proof {
                    // Running, so the whole: record the state, then split.
                    let ghost now = self@;
                    self.frac.borrow_mut().update(now);
                    half = self.frac.borrow_mut().split();
                    self.running = Ghost(false);
                }
                complete(rpc, (), Tracked(Some(half)));
            },
            StateRequest::Save(rpc) => {
                let tracked lent = gin.tracked_unwrap();
                proof {
                    // The lent half agrees with ours, which records the state.
                    self.frac.borrow().agree(&lent);
                }
                let saved = match &self.saved {
                    Some(blob) => Some(blob.clone()),
                    None => None,
                };
                complete(rpc, Ok(saved), Tracked(Some(lent)));
            },
            StateRequest::Restore(rpc) => {
                let tracked mut lent = gin.tracked_unwrap();
                let (blob, rpc) = split::<SavedStateBlob, Result<(), RemoteError>, Option<FracGhost<UnitSnapshot>>>(rpc);
                self.saved = Some(blob);
                proof {
                    // With the lent half the unit holds the whole: it is
                    // stopped. Record the restored state, then split again.
                    self.frac.borrow_mut().combine(lent);
                    self.frac.borrow().bounded();
                    let ghost now = self@;
                    self.frac.borrow_mut().update(now);
                    lent = self.frac.borrow_mut().split();
                }
                complete(rpc, Ok(()), Tracked(Some(lent)));
            },
            _ => {
                // Other requests are not part of this example. (Their
                // messages are dropped; for those carrying the caller's half,
                // the caller gets an error and the half is lost.)
            },
        }
    }
}

} // verus!
