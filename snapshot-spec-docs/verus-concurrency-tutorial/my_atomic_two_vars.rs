#![verifier::exec_allows_no_decreases_clause]

use std::sync::Arc;
use vstd::atomic::*;
use vstd::invariant::*;
use vstd::prelude::*;
use vstd::thread::*;

verus! {

////////////////////////////////////////////////////////////////////////////////
// An atomic invariant over TWO atomic variables.
//
// A window [lo, hi] that threads may only widen:
//   * `widen_down(n)`: lo = min(lo, n)   (one `fetch_min` on `lo`)
//   * `widen_up(n)`:   hi = max(hi, n)   (one `fetch_max` on `hi`)
//
// The invariant relates the two variables: lo <= hi. An invariant sees only its own
// contents, so to talk about both variables it must hold BOTH permissions. Each
// operation touches only one atomic, but it opens the same shared invariant and
// gets both permissions; it uses one and puts both back.
//
// Each step keeps lo <= hi because it moves its variable in the safe direction
// (lo only goes down, hi only goes up).
////////////////////////////////////////////////////////////////////////////////

pub struct K {
    pub lo_id: int,
    pub hi_id: int,
}

pub struct V {
    pub lo_p: PermissionU64,
    pub hi_p: PermissionU64,
}

// A permission is linear: each one lives in exactly one invariant, so we cannot also
// give `lo` or `hi` its own separate invariant. Properties of a single variable
// (e.g. `lo <= 100`) are simply extra clauses in this joint predicate.
pub struct WindowInv;

impl InvariantPredicate<K, V> for WindowInv {
    open spec fn inv(k: K, v: V) -> bool {
        &&& v.lo_p.id() == k.lo_id
        &&& v.hi_p.id() == k.hi_id
        &&& v.lo_p.value() <= v.hi_p.value()  // the relation between the two variables
    }
}

type Inv = AtomicInvariant<K, V, WindowInv>;

// One physical step (fetch_min on lo), so one open/close of the invariant.
pub fn widen_down(lo: &PAtomicU64, Tracked(inv): Tracked<&Inv>, n: u64)
    requires
        inv.constant().lo_id == lo.id(),
{
    open_atomic_invariant!(inv => v => {
        lo.fetch_min(Tracked(&mut v.lo_p), n);
    });
}

// One physical step (fetch_max on hi), so one open/close of the invariant.
pub fn widen_up(hi: &PAtomicU64, Tracked(inv): Tracked<&Inv>, n: u64)
    requires
        inv.constant().hi_id == hi.id(),
{
    open_atomic_invariant!(inv => v => {
        hi.fetch_max(Tracked(&mut v.hi_p), n);
    });
}

// Widen the window so that it contains v: two physical steps, each one safe by itself.
// The fetch_min / fetch_max instructions ARE the "if v < lo" / "if v > hi" checks, done
// atomically; writing `load` then `store` by hand would let the decision go stale.
// At most one of the two steps changes anything (v < lo implies v < hi, and so on), so
// from the outside this behaves like one step, but the invariant proof does not need that.
pub fn widen(lo: &PAtomicU64, hi: &PAtomicU64, Tracked(inv): Tracked<&Inv>, v: u64)
    requires
        inv.constant().lo_id == lo.id(),
        inv.constant().hi_id == hi.id(),
{
    widen_down(lo, Tracked(inv), v);
    widen_up(hi, Tracked(inv), v);
}

fn spawn_widen_down(lo: &Arc<PAtomicU64>, inv: &Arc<Tracked<Inv>>, n: u64) -> (h: JoinHandle<()>)
    requires
        inv@.constant().lo_id == lo.id(),
{
    let (lo, inv) = (lo.clone(), inv.clone());
    spawn(move || -> (r: ()) { widen_down(&lo, Tracked(inv.borrow()), n) })
}

fn spawn_widen_up(hi: &Arc<PAtomicU64>, inv: &Arc<Tracked<Inv>>, n: u64) -> (h: JoinHandle<()>)
    requires
        inv@.constant().hi_id == hi.id(),
{
    let (hi, inv) = (hi.clone(), inv.clone());
    spawn(move || -> (r: ()) { widen_up(&hi, Tracked(inv.borrow()), n) })
}

fn spawn_widen(
    lo: &Arc<PAtomicU64>,
    hi: &Arc<PAtomicU64>,
    inv: &Arc<Tracked<Inv>>,
    v: u64,
) -> (h: JoinHandle<()>)
    requires
        inv@.constant().lo_id == lo.id(),
        inv@.constant().hi_id == hi.id(),
{
    let (lo, hi, inv) = (lo.clone(), hi.clone(), inv.clone());
    spawn(move || -> (r: ()) { widen(&lo, &hi, Tracked(inv.borrow()), v) })
}

fn run_threads() {
    let (lo, Tracked(lo_p)) = PAtomicU64::new(50);
    let (hi, Tracked(hi_p)) = PAtomicU64::new(50);
    let tracked inv = Inv::new(K { lo_id: lo.id(), hi_id: hi.id() }, V { lo_p, hi_p }, 1);
    let lo = Arc::new(lo);
    let hi = Arc::new(hi);
    let inv = Arc::new(Tracked(inv));

    let d0 = spawn_widen_down(&lo, &inv, 40);
    let d1 = spawn_widen_down(&lo, &inv, 10);
    let d2 = spawn_widen_down(&lo, &inv, 30);

    let u0 = spawn_widen_up(&hi, &inv, 60);
    let u1 = spawn_widen_up(&hi, &inv, 90);
    let u2 = spawn_widen_up(&hi, &inv, 70);

    let w0 = spawn_widen(&lo, &hi, &inv, 5);
    let w1 = spawn_widen(&lo, &hi, &inv, 95);

    let _ = d0.join();
    let _ = d1.join();
    let _ = d2.join();
    let _ = u0.join();
    let _ = u1.join();
    let _ = u2.join();
    let _ = w0.join();
    let _ = w1.join();

    // Exec code cannot read two atomics in one atomic step, so the relation is stated
    // about the ghost permissions, in proof mode, by opening the invariant there.
    let Tracked(credit) = vstd::invariant::create_open_invariant_credit();
    proof {
        open_atomic_invariant_in_proof!(credit => inv.borrow() => v => {
            assert(v.lo_p.value() <= v.hi_p.value());
        });
    }
}

fn main() {
    run_threads();
}

} // verus!
