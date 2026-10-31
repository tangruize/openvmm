use std::sync::Arc;
use vstd::atomic::*;
use vstd::invariant::*;
use vstd::prelude::*;
use vstd::resource::Loc;
use vstd::resource::ghost_var::*;
use vstd::thread::*;

verus! {

////////////////////////////////////////////////////////////////////////////////
// One owner writes, many threads read.
//
// A shared atomic counter. Only the owner (the parent) ever changes it; other threads
// just read it. The owner wants to know the counter's EXACT value at all times, even
// while the readers run.
//
// One ghost variable tracks the counter's value. It comes as two tokens that always
// agree: `GhostVarAuth` sits in the atomic invariant, and the owner keeps `GhostVar`.
// The rule says  counter == the ghost number.  Changing the ghost number needs BOTH
// tokens, so only the owner (who holds one of them) can add to the counter.
////////////////////////////////////////////////////////////////////////////////

// K: the ids that say which counter and which ghost variable the rule is about.
pub struct K {
    pub x_id: int,
    pub gv_id: Loc,
}

// V: the contents of the atomic invariant: the permission for the counter, and the
// invariant's token for the ghost variable.
pub struct V {
    pub perm: PermissionU64,
    pub auth: GhostVarAuth<u64>,
}

pub struct MirrorInv;

impl InvariantPredicate<K, V> for MirrorInv {
    open spec fn inv(k: K, v: V) -> bool {
        &&& v.perm.id() == k.x_id
        &&& v.auth.id() == k.gv_id
        &&& v.perm.value() == v.auth@  // the counter equals the ghost number
    }
}

type Inv = AtomicInvariant<K, V, MirrorInv>;

// The owner adds `n`. In the same atomic step it moves its ghost variable, so the owner
// always knows the exact new value.
fn add(
    x: &PAtomicU64,
    Tracked(inv): Tracked<&Inv>,
    Tracked(var): Tracked<&mut GhostVar<u64>>,
    n: u64,
)
    requires
        inv.constant().x_id == x.id(),
        inv.constant().gv_id == old(var).id(),
    ensures
        final(var).id() == old(var).id(),
        final(var)@ == old(var)@.wrapping_add(n),
{
    open_atomic_invariant!(inv => v => {
        let tracked V { mut perm, mut auth } = v;
        proof {
            auth.agree(&*var);  // the invariant's token and the owner's token agree
        }
        x.fetch_add_wrapping(Tracked(&mut perm), n);
        proof {
            auth.update(var, perm.value());  // move the ghost number to the new value
            v = V { perm, auth };
        }
    });
}

// A reader: it just loads the counter. It can say nothing exact about the value.
fn read(x: &PAtomicU64, Tracked(inv): Tracked<&Inv>) -> (r: u64)
    requires
        inv.constant().x_id == x.id(),
{
    let result;
    open_atomic_invariant!(inv => v => {
        result = x.load(Tracked(&v.perm));
    });
    result
}

fn spawn_reader(x: &Arc<PAtomicU64>, inv: &Arc<Tracked<Inv>>) -> (h: JoinHandle<u64>)
    requires
        inv@.constant().x_id == x.id(),
{
    let (x, inv) = (x.clone(), inv.clone());
    spawn(move || -> (r: u64) { read(&x, Tracked(inv.borrow())) })
}

fn owner() {
    let (x, Tracked(perm)) = PAtomicU64::new(0);
    let tracked (auth, mut var) = GhostVarAuth::new(0u64);
    let tracked inv = Inv::new(K { x_id: x.id(), gv_id: auth.id() }, V { perm, auth }, 1);
    let x = Arc::new(x);
    let inv = Arc::new(Tracked(inv));

    let reader_a = spawn_reader(&x, &inv);
    let reader_b = spawn_reader(&x, &inv);

    // The owner adds twice while the readers run. It knows the exact value.
    add(&x, Tracked(inv.borrow()), Tracked(&mut var), 5);
    add(&x, Tracked(inv.borrow()), Tracked(&mut var), 7);
    assert(var@ == 12);

    let _ = reader_a.join();
    let _ = reader_b.join();

    // The final read: the owner's token agrees with the invariant's, so the value is 12.
    let result;
    open_atomic_invariant!(inv.borrow() => v => {
        let tracked V { perm, auth } = v;
        result = x.load(Tracked(&perm));
        proof {
            auth.agree(&var);
            v = V { perm, auth };
        }
    });
    assert(result == 12);
}

fn main() {
    owner();
}

} // verus!
