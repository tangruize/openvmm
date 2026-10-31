#![verifier::exec_allows_no_decreases_clause]
#![verifier::loop_isolation(false)]

use std::sync::Arc;
use vstd::atomic::*;
use vstd::invariant::*;
use vstd::prelude::*;
use vstd::thread::*;

verus! {

////////////////////////////////////////////////////////////////////////////////
// A signed counter whose value is always non-negative.
//
// `reset` sets the counter to 0. `increment` adds one, but saturates at
// `i64::MAX` instead of wrapping around to `i64::MIN`. Because of that, every
// operation keeps the counter in the range [0, i64::MAX], so the atomic
// invariant "value >= 0" is inductive: no ghost state beyond the permission
// is needed, no matter how many threads call the functions.
////////////////////////////////////////////////////////////////////////////////

// Add one, but stay at i64::MAX instead of wrapping to i64::MIN.
pub open spec fn saturating_inc(v: i64) -> i64 {
    if v == i64::MAX {
        v
    } else {
        (v + 1) as i64
    }
}

pub fn reset(var: &PAtomicI64)
    atomically (atomic_update) {
        (perm: PermissionI64) -> (commit: Commit<PermissionI64>),
        requires
            perm.id() == var.id(),
        ensures
            commit@.id() == perm.id(),
            commit@.value() == 0,
        outer_mask any,
        inner_mask none,
    },
{
    try_open_atomic_update!(atomic_update, mut perm => {
        var.store(Tracked(&mut perm), 0);
        Tracked(Commit(perm))
    });
    assert(atomic_update.resolves());
}

// Saturating increment: never overflows, so the counter never becomes negative.
pub fn increment(var: &PAtomicI64) -> (out: i64)
    atomically (atomic_update) {
        (perm: PermissionI64) -> (res: Result<PermissionI64, (PermissionI64, OpenInvariantCredit)>),
        requires
            perm.id() == var.id(),
        ensures match res {
            Err((p, _)) => p@ == perm@,
            Ok(p) => {
                &&& p.id() == perm.id()
                &&& p.value() == saturating_inc(perm.value())
            }
        },
        outer_mask any,
        inner_mask none,
    },
    ensures
        out == perm.value(),
{
    let Tracked(credit) = vstd::invariant::create_open_invariant_credit();
    let tracked mut au = atomic_update;

    let mut curr;
    let wrapped_au = try_open_atomic_update!(au, perm => {
        curr = var.load(Tracked(&perm));
        Tracked(Err((perm, credit)))
    });

    proof { au = wrapped_au.get().tracked_unwrap_err() };

    loop invariant au == atomic_update {
        let Tracked(credit) = vstd::invariant::create_open_invariant_credit();

        if curr == i64::MAX {
            // Saturated: commit without changing the value, but only if the
            // counter is still at the maximum at the moment we look at it.
            let seen;
            let maybe_au = try_open_atomic_update!(au, perm => {
                seen = var.load(Tracked(&perm));
                Tracked(if seen == i64::MAX { Ok(perm) } else { Err((perm, credit)) })
            });

            if seen == i64::MAX {
                assert(atomic_update.resolves());
                return seen;
            }
            proof { au = maybe_au.get().tracked_unwrap_err() };
            curr = seen;
        } else {
            // No overflow: curr != i64::MAX in this branch.
            let next = curr + 1;

            let res;
            let maybe_au = try_open_atomic_update!(au, mut perm => {
                res = var.compare_exchange_weak(Tracked(&mut perm), curr, next);

                Tracked(match res {
                    Ok(_) => Ok(perm),
                    Err(_) => Err((perm, credit)),
                })
            });

            match res {
                Ok(_) => {
                    assert(atomic_update.resolves());
                    return curr;
                }

                Err(new) => {
                    proof { au = maybe_au.get().tracked_unwrap_err() };
                    curr = new;
                },
            }
        }
    }
}

////////////////////////////////////////////////////////////////////////////////
// The atomic invariant. The only interesting clause is `value >= 0`; the
// first clause just ties the permission to the atomic variable it belongs to.
////////////////////////////////////////////////////////////////////////////////

pub struct NonNegInv;

impl InvariantPredicate<int, PermissionI64> for NonNegInv {
    open spec fn inv(x_id: int, perm: PermissionI64) -> bool {
        &&& perm.id() == x_id
        &&& perm.value() >= 0
    }
}

pub struct Shared {
    pub x: PAtomicI64,
    pub inv: Tracked<AtomicInvariant<int, PermissionI64, NonNegInv>>,
}

impl Shared {
    pub open spec fn wf(self) -> bool {
        &&& self.inv@.constant() == self.x.id()
        &&& self.inv@.namespace() == 1
    }
}

fn increment_thread(s: Arc<Shared>)
    requires
        s.wf(),
{
    let tracked inv = s.inv.borrow();
    let Tracked(mut credit) = vstd::invariant::create_open_invariant_credit();

    increment(&s.x) atomically loop |update|
        invariant
            s.wf(),
    {
        let tracked mut spare = None;
        open_atomic_invariant!(credit => inv => perm => {
            let tracked res: Result<PermissionI64, (PermissionI64, OpenInvariantCredit)> =
                update(perm);
            match res {
                Ok(p) => perm = p,
                Err((p, c)) => {
                    perm = p;
                    spare = Some(c);
                }
            }
        });

        match spare {
            None => break,
            Some(c) => credit = c,
        }
    };
}

fn reset_thread(s: Arc<Shared>)
    requires
        s.wf(),
{
    let tracked inv = s.inv.borrow();
    let Tracked(mut credit) = vstd::invariant::create_open_invariant_credit();

    reset(&s.x) atomically |update| {
        open_atomic_invariant!(credit => inv => perm => {
            perm = Commit::get(update(perm));
        });
    };
}

fn spawn_incrementer(s: &Arc<Shared>) -> (h: JoinHandle<()>)
    requires
        s.wf(),
{
    let s = s.clone();
    spawn(move || -> (r: ()) { increment_thread(s) })
}

fn spawn_resetter(s: &Arc<Shared>) -> (h: JoinHandle<()>)
    requires
        s.wf(),
{
    let s = s.clone();
    spawn(move || -> (r: ()) { reset_thread(s) })
}

fn run_threads() -> (out: Result<i64, ()>)
    ensures
        out is Ok ==> out.unwrap() >= 0,
{
    let (x, Tracked(perm)) = PAtomicI64::new(0);
    let tracked inv = AtomicInvariant::<int, PermissionI64, NonNegInv>::new(x.id(), perm, 1);
    let s = Arc::new(Shared { x, inv: Tracked(inv) });

    let i0 = spawn_incrementer(&s);
    let i1 = spawn_incrementer(&s);
    let i2 = spawn_incrementer(&s);
    let i3 = spawn_incrementer(&s);
    let i4 = spawn_incrementer(&s);

    let r0 = spawn_resetter(&s);
    let r1 = spawn_resetter(&s);
    let r2 = spawn_resetter(&s);
    let r3 = spawn_resetter(&s);
    let r4 = spawn_resetter(&s);

    let ok = i0.join().is_ok() && i1.join().is_ok() && i2.join().is_ok() && i3.join().is_ok()
        && i4.join().is_ok() && r0.join().is_ok() && r1.join().is_ok() && r2.join().is_ok()
        && r3.join().is_ok() && r4.join().is_ok();

    if ok {
        let result;
        open_atomic_invariant!(s.inv.borrow() => perm => {
            result = s.x.load(Tracked(&perm));
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
