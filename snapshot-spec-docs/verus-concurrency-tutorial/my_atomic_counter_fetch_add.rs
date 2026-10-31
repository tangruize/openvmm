#![verifier::exec_allows_no_decreases_clause]

use std::sync::Arc;
use vstd::atomic::*;
use vstd::invariant::*;
use vstd::prelude::*;
use vstd::thread::*;

verus! {

////////////////////////////////////////////////////////////////////////////////
// The simplest atomic-invariant example.
//
// A shared counter that is only ever changed by:
//   * `add_two`: one hardware instruction (`fetch_add`), and
//   * `reset`:   one hardware instruction (`store`).
// Each operation is ONE physical atomic step, so it opens the invariant exactly
// once. No loop, no compare-and-swap, no ghost tokens, no atomic specification.
//
// The invariant says the counter is always even. Every step preserves it, so no
// matter how many threads run these operations in any interleaving, the value
// read at the end is even.
////////////////////////////////////////////////////////////////////////////////

pub struct EvenInv;

impl InvariantPredicate<int, PermissionU64> for EvenInv {
    open spec fn inv(x_id: int, perm: PermissionU64) -> bool {
        &&& perm.id() == x_id  // the permission belongs to our atomic variable
        &&& perm.value() % 2 == 0  // the property we care about
    }
}

type Inv = AtomicInvariant<int, PermissionU64, EvenInv>;

// One physical step (fetch_add), so one open/close of the invariant.
pub fn add_two(x: &PAtomicU64, Tracked(inv): Tracked<&Inv>)
    requires
        inv.constant() == x.id(),
{
    open_atomic_invariant!(inv => perm => {
        x.fetch_add_wrapping(Tracked(&mut perm), 2);
    });
}

// One physical step (store), so one open/close of the invariant.
pub fn reset(x: &PAtomicU64, Tracked(inv): Tracked<&Inv>)
    requires
        inv.constant() == x.id(),
{
    open_atomic_invariant!(inv => perm => {
        x.store(Tracked(&mut perm), 0);
    });
}

// The atomic variable and the (ghost) invariant are shared between threads in
// two separate `Arc`s.
fn spawn_adder(x: &Arc<PAtomicU64>, inv: &Arc<Tracked<Inv>>) -> (h: JoinHandle<()>)
    requires
        inv@.constant() == x.id(),
{
    let (x, inv) = (x.clone(), inv.clone());
    spawn(move || -> (r: ()) { add_two(&x, Tracked(inv.borrow())) })
}

fn spawn_resetter(x: &Arc<PAtomicU64>, inv: &Arc<Tracked<Inv>>) -> (h: JoinHandle<()>)
    requires
        inv@.constant() == x.id(),
{
    let (x, inv) = (x.clone(), inv.clone());
    spawn(move || -> (r: ()) { reset(&x, Tracked(inv.borrow())) })
}

fn run_threads() -> (out: Result<u64, ()>)
    ensures
        out is Ok ==> out.unwrap() % 2 == 0,
{
    let (x, Tracked(perm)) = PAtomicU64::new(0);
    let tracked inv = Inv::new(x.id(), perm, 1);
    let x = Arc::new(x);
    let inv = Arc::new(Tracked(inv));

    let a0 = spawn_adder(&x, &inv);
    let a1 = spawn_adder(&x, &inv);

    let r0 = spawn_resetter(&x, &inv);
    let r1 = spawn_resetter(&x, &inv);

    let ok = a0.join().is_ok() && a1.join().is_ok() && r0.join().is_ok() && r1.join().is_ok();

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
        assert(x % 2 == 0);
    }
}

} // verus!
