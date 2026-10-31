#![verifier::exec_allows_no_decreases_clause]
#![verifier::loop_isolation(false)]

use std::sync::Arc;
use vstd::atomic::*;
use vstd::invariant::*;
use vstd::prelude::*;
use vstd::thread::*;

verus! {

////////////////////////////////////////////////////////////////////////////////
// Same counter as my_atomic_counter_i64.rs, but with NO logically atomic
// specifications. The functions open the atomic invariant themselves, around
// each single physical atomic operation (load, store, compare-and-swap).
// The invariant `value >= 0` holds after every such step, so we never need to
// treat `increment` as one big atomic step.
//
// The price: `reset` and `increment` are tied to this particular invariant.
////////////////////////////////////////////////////////////////////////////////

pub struct NonNegInv;

impl InvariantPredicate<int, PermissionI64> for NonNegInv {
    open spec fn inv(x_id: int, perm: PermissionI64) -> bool {
        &&& perm.id() == x_id
        &&& perm.value() >= 0
    }
}

type Inv = AtomicInvariant<int, PermissionI64, NonNegInv>;

// One physical step (a store), so one open/close of the invariant.
pub fn reset(x: &PAtomicI64, Tracked(inv): Tracked<&Inv>)
    requires
        inv.constant() == x.id(),
{
    open_atomic_invariant!(inv => perm => {
        x.store(Tracked(&mut perm), 0);
    });
}

// Saturating increment: several physical steps, each under its own open/close.
pub fn increment(x: &PAtomicI64, Tracked(inv): Tracked<&Inv>) -> (out: i64)
    requires
        inv.constant() == x.id(),
{
    let mut curr;
    open_atomic_invariant!(inv => perm => {
        curr = x.load(Tracked(&perm));
    });

    loop {
        if curr == i64::MAX {
            // Saturated: nothing to write, just re-check the value.
            let seen;
            open_atomic_invariant!(inv => perm => {
                seen = x.load(Tracked(&perm));
            });
            if seen == i64::MAX {
                return seen;
            }
            curr = seen;
        } else {
            // No overflow: curr != i64::MAX in this branch.
            let next = curr + 1;
            let res;
            open_atomic_invariant!(inv => perm => {
                res = x.compare_exchange_weak(Tracked(&mut perm), curr, next);
            });
            match res {
                Ok(_) => return curr,
                Err(new) => curr = new,
            }
        }
    }
}

// The atomic variable and the (ghost) invariant are shared between threads in
// two separate `Arc`s, so the atomic variable itself is not wrapped in a new type.
fn spawn_incrementer(x: &Arc<PAtomicI64>, inv: &Arc<Tracked<Inv>>) -> (h: JoinHandle<()>)
    requires
        inv@.constant() == x.id(),
{
    let (x, inv) = (x.clone(), inv.clone());
    spawn(move || -> (r: ()) { increment(&x, Tracked(inv.borrow())); })
}

fn spawn_resetter(x: &Arc<PAtomicI64>, inv: &Arc<Tracked<Inv>>) -> (h: JoinHandle<()>)
    requires
        inv@.constant() == x.id(),
{
    let (x, inv) = (x.clone(), inv.clone());
    spawn(move || -> (r: ()) { reset(&x, Tracked(inv.borrow())) })
}

fn run_threads() -> (out: Result<i64, ()>)
    ensures
        out is Ok ==> out.unwrap() >= 0,
{
    let (x, Tracked(perm)) = PAtomicI64::new(0);
    let tracked inv = Inv::new(x.id(), perm, 1);
    let x = Arc::new(x);
    let inv = Arc::new(Tracked(inv));

    let i0 = spawn_incrementer(&x, &inv);
    let i1 = spawn_incrementer(&x, &inv);
    let i2 = spawn_incrementer(&x, &inv);
    let i3 = spawn_incrementer(&x, &inv);
    let i4 = spawn_incrementer(&x, &inv);

    let r0 = spawn_resetter(&x, &inv);
    let r1 = spawn_resetter(&x, &inv);
    let r2 = spawn_resetter(&x, &inv);
    let r3 = spawn_resetter(&x, &inv);
    let r4 = spawn_resetter(&x, &inv);

    let ok = i0.join().is_ok() && i1.join().is_ok() && i2.join().is_ok() && i3.join().is_ok()
        && i4.join().is_ok() && r0.join().is_ok() && r1.join().is_ok() && r2.join().is_ok()
        && r3.join().is_ok() && r4.join().is_ok();

    if ok {
        let result;
        open_atomic_invariant!(inv.borrow() => perm => {
            result = x.load(Tracked(&perm));
        });
        Ok(result)
    } else {
        Err(())
    }
}

fn main() {
    if let Ok(x) = run_threads() {
        assert(x >= 0);
    }
}

} // verus!
