use std::sync::Arc;
use vstd::atomic::*;
use vstd::invariant::*;
use vstd::prelude::*;
use vstd::resource::frac::FracGhost;
use vstd::resource::Loc;
use vstd::shared::Shared;

verus! {

////////////////////////////////////////////////////////////////////////////////
// Count to 2 (after Chris Hawblitzel's example).
//
// Two threads each add 1 to a shared counter that starts at 0. When both are done, the
// parent wants to prove: the counter is exactly 2.
//
// Global protocol: an atomic invariant keeps the rule  counter == inc_a + inc_b,  where
// inc_a and inc_b are ghost increments: inc_a == 1 means "thread A has added", and
// inc_b == 1 means "thread B has added".
//
// Local actions: thread A is in charge of `inc_a`, thread B of `inc_b`. Each ghost
// increment is a `FracGhost` value, and it can only be changed by someone who holds ALL of
// it (fraction 1). So each one is split in two halves: the thread holds one half
// (`inc_a_thread`) and the atomic invariant keeps the other (`inc_a_inv`). Only inside the
// atomic invariant, where a thread can combine its half with the invariant's half, can
// the value change. The thread does that in the same atomic step as its real add.
////////////////////////////////////////////////////////////////////////////////

// The ids to match permissions.
// (`Loc` is vstd's type for the id of a ghost resource: it is what `.id()` returns.)
pub struct CountK {
    pub counter_id: int,
    pub inc_a_id: Loc,
    pub inc_b_id: Loc,
}

// The permission to the real counter, and the invariant's
// half of each of the two ghost increments.
pub struct CountV {
    pub counter_perm: PermissionU32,
    pub inc_a_inv: FracGhost<int>,
    pub inc_b_inv: FracGhost<int>,
}

// The label that names the invariant rule.
pub struct CountInv;

impl InvariantPredicate<CountK, CountV> for CountInv {
    open spec fn inv(k: CountK, v: CountV) -> bool {
        // the permission and the halves belong to the right counter and the right ghost increments
        &&& v.counter_perm.id() == k.counter_id
        &&& v.inc_a_inv.id() == k.inc_a_id
        &&& v.inc_b_inv.id() == k.inc_b_id
        // exactly half of each ghost increment is here (the other half is out with a thread)
        &&& v.inc_a_inv.frac() == 0.5real
        &&& v.inc_b_inv.frac() == 0.5real
        // inc_a and inc_b are each 0 or 1
        &&& 0 <= v.inc_a_inv@ < 2
        &&& 0 <= v.inc_b_inv@ < 2
        // the counter equals inc_a + inc_b
        &&& v.counter_perm.value() == v.inc_a_inv@ + v.inc_b_inv@
    }
}

type Inv = AtomicInvariant<CountK, CountV, CountInv>;

fn main() {
    // The real counter, shared between the threads.
    let (ptr, Tracked(counter_perm)) = PAtomicU32::new(0);
    let ptr = Arc::new(ptr);

    // Set up Thread A's increment ghost; split it to two halves, one to be kept in
    // the atomic invariant and one to be passed directly to thread A
    let tracked mut inc_a_inv = FracGhost::<int>::new(0);
    let tracked inc_a_thread = inc_a_inv.split();
    let ghost inc_a_id = inc_a_thread.id();

    // Set up Thread B's increment ghost similarly
    let tracked mut inc_b_inv = FracGhost::<int>::new(0);
    let tracked inc_b_thread = inc_b_inv.split();
    let ghost inc_b_id = inc_b_thread.id();

    // The shared atomic invariant.
    let tracked inv = Inv::new(
        CountK { counter_id: ptr.id(), inc_a_id: inc_a_inv.id(), inc_b_id: inc_b_inv.id() },
        CountV { counter_perm, inc_a_inv, inc_b_inv },
        0,
    );

    // Clone the invariant and the atomic counter to pass to each thread
    let tracked shared_inv = Shared::new(inv);
    let tracked inv_for_a = shared_inv.clone();
    let ptr_for_a = ptr.clone();
    let tracked inv_for_b = shared_inv.clone();
    let ptr_for_b = ptr.clone();

    // Thread A: add 1 to the real counter and, in the same atomic step, set inc_a from 0 to 1.
    let thread_a = vstd::thread::spawn(
        move || -> (r: Tracked<FracGhost<int>>)
            ensures
                r@.id() == inc_a_id,
                r@.frac() == 0.5real,
                r@@ == 1,
            {
                // (a closure cannot change a captured variable, so make a `mut` copy)
                let tracked mut inc_a_thread = inc_a_thread;
                open_atomic_invariant!(inv_for_a.borrow() => v => {
                    // the real increment
                    ptr_for_a.fetch_add(Tracked(&mut v.counter_perm), 1);

                    proof {
                        // the ghost update, in the same atomic step:
                        v.inc_a_inv.combine(inc_a_thread);  // join the halves: now we own all of `inc_a`
                        v.inc_a_inv.update(1);  // allowed: we own everything. inc_a: 0 -> 1
                        inc_a_thread = v.inc_a_inv.split();  // split again: each side keeps a half
                    }
                });
                Tracked(inc_a_thread)
            },
    );

    // Thread B: the same, for inc_b.
    let thread_b = vstd::thread::spawn(
        move || -> (r: Tracked<FracGhost<int>>)
            ensures
                r@.id() == inc_b_id,
                r@.frac() == 0.5real,
                r@@ == 1,
            {
                let tracked mut inc_b_thread = inc_b_thread;
                open_atomic_invariant!(inv_for_b.borrow() => v => {
                    ptr_for_b.fetch_add(Tracked(&mut v.counter_perm), 1);
                    proof {
                        v.inc_b_inv.combine(inc_b_thread);
                        v.inc_b_inv.update(1);
                        inc_b_thread = v.inc_b_inv.split();
                    }
                });
                Tracked(inc_b_thread)
            },
    );

    // The parent joins both threads and gets both halves back, both saying "added".
    if let (Ok(join_a), Ok(join_b)) = (thread_a.join(), thread_b.join()) {
        let Tracked(inc_a_thread) = join_a;
        let Tracked(inc_b_thread) = join_b;
        let n: u32;
        open_atomic_invariant!(shared_inv.borrow() => v => {
            n = ptr.load(Tracked(&v.counter_perm));
            proof {
                // the atomic invariant's halves agree with the halves we hold: inc_a == 1 and inc_b == 1
                v.inc_a_inv.agree(&inc_a_thread);
                v.inc_b_inv.agree(&inc_b_thread);
            }
        });
        // the rule says counter == inc_a + inc_b == 1 + 1
        assert(n == 2);
    }
}

} // verus!
