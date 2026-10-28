// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

// Declarations and the registry's lock invariant; no human review needed.

use crate::Inner;
use crate::InnerPred;
use crate::State;
use crate::StateRequest;
use crate::StateUnits;
use crate::Unit;
use crate::UnitSnapshot;
use crate::mutex_t::MutexPredicate;
use mesh::error::RemoteError;
use mesh::rpc::Rpc;
use mesh_rpc_verus::*;
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

// (`anyhow::Error` is declared in vmcore, `save_restore.proof.rs`.)

#[verifier::external_type_specification]
pub struct ExSavedStateUnit(crate::SavedStateUnit);

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExQuiesceForSaveError(crate::quiesce::QuiesceForSaveError);

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExSensitivityLevel(inspect::SensitivityLevel);

// (`inspect::InspectMut` and `inspect::Inspect` are declared in vmcore,
// `save_restore.proof.rs`.)

/// `InspectConcurrently`, declared to Verus (no methods).
#[verifier::external_trait_specification]
pub trait ExInspectConcurrently {
    type ExternalTraitSpecificationFor: crate::InspectConcurrently;

    type Inspector: inspect::Inspect;
}

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

// (`mesh::error::RemoteError` is declared in `mesh_rpc_verus`, and
// `inspect::Deferred` in vmcore.)

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
    /// The id of the shared record of the collected units (`StateUnits`
    /// holds the other half of it).
    type Key = Loc;

    open(crate) spec fn key(v: Inner) -> Loc {
        v.registry_ghost@.collected.id()
    }

    /// The registry's ghost state matches its units: each registered unit
    /// has an entry (name, FracGhost id, `advances_time`); registered units
    /// have distinct names; the halves kept here belong to registered units;
    /// and each registered unit is either collected by `StateUnits`, with
    /// its entry unchanged since, or has its half kept here (registered
    /// since the last collection).
    ///
    /// Registration and removal keep it (`Inner::insert_unit`,
    /// `Inner::remove_unit`, verified). Other unverified code that locks the
    /// registry is trusted to keep it (it changes only units' states and
    /// dependency lists, which the invariant does not mention).
    open(crate) spec fn inv(v: Inner) -> bool {
        let g = v.registry_ghost@;
        &&& g.entries.dom() == v.units@.dom()
        &&& forall|id: u64| #[trigger] v.units@.contains_key(id) ==> g.entries[id] == (
            v.units@[id].name.view_str(),
            v.units@[id].send.constant(),
            v.units@[id].advances_time,
        )
        // Registered units have distinct names (`build` checks `names`).
        &&& forall|a: u64, b: u64|
            #[trigger] v.units@.contains_key(a) && #[trigger] v.units@.contains_key(b) && a != b
                ==> v.units@[a].name.view_str() != v.units@[b].name.view_str()
        &&& g.collected.frac() == 0.5real
        &&& g.halves.dom() <= g.entries.dom()
        &&& forall|id: u64| #[trigger] g.halves.contains_key(id)
            ==> g.halves[id].valid(g.entries[id].1, 0.5real)
        &&& forall|id: u64| #[trigger] g.entries.contains_key(id) ==> {
            ||| g.collected@.contains_key(id) && g.collected@[id] == g.entries[id]
            ||| g.halves.contains_key(id)
        }
    }
}

/// The registry's ghost state, kept with the units behind the registry's
/// mutex (`Inner::registry_ghost`).
pub tracked struct RegistryGhost {
    /// Each registered unit's entry, by id: name, the id of the unit's
    /// FracGhost (its channel's constant), and whether it advances time.
    pub ghost entries: Map<u64, (Seq<char>, Loc, bool)>,
    /// The halves of the units registered since `StateUnits` last collected.
    pub tracked halves: Map<u64, FracGhost<UnitSnapshot>>,
    /// The registry's half of the shared record of the units `StateUnits`
    /// has collected, with their entries. Both halves change together, when
    /// `StateUnits` collects (`collect_halves`).
    pub tracked collected: FracGhost<Map<u64, (Seq<char>, Loc, bool)>>,
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
    /// `StateUnits`' half of the shared record of the collected units
    /// (the registry holds the other).
    pub tracked collected: FracGhost<Map<u64, (Seq<char>, Loc, bool)>>,
}

