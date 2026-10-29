// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

//! The two changes to the registry's set of units, verified against the
//! registry's lock invariant (`InnerPred`, `lib.proof.rs`): registering a
//! unit (from `UnitBuilder::build`) and removing one (from `UnitHandle`).
//! Their callers are unverified; in a normal build the ghost parts are
//! erased.

use crate::Inner;
#[cfg(verus_keep_ghost)]
use crate::InnerPred;
use crate::Unit;
#[cfg(verus_keep_ghost)]
use crate::mutex_t::MutexPredicate;
#[cfg(verus_keep_ghost)]
use mesh_rpc_verus::SenderAdditionalSpecFns;
use mesh_rpc_verus::take_sender_ghost;
use vstd::prelude::*;

verus! {

impl Inner {
    /// Registers `unit` under `id`, keeping the half attached to its sender
    /// (from `unit_channel`) with the registry until `StateUnits` collects
    /// it. The caller (`UnitBuilder::build`) checks that the name is new.
    pub(crate) fn insert_unit(&mut self, id: u64, unit: Unit)
        requires
            InnerPred::inv(*old(self)),
            !old(self).units@.contains_key(id),
            forall|other: u64| #[trigger] old(self).units@.contains_key(other)
                ==> old(self).units@[other].name@ != unit.name@,
            unit.send.attached() matches Some(h) && h.valid(unit.send.constant(), 0.5real),
        ensures
            InnerPred::inv(*final(self)),
            InnerPred::key(*final(self)) == InnerPred::key(*old(self)),
    {
        let mut unit = unit;
        let Tracked(half) = take_sender_ghost(&mut unit.send);
        let ghost entry = (unit.name@, unit.send.constant(), unit.advances_time);
        let ghost old_units = self.units@;
        self.units.insert(id, unit);
        proof {
            let tracked rg = self.registry_ghost.borrow_mut();
            rg.entries = rg.entries.insert(id, entry);
            rg.halves.tracked_insert(id, half.tracked_unwrap());
            assert(self.units@ == old_units.insert(id, unit));
            assert(rg.entries.dom() =~= self.units@.dom());
        }
    }

    /// Removes the unit `id`, if registered, dropping its entry and any half
    /// kept for it. Returns whether it was registered.
    pub(crate) fn remove_unit(&mut self, id: u64) -> (removed: bool)
        requires
            InnerPred::inv(*old(self)),
        ensures
            InnerPred::inv(*final(self)),
            InnerPred::key(*final(self)) == InnerPred::key(*old(self)),
            removed == old(self).units@.contains_key(id),
    {
        let ghost old_units = self.units@;
        let removed = self.units.remove(&id).is_some();
        proof {
            let tracked rg = self.registry_ghost.borrow_mut();
            rg.entries = rg.entries.remove(id);
            if rg.halves.contains_key(id) {
                let tracked _dropped = rg.halves.tracked_remove(id);
            }
            assert(self.units@ == old_units.remove(id));
            assert(rg.entries.dom() =~= self.units@.dom());
            assert(rg.halves.dom() <= rg.entries.dom());
        }
        removed
    }
}

} // verus!
