#![verifier::exec_allows_no_decreases_clause]
#![verifier::loop_isolation(false)]

use std::sync::Arc;
use vstd::atomic::*;
use vstd::invariant::*;
use vstd::prelude::*;
use vstd::thread::*;

verus! {

////////////////////////////////////////////////////////////////////////////////
// Several atomic steps: a compare-and-swap loop.
//
// The counter should always be even and every call adds 2. A single `fetch_add` could do
// this, but suppose we want "add 2 unless ...", or any update that is not one hardware
// instruction. The standard tool is a loop:
//     read the value,  compute the new value,  try to swap it in,  retry if someone interfered.
// That is SEVERAL atomic steps, and each one opens and closes the atomic invariant on its own.
////////////////////////////////////////////////////////////////////////////////

pub struct EvenInv;

impl InvariantPredicate<int, PermissionU64> for EvenInv {
    open spec fn inv(x_id: int, perm: PermissionU64) -> bool {
        &&& perm.id() == x_id  // the permission belongs to our counter
        &&& perm.value() % 2 == 0  // the rule: always even
    }
}

type Inv = AtomicInvariant<int, PermissionU64, EvenInv>;

// Add 2 with a compare-and-swap loop.
pub fn add_two(x: &PAtomicU64, Tracked(inv): Tracked<&Inv>)
    requires
        inv.constant() == x.id(),
{
    // step 1: read the current value
    let mut curr;
    open_atomic_invariant!(inv => perm => {
        curr = x.load(Tracked(&perm));
    });

    loop {
        let next = curr.wrapping_add(2);
        // step 2: swap `next` in, but only if the value is still `curr`
        let res;
        open_atomic_invariant!(inv => perm => {
            res = x.compare_exchange_weak(Tracked(&mut perm), curr, next);
        });
        match res {
            Ok(_) => {
                return ;
            },
            // somebody changed the counter first: `new` is its latest value, try again
            Err(new) => {
                curr = new;
            },
        }
    }
}

fn spawn_add_two(x: &Arc<PAtomicU64>, inv: &Arc<Tracked<Inv>>) -> (h: JoinHandle<()>)
    requires
        inv@.constant() == x.id(),
{
    let (x, inv) = (x.clone(), inv.clone());
    spawn(move || -> (r: ()) { add_two(&x, Tracked(inv.borrow())) })
}

fn main() {
    let (x, Tracked(perm)) = PAtomicU64::new(0);
    let tracked inv = Inv::new(x.id(), perm, 1);
    let x = Arc::new(x);
    let inv = Arc::new(Tracked(inv));

    let t0 = spawn_add_two(&x, &inv);
    let t1 = spawn_add_two(&x, &inv);
    let t2 = spawn_add_two(&x, &inv);
    let _ = t0.join();
    let _ = t1.join();
    let _ = t2.join();

    let result;
    open_atomic_invariant!(inv.borrow() => perm => {
        result = x.load(Tracked(&perm));
    });
    assert(result % 2 == 0);
}

} // verus!