/// The ghost state of a new `StateUnits` and of its registry: no unit
/// registered or collected, and the two halves of the shared record.
pub(crate) fn new_ghosts() -> (result: (Tracked<RegistryGhost>, Tracked<UnitHalves>))
    ensures
        result.0@.entries == Map::<u64, (Seq<char>, Loc, bool)>::empty(),
        result.0@.halves == Map::<u64, FracGhost<UnitSnapshot>>::empty(),
        result.0@.collected.frac() == 0.5real,
        result.0@.collected@ == Map::<u64, (Seq<char>, Loc, bool)>::empty(),
        result.1@.halves == Map::<u64, FracGhost<UnitSnapshot>>::empty(),
        result.1@.registry == Map::<u64, (Seq<char>, Loc, bool)>::empty(),
        result.1@.collected.valid(result.0@.collected.id(), 0.5real),
        result.1@.collected@ == Map::<u64, (Seq<char>, Loc, bool)>::empty(),
{
    let tracked mut mine;
    let tracked theirs;
    proof {
        mine = FracGhost::new(Map::<u64, (Seq<char>, Loc, bool)>::empty());
        theirs = mine.split();
    }
    (
        Tracked(RegistryGhost { entries: Map::empty(), halves: Map::tracked_empty(), collected: theirs }),
        Tracked(UnitHalves { halves: Map::tracked_empty(), registry: Map::empty(), collected: mine }),
    )
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

    /// Collects the halves of the units registered since the last
    /// collection, and forgets the units removed since: afterwards
    /// `StateUnits`' ghost state covers exactly the registered units. Ghost
    /// only (its call sites are Verus-only); it locks the registry.
    pub fn collect_halves(&mut self)
        requires
            old(self).wf(),
        ensures
            final(self).wf(),
            final(self)@.running == old(self)@.running,
    {
        let (mut inner, handle) = self.inner.acquire();
        proof {
            let tracked ug = self.units_ghost.borrow_mut();
            let tracked rg = inner.registry_ghost.borrow_mut();
            let ghost old_registry = ug.registry;
            let ghost old_halves = ug.halves;
            let ghost pending_dom = rg.halves.dom();
            let ghost entries = rg.entries;
            // The two halves of the shared record agree: the registry's
            // record of the collected units is `StateUnits`' registry.
            ug.collected.agree(&rg.collected);
            // Keep the halves of the units still registered (those of
            // removed units are dropped), and take those of the units
            // registered since.
            let tracked mut halves = ug.halves.tracked_remove_keys(
                old_halves.dom().intersect(entries.dom()),
            );
            let tracked pending = rg.halves.tracked_remove_keys(pending_dom);
            halves.tracked_union_prefer_right(pending);
            vstd::modes::tracked_swap(&mut ug.halves, &mut halves);
            // Record the registered units as collected, in both halves.
            ug.collected.update_with(&mut rg.collected, entries);
            ug.registry = entries;

            assert(rg.halves.dom() =~= Set::<u64>::empty());
            assert(ug.halves.dom() <= entries.dom());
            assert forall|id: u64| #[trigger] ug.halves.contains_key(id)
                implies ug.halves[id].valid(entries[id].1, 0.5real) by {
                if !pending.contains_key(id) {
                    // Held before, and collected with this entry.
                    assert(old_halves.contains_key(id));
                    assert(old_registry.contains_key(id));
                    assert(old_registry[id] == entries[id]);
                }
            }
            if !self.running {
                assert forall|id: u64| #[trigger] entries.contains_key(id)
                    implies ug.halves.contains_key(id) by {
                    if !pending.contains_key(id) {
                        assert(old_registry.contains_key(id));
                        assert(old_halves.contains_key(id));
                    }
                }
                assert(ug.halves.dom() =~= entries.dom());
            }
            assert forall|a: u64, b: u64|
                #[trigger] entries.contains_key(a) && #[trigger] entries.contains_key(b) && a != b
                    implies entries[a].0 != entries[b].0 by {
                assert(inner.units@.contains_key(a) && inner.units@.contains_key(b));
            }
        }
        handle.release(inner);
        proof {
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
