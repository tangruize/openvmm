#![verifier::exec_allows_no_decreases_clause]
#![verifier::loop_isolation(false)]

use std::sync::Arc;
use vstd::atomic::*;
use vstd::invariant::*;
use vstd::prelude::*;
use vstd::resource::map::*;
use vstd::resource::Loc;
use vstd::set_lib::set_int_range;
use vstd::thread::*;

verus! {

////////////////////////////////////////////////////////////////////////////////
// The two logically atomic functions (copied from examples/guide/logatom.rs)
////////////////////////////////////////////////////////////////////////////////

pub fn reset(var: &PAtomicU64)
    atomically (atomic_update) {
        (perm: PermissionU64) -> (commit: Commit<PermissionU64>),
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

pub fn increment(var: &PAtomicU64) -> (out: u64)
    atomically (atomic_update) {
        (perm: PermissionU64) -> (res: Result<PermissionU64, (PermissionU64, OpenInvariantCredit)>),
        requires
            perm.id() == var.id(),
        ensures match res {
            Err((p, _)) => p@ == perm@,
            Ok(p) => {
                &&& p.id() == perm.id()
                &&& p.value() == perm.value().wrapping_add(1)
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
        let next = curr.wrapping_add(1);

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

////////////////////////////////////////////////////////////////////////////////
// Ghost bookkeeping: "which threads have already incremented?"
//
// Every incrementing thread `i` owns a ghost token `i -> false` ("I have not
// incremented yet"). When it increments, it atomically flips its token to
// `i -> true`. `done_count(m, n)` counts how many of threads 0..n have flipped.
////////////////////////////////////////////////////////////////////////////////

pub open spec fn flag(m: Map<int, bool>, k: int) -> nat {
    if m.dom().contains(k) && m[k] {
        1
    } else {
        0
    }
}

pub open spec fn done_count(m: Map<int, bool>, n: nat) -> nat
    decreases n,
{
    if n == 0 {
        0
    } else {
        done_count(m, (n - 1) as nat) + flag(m, (n - 1) as int)
    }
}

pub open spec fn all_false(n: nat) -> Map<int, bool> {
    Map::new(set_int_range(0, n as int), |i: int| false)
}

proof fn lemma_all_false(n: nat, k: int)
    requires
        0 <= k < n,
    ensures
        all_false(n).dom().contains(k),
        all_false(n)[k] == false,
{
}

proof fn lemma_count_le(m: Map<int, bool>, n: nat)
    ensures
        done_count(m, n) <= n,
    decreases n,
{
    if n > 0 {
        lemma_count_le(m, (n - 1) as nat);
    }
}

proof fn lemma_count_all_false(n: nat)
    ensures
        done_count(all_false(n), n) == 0,
    decreases n,
{
    if n > 0 {
        lemma_count_all_false((n - 1) as nat);
        // all_false(n) and all_false(n - 1) agree on 0..n-1
        lemma_count_same_below(all_false(n), all_false((n - 1) as nat), (n - 1) as nat);
    }
}

proof fn lemma_count_same_below(m1: Map<int, bool>, m2: Map<int, bool>, n: nat)
    requires
        forall|k: int| 0 <= k < n ==> flag(m1, k) == flag(m2, k),
    ensures
        done_count(m1, n) == done_count(m2, n),
    decreases n,
{
    if n > 0 {
        lemma_count_same_below(m1, m2, (n - 1) as nat);
    }
}

/// Flipping one `false` entry of `m` to `true` increases the count by exactly one
/// (and shows that the count was below `n` before).
proof fn lemma_count_flip(m: Map<int, bool>, i: int, n: nat)
    requires
        0 <= i < n,
        m.dom().contains(i),
        m[i] == false,
    ensures
        done_count(m.insert(i, true), n) == done_count(m, n) + 1,
        done_count(m, n) < n,
    decreases n,
{
    let m2 = m.insert(i, true);
    lemma_count_le(m, (n - 1) as nat);
    if i == n - 1 {
        lemma_count_same_below(m, m2, (n - 1) as nat);
    } else {
        lemma_count_flip(m, i, (n - 1) as nat);
    }
}

////////////////////////////////////////////////////////////////////////////////
// Types shared by both examples
////////////////////////////////////////////////////////////////////////////////

pub struct K {
    pub x_id: int,
    pub map_id: Loc,
}

pub struct V {
    pub perm: PermissionU64,
    pub auth: GhostMapAuth<int, bool>,
}

pub type Token = GhostPointsTo<int, bool>;

////////////////////////////////////////////////////////////////////////////////
// Example 1: two threads each increment once, starting from 0. Result is 2.
////////////////////////////////////////////////////////////////////////////////

pub struct ExactInv;

impl InvariantPredicate<K, V> for ExactInv {
    open spec fn inv(k: K, v: V) -> bool {
        &&& v.perm.id() == k.x_id
        &&& v.auth.id() == k.map_id
        &&& v.auth@.dom() == all_false(2).dom()
        // the counter is exactly the number of threads that have incremented
        &&& v.perm.value() as int == done_count(v.auth@, 2)
    }
}

pub struct Shared1 {
    pub x: PAtomicU64,
    pub inv: Tracked<AtomicInvariant<K, V, ExactInv>>,
}

impl Shared1 {
    pub open spec fn wf(self) -> bool {
        &&& self.inv@.constant().x_id == self.x.id()
        &&& self.inv@.namespace() == 1
    }
}

fn increment_thread1(s: Arc<Shared1>, Tracked(tok): Tracked<Token>) -> (out: Tracked<Token>)
    requires
        s.wf(),
        tok.id() == s.inv@.constant().map_id,
        tok@.1 == false,
        0 <= tok@.0 < 2,
    ensures
        out@.id() == tok.id(),
        out@@ == (tok@.0, true),
{
    let tracked mut tok = tok;
    let ghost key = tok@.0;
    let ghost map_id = tok.id();
    let tracked inv = s.inv.borrow();
    let Tracked(mut credit) = vstd::invariant::create_open_invariant_credit();

    increment(&s.x) atomically loop |update|
        invariant
            s.wf(),
            tok.id() == map_id,
            tok@ == (key, false),
            0 <= key < 2,
    {
        let tracked mut spare = None;
        open_atomic_invariant!(credit => inv => v => {
            let tracked V { perm, mut auth } = v;
            let ghost perm0 = perm;
            let tracked res: Result<PermissionU64, (PermissionU64, OpenInvariantCredit)> = update(perm);
            match res {
                Ok(p) => {
                    let ghost m0 = auth@;
                    tok.agree(&auth);
                    lemma_count_flip(m0, key, 2);
                    tok.update(&mut auth, true);
                    assert(auth@ =~= m0.insert(key, true));
                    assert(auth@.dom() == m0.dom());
                    assert(p.id() == perm0.id());
                    assert(perm0.value() as int == done_count(m0, 2));
                    assert(p.value() as int == perm0.value() as int + 1);
                    v = V { perm: p, auth };
                }
                Err((p, c)) => {
                    v = V { perm: p, auth };
                    spare = Some(c);
                }
            }
        });

        match spare {
            None => break,
            Some(c) => credit = c,
        }
    };

    Tracked(tok)
}

fn two_increments() -> (out: Result<u64, ()>)
    ensures
        out is Ok ==> out.unwrap() == 2,
{
    let (x, Tracked(perm)) = PAtomicU64::new(0);
    let tracked (mut auth, mut sub) = GhostMapAuth::new(all_false(2));
    let ghost map_id = auth.id();
    proof {
        lemma_all_false(2, 0);
        lemma_all_false(2, 1);
    }
    let tracked t0 = sub.split_points_to(0);
    let tracked t1 = sub.split_points_to(1);

    proof {
        lemma_count_all_false(2);
    }
    let tracked inv = AtomicInvariant::<K, V, ExactInv>::new(
        K { x_id: x.id(), map_id },
        V { perm, auth },
        1,
    );
    let s = Arc::new(Shared1 { x, inv: Tracked(inv) });

    let s0 = s.clone();
    let h0 = spawn(
        move || -> (r: Tracked<Token>)
            ensures
                r@.id() == map_id,
                r@@ == (0int, true),
            { increment_thread1(s0, Tracked(t0)) },
    );
    let s1 = s.clone();
    let h1 = spawn(
        move || -> (r: Tracked<Token>)
            ensures
                r@.id() == map_id,
                r@@ == (1int, true),
            { increment_thread1(s1, Tracked(t1)) },
    );

    if let (Ok(r0), Ok(r1)) = (h0.join(), h1.join()) {
        let result;
        open_atomic_invariant!(s.inv.borrow() => v => {
            let tracked V { perm, auth } = v;
            proof {
                r0.borrow().agree(&auth);
                r1.borrow().agree(&auth);
                reveal_with_fuel(done_count, 3);
            }
            result = s.x.load(Tracked(&perm));
            proof {
                v = V { perm, auth };
            }
        });
        assert(result == 2);
        Ok(result)
    } else {
        Err(())
    }
}

////////////////////////////////////////////////////////////////////////////////
// Example 2: 5 threads increment (once each), 5 threads reset.
// The counter can never exceed 5, and (being a u64) it is never negative.
////////////////////////////////////////////////////////////////////////////////

pub struct BoundedInv;

impl InvariantPredicate<K, V> for BoundedInv {
    open spec fn inv(k: K, v: V) -> bool {
        &&& v.perm.id() == k.x_id
        &&& v.auth.id() == k.map_id
        &&& v.auth@.dom() == all_false(5).dom()
        // the counter never exceeds the number of threads that have incremented
        // (resets can only lower it)
        &&& v.perm.value() as int <= done_count(v.auth@, 5)
    }
}

pub struct Shared2 {
    pub x: PAtomicU64,
    pub inv: Tracked<AtomicInvariant<K, V, BoundedInv>>,
}

impl Shared2 {
    pub open spec fn wf(self) -> bool {
        &&& self.inv@.constant().x_id == self.x.id()
        &&& self.inv@.namespace() == 2
    }
}

fn increment_thread2(s: Arc<Shared2>, Tracked(tok): Tracked<Token>)
    requires
        s.wf(),
        tok.id() == s.inv@.constant().map_id,
        tok@.1 == false,
        0 <= tok@.0 < 5,
{
    let tracked mut tok = tok;
    let ghost key = tok@.0;
    let ghost map_id = tok.id();
    let tracked inv = s.inv.borrow();
    let Tracked(mut credit) = vstd::invariant::create_open_invariant_credit();

    increment(&s.x) atomically loop |update|
        invariant
            s.wf(),
            tok.id() == map_id,
            tok@ == (key, false),
            0 <= key < 5,
    {
        let tracked mut spare = None;
        open_atomic_invariant!(credit => inv => v => {
            let tracked V { perm, mut auth } = v;
            let ghost perm0 = perm;
            let tracked res: Result<PermissionU64, (PermissionU64, OpenInvariantCredit)> = update(perm);
            match res {
                Ok(p) => {
                    let ghost m0 = auth@;
                    tok.agree(&auth);
                    lemma_count_flip(m0, key, 5);
                    tok.update(&mut auth, true);
                    assert(auth@ =~= m0.insert(key, true));
                    assert(auth@.dom() == m0.dom());
                    assert(p.id() == perm0.id());
                    assert(perm0.value() as int <= done_count(m0, 5));
                    assert(p.value() as int == perm0.value() as int + 1);
                    v = V { perm: p, auth };
                }
                Err((p, c)) => {
                    v = V { perm: p, auth };
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

fn reset_thread2(s: Arc<Shared2>)
    requires
        s.wf(),
{
    let tracked inv = s.inv.borrow();
    let Tracked(mut credit) = vstd::invariant::create_open_invariant_credit();

    reset(&s.x) atomically |update| {
        open_atomic_invariant!(credit => inv => v => {
            let tracked V { perm, auth } = v;
            let tracked commit = update(perm);
            v = V { perm: Commit::get(commit), auth };
        });
    };
}

fn spawn_incrementer(s: &Arc<Shared2>, Tracked(tok): Tracked<Token>) -> (h: JoinHandle<()>)
    requires
        s.wf(),
        tok.id() == s.inv@.constant().map_id,
        tok@.1 == false,
        0 <= tok@.0 < 5,
{
    let s = s.clone();
    spawn(move || -> (r: ()) { increment_thread2(s, Tracked(tok)) })
}

fn spawn_resetter(s: &Arc<Shared2>) -> (h: JoinHandle<()>)
    requires
        s.wf(),
{
    let s = s.clone();
    spawn(move || -> (r: ()) { reset_thread2(s) })
}

fn five_increments_five_resets() -> (out: Result<u64, ()>)
    ensures
        out is Ok ==> 0 <= out.unwrap() <= 5,
{
    let (x, Tracked(perm)) = PAtomicU64::new(0);
    let tracked (mut auth, mut sub) = GhostMapAuth::new(all_false(5));
    let ghost map_id = auth.id();
    proof {
        lemma_all_false(5, 0);
        lemma_all_false(5, 1);
        lemma_all_false(5, 2);
        lemma_all_false(5, 3);
        lemma_all_false(5, 4);
    }
    let tracked t0 = sub.split_points_to(0);
    let tracked t1 = sub.split_points_to(1);
    let tracked t2 = sub.split_points_to(2);
    let tracked t3 = sub.split_points_to(3);
    let tracked t4 = sub.split_points_to(4);

    proof {
        lemma_count_all_false(5);
    }
    let tracked inv = AtomicInvariant::<K, V, BoundedInv>::new(
        K { x_id: x.id(), map_id },
        V { perm, auth },
        2,
    );
    let s = Arc::new(Shared2 { x, inv: Tracked(inv) });

    let i0 = spawn_incrementer(&s, Tracked(t0));
    let i1 = spawn_incrementer(&s, Tracked(t1));
    let i2 = spawn_incrementer(&s, Tracked(t2));
    let i3 = spawn_incrementer(&s, Tracked(t3));
    let i4 = spawn_incrementer(&s, Tracked(t4));

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
        open_atomic_invariant!(s.inv.borrow() => v => {
            let tracked V { perm, auth } = v;
            proof {
                lemma_count_le(auth@, 5);
            }
            result = s.x.load(Tracked(&perm));
            proof {
                v = V { perm, auth };
            }
        });
        Ok(result)
    } else {
        Err(())
    }
}

fn main() {
    let _ = two_increments();
    let _ = five_increments_five_resets();
}

} // verus!
