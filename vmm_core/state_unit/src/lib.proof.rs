// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

// Declarations and the registry's lock invariant; no human review needed.

use super::Inner;
use super::InnerPred;
use super::State;
use super::StateRequest;
use super::StateUnits;
use super::Unit;
use super::mesh_rpc_t::*;
use super::mutex_t::MutexPredicate;
use super::spec::UnitSnapshot;
use std::sync::Arc;
use vstd::prelude::*;
use vstd::resource::Loc;
use vstd::resource::frac::FracGhost;

verus! {

// Make the registry's fields visible to Verus. Verus checks that they match
// the real definitions.

#[verifier::external_type_specification]
pub(super) struct ExInner(Inner);

#[verifier::external_type_specification]
pub(super) struct ExUnit(Unit);

#[verifier::external_type_specification]
#[verifier::external_body]
pub(super) struct ExState(State);

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
pub(super) struct ExInnerPred(InnerPred);

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExStateRequest(StateRequest);

/// The ghosts that travel on a state unit's channel. The channel's constant is
/// the id of the unit's FracGhost.
impl MeshProtocol for StateRequest {
    type Const = Loc;
    /// A half lent or returned by the caller.
    type GhostIn = Option<FracGhost<UnitSnapshot>>;
    /// A half returned to the caller.
    type GhostOut = Option<FracGhost<UnitSnapshot>>;
    /// `StateUnits`' half, attached at creation (`unit_channel`).
    type SenderGhost = FracGhost<UnitSnapshot>;
    /// The unit's half, attached at creation.
    type ReceiverGhost = FracGhost<UnitSnapshot>;

    /// No rules yet: they come with the request protocol.
    open spec fn request_ok(k: Loc, msg: StateRequest, gin: Self::GhostIn) -> bool {
        true
    }
}

impl MutexPredicate<Inner> for InnerPred {
    /// Each registered unit's sender, until `StateUnits` collects its half,
    /// carries a half of that unit's FracGhost (the channel's constant is its
    /// id), still recording `Unknown`.
    ///
    /// `UnitBuilder::build` (unverified) keeps this, given that its caller
    /// passes a sender from `unit_channel` (ASSUMPTION).
    open(crate) spec fn inv(v: Inner) -> bool {
        forall|id: u64| #[trigger] v.units@.contains_key(id) ==> {
            let send = v.units@[id].send;
            send.attached() matches Some(h) ==> h.valid(send.constant(), 0.5real) && h@
                == UnitSnapshot::Unknown
        }
    }
}

/// The halves of a new `StateUnits`: none.
pub(super) fn no_halves() -> (halves: Tracked<Map<Seq<char>, FracGhost<UnitSnapshot>>>)
    ensures
        halves@ == Map::<Seq<char>, FracGhost<UnitSnapshot>>::empty(),
{
    Tracked(Map::tracked_empty())
}

impl StateUnits {
    /// Collects the halves still attached to registered units' senders.
    /// Halves already held are kept. Ghost only: call sites are Verus-only.
    pub(super) fn collect_halves(&mut self)
        ensures
            forall|name: Seq<char>| #[trigger] old(self).halves@.contains_key(name)
                ==> final(self).halves@.contains_key(name) && final(self).halves@[name]
                == old(self).halves@[name],
            final(self).running == old(self).running,
    {
        let (mut inner, handle) = self.inner.acquire();
        let mut ids: Vec<u64> = Vec::new();
        for id in inner.units.keys() {
            ids.push(*id);
        }
        let mut i: usize = 0;
        while i < ids.len()
            invariant
                InnerPred::inv(inner),
                forall|name: Seq<char>| #[trigger] old(self).halves@.contains_key(name)
                    ==> self.halves@.contains_key(name) && self.halves@[name] == old(self).halves@[name],
                self.running == old(self).running,
            decreases ids.len() - i,
        {
            if let Some(unit) = inner.units.get_mut(&ids[i]) {
                let Tracked(half) = take_sender_ghost(&mut unit.send);
                proof {
                    if half is Some && !self.halves@.contains_key(unit.name.view_str()) {
                        self.halves.borrow_mut().tracked_insert(unit.name.view_str(), half.tracked_unwrap());
                    }
                }
            }
            i += 1;
        }
        handle.release(inner);
    }
}

} // verus!
