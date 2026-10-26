//! Example for `mutex_t.rs`: units are registered through `&self`, each with a
//! FracGhost half that waits in the mutex until the owner, holding `&mut`,
//! collects it into an ordinary ghost field.
//!
//! Everything in this file is verified.

use crate::mutex_t::*;
use vstd::prelude::*;
use vstd::resource::Loc;
use vstd::resource::frac::FracGhost;
use vstd::rwlock::RwLockPredicate;

verus! {

/// What the mutex holds: the registered units (exec data, with each unit's
/// ghost id), and the halves not yet collected.
pub struct Registry {
    pub ids: Vec<(u64, Ghost<Loc>)>,
    pub inbox: Tracked<Map<u64, FracGhost<u64>>>,
}

pub struct RegistryPred;

impl RwLockPredicate<Registry> for RegistryPred {
    /// Each uncollected half is a half, and belongs to a registered unit.
    open spec fn inv(self, r: Registry) -> bool {
        forall|name: u64| #[trigger] r.inbox@.contains_key(name) ==> {
            &&& r.inbox@[name].frac() == 0.5real
            &&& exists|i: int|
                0 <= i < r.ids@.len() && #[trigger] r.ids@[i].0 == name && r.ids@[i].1@
                    == r.inbox@[name].id()
        }
    }
}

pub struct Units {
    registry: Mutex<Registry, RegistryPred>,
    halves: Tracked<Map<u64, FracGhost<u64>>>,
}

impl Units {
    pub closed spec fn wf(&self) -> bool {
        self.registry.pred() == RegistryPred
    }

    /// The collected halves.
    pub closed spec fn halves(&self) -> Map<u64, FracGhost<u64>> {
        self.halves@
    }

    pub fn new() -> (s: Self)
        ensures
            s.wf(),
            s.halves().is_empty(),
    {
        let r = Registry { ids: Vec::new(), inbox: Tracked(Map::tracked_empty()) };
        Units { registry: Mutex::new(r, Ghost(RegistryPred)), halves: Tracked(Map::tracked_empty()) }
    }

    /// Registers a unit through `&self`; its half waits in the mutex.
    pub fn add(&self, name: u64, half: Tracked<FracGhost<u64>>)
        requires
            self.wf(),
            half@.frac() == 0.5real,
    {
        let (mut r, handle) = self.registry.acquire();
        let ghost old_r = r;
        let ghost id = half@.id();
        let Tracked(half) = half;
        r.ids.push((name, Ghost(id)));
        proof {
            if !r.inbox@.contains_key(name) {
                r.inbox.borrow_mut().tracked_insert(name, half);
            }
            // (A name registered twice keeps its first half.)
            assert forall|n: u64| #[trigger] r.inbox@.contains_key(n) implies {
                &&& r.inbox@[n].frac() == 0.5real
                &&& exists|i: int|
                    0 <= i < r.ids@.len() && #[trigger] r.ids@[i].0 == n && r.ids@[i].1@
                        == r.inbox@[n].id()
            } by {
                if old_r.inbox@.contains_key(n) {
                    let i = choose|i: int|
                        0 <= i < old_r.ids@.len() && #[trigger] old_r.ids@[i].0 == n
                            && old_r.ids@[i].1@ == old_r.inbox@[n].id();
                    assert(r.ids@[i] == old_r.ids@[i]);
                } else {
                    assert(r.ids@[r.ids@.len() - 1].0 == n);
                }
            }
        }
        handle.release(r);
    }

    /// Collects the waiting halves. Halves collected before are kept.
    pub fn collect(&mut self)
        requires
            old(self).wf(),
        ensures
            final(self).wf(),
            forall|n: u64| #[trigger] old(self).halves().contains_key(n) ==> final(self).halves().contains_key(n)
                && final(self).halves()[n] == old(self).halves()[n],
    {
        let (mut r, handle) = self.registry.acquire();
        proof {
            let ghost inbox_keys = r.inbox@.dom();
            let tracked inbox = r.inbox.borrow_mut().tracked_remove_keys(inbox_keys);
            // Halves already collected win over a duplicate in the inbox.
            let tracked mut collected = inbox;
            let ghost before_keys = self.halves@.dom();
            let tracked before = self.halves.borrow_mut().tracked_remove_keys(before_keys);
            collected.tracked_union_prefer_right(before);
            self.halves.borrow_mut().tracked_union_prefer_right(collected);
        }
        handle.release(r);
    }
}

} // verus!
