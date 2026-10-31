#![verifier::exec_allows_no_decreases_clause]
#![verifier::loop_isolation(false)]

use std::sync::Arc;
use vstd::atomic::*;
use vstd::invariant::*;
use vstd::prelude::*;
use vstd::resource::Loc;
use vstd::resource::ghost_var::*;
use vstd::thread::*;

verus! {

////////////////////////////////////////////////////////////////////////////////
// Why an atomic specification: ONE library function with SEVERAL atomic steps,
// used by two owners whose invariants the function cannot know.
//
// `add_two` adds 2 with a compare-and-swap loop (several atomic steps). It knows nothing
// about invariants. It only promises: "at ONE instant I turn a permission with value v
// into one with value v + 2", and it asks the caller for the permission at the moments it
// needs it.
//
// Two different counters use it, each owned by its own invariant:
//   * owner A: just "the value is even"
//   * owner B: "the value equals a ghost number"; B moves that number at the very instant
//     of the add, so B learns exactly what the call did.
////////////////////////////////////////////////////////////////////////////////

// ANCHOR: reset
// The simplest atomic specification: ONE atomic step, and the update is always committed.
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
// ANCHOR_END: reset

// ANCHOR: add_two
pub fn add_two(var: &PAtomicU64) -> (out: u64)
    atomically (atomic_update) {
        (perm: PermissionU64) -> (res: Result<PermissionU64, (PermissionU64, OpenInvariantCredit)>),
        requires
            perm.id() == var.id(),
        ensures match res {
            Err((p, _)) => p@ == perm@,
            Ok(p) => {
                &&& p.id() == perm.id()
                &&& p.value() == perm.value().wrapping_add(2)
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

    // the first read: open the update, look at the value, and give it back unchanged (abort)
    let mut curr;
    let wrapped_au = try_open_atomic_update!(au, perm => {
        curr = var.load(Tracked(&perm));
        Tracked(Err((perm, credit)))
    });
    proof { au = wrapped_au.get().tracked_unwrap_err() };

    loop invariant au == atomic_update {
        let Tracked(credit) = vstd::invariant::create_open_invariant_credit();
        let next = curr.wrapping_add(2);

        // try to swap the new value in; commit the update if it worked, abort if not
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
            },
            Err(new) => {
                proof { au = maybe_au.get().tracked_unwrap_err() };
                curr = new;
            },
        }
    }
}
// ANCHOR_END: add_two

////////////////////////////////////////////////////////////////////////////////
// Owner A: the counter must always be even. Several threads share it.
////////////////////////////////////////////////////////////////////////////////

// ANCHOR: owner_a
pub struct EvenInv;

impl InvariantPredicate<int, PermissionU64> for EvenInv {
    open spec fn inv(x_id: int, perm: PermissionU64) -> bool {
        &&& perm.id() == x_id
        &&& perm.value() % 2 == 0
    }
}

type InvA = AtomicInvariant<int, PermissionU64, EvenInv>;

// Owner A's wrapper: it opens ITS OWN invariant inside `update`.
fn add_two_even(x: &PAtomicU64, Tracked(inv): Tracked<&InvA>)
    requires
        inv.constant() == x.id(),
{
    let Tracked(mut credit) = vstd::invariant::create_open_invariant_credit();
    add_two(x) atomically loop |update| {
        let tracked mut spare = None;
        open_atomic_invariant!(credit => inv => perm => {
            let tracked res: Result<PermissionU64, (PermissionU64, OpenInvariantCredit)> =
                update(perm);
            match res {
                Ok(p) => perm = p,
                Err((p, c)) => {
                    perm = p;
                    spare = Some(c);
                },
            }
        });
        match spare {
            None => break,
            Some(c) => credit = c,
        }
    };
}
// ANCHOR_END: owner_a

// ANCHOR: reset_even
// Owner A's wrapper for `reset`: no loop, because the update cannot be aborted.
fn reset_even(x: &PAtomicU64, Tracked(inv): Tracked<&InvA>)
    requires
        inv.constant() == x.id(),
{
    let Tracked(credit) = vstd::invariant::create_open_invariant_credit();
    reset(x) atomically |update| {
        open_atomic_invariant!(credit => inv => perm => {
            perm = Commit::get(update(perm));
        });
    };
}
// ANCHOR_END: reset_even

fn spawn_add_two_even(x: &Arc<PAtomicU64>, inv: &Arc<Tracked<InvA>>) -> (h: JoinHandle<()>)
    requires
        inv@.constant() == x.id(),
{
    let (x, inv) = (x.clone(), inv.clone());
    spawn(move || -> (r: ()) { add_two_even(&x, Tracked(inv.borrow())) })
}

fn owner_a() {
    let (x, Tracked(perm)) = PAtomicU64::new(0);
    let tracked inv = InvA::new(x.id(), perm, 1);
    let x = Arc::new(x);
    let inv = Arc::new(Tracked(inv));

    let t0 = spawn_add_two_even(&x, &inv);
    let t1 = spawn_add_two_even(&x, &inv);
    let t2 = spawn_add_two_even(&x, &inv);
    let _ = t0.join();
    let _ = t1.join();
    let _ = t2.join();

    reset_even(&x, Tracked(inv.borrow()));

    let result;
    open_atomic_invariant!(inv.borrow() => perm => {
        result = x.load(Tracked(&perm));
    });
    assert(result % 2 == 0);
}

////////////////////////////////////////////////////////////////////////////////
// Owner B: the counter mirrors a ghost number. B holds a ticket for that number and
// moves it at the instant of the add, so afterwards B knows exactly what happened.
////////////////////////////////////////////////////////////////////////////////

// ANCHOR: owner_b
pub struct KB {
    pub x_id: int,
    pub gv_id: Loc,
}

pub struct VB {
    pub perm: PermissionU64,
    pub auth: GhostVarAuth<u64>,
}

pub struct MirrorInv;

impl InvariantPredicate<KB, VB> for MirrorInv {
    open spec fn inv(k: KB, v: VB) -> bool {
        &&& v.perm.id() == k.x_id
        &&& v.auth.id() == k.gv_id
        &&& v.perm.value() == v.auth@  // the counter equals the ghost number
    }
}

type InvB = AtomicInvariant<KB, VB, MirrorInv>;

// Owner B's wrapper around the SAME `add_two`: it opens ITS (different) invariant inside
// `update`, and moves its ticket to the new value at that instant.
fn add_two_tracked(x: &PAtomicU64, Tracked(inv): Tracked<&InvB>, Tracked(ticket): Tracked<&mut GhostVar<u64>>) -> (out: u64)
    requires
        inv.constant().x_id == x.id(),
        inv.constant().gv_id == old(ticket).id(),
    ensures
        final(ticket).id() == old(ticket).id(),
        out == old(ticket)@,
        final(ticket)@ == old(ticket)@.wrapping_add(2),
{
    let Tracked(mut credit) = vstd::invariant::create_open_invariant_credit();
    let ghost start = ticket@;
    let ghost ticket_id = ticket.id();
    let out = add_two(x) atomically loop |update|
        invariant
            ticket.id() == ticket_id,
            ticket@ == start,
            inv.constant().gv_id == ticket_id,
    {
        let tracked mut spare = None;
        open_atomic_invariant!(credit => inv => v => {
            let tracked VB { perm, mut auth } = v;
            auth.agree(&*ticket);
            let tracked res: Result<PermissionU64, (PermissionU64, OpenInvariantCredit)> =
                update(perm);
            match res {
                Ok(p) => {
                    auth.update(ticket, p.value());
                    v = VB { perm: p, auth };
                },
                Err((p, c)) => {
                    v = VB { perm: p, auth };
                    spare = Some(c);
                },
            }
        });
        match spare {
            None => break,
            Some(c) => credit = c,
        }
    };
    out
}
// ANCHOR_END: owner_b

fn owner_b() {
    let (x, Tracked(perm)) = PAtomicU64::new(0);
    let tracked (auth, mut ticket) = GhostVarAuth::new(0u64);
    let tracked inv = InvB::new(KB { x_id: x.id(), gv_id: auth.id() }, VB { perm, auth }, 1);

    let a = add_two_tracked(&x, Tracked(&inv), Tracked(&mut ticket));
    let b = add_two_tracked(&x, Tracked(&inv), Tracked(&mut ticket));

    assert(a == 0);
    assert(b == 2);
    assert(ticket@ == 4);
}

fn main() {
    owner_a();
    owner_b();
}

} // verus!
