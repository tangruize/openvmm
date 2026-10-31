use vstd::atomic::*;
use vstd::prelude::*;

verus! {

////////////////////////////////////////////////////////////////////////////////
// An atomic variable and its permission.
//
// `PAtomicU64::new` returns the variable together with its permission. Every
// operation on the variable takes the permission as a ghost argument.
////////////////////////////////////////////////////////////////////////////////

fn permission_demo() {
    let (x, Tracked(mut perm)) = PAtomicU64::new(0);
    x.store(Tracked(&mut perm), 11);
    let v = x.load(Tracked(&perm));
    assert(v == 11);
}

fn main() {
    permission_demo();
}

} // verus!
