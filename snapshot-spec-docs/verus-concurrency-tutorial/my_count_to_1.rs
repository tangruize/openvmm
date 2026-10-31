use vstd::atomic::*;
use vstd::prelude::*;
use vstd::thread::*;

verus! {

////////////////////////////////////////////////////////////////////////////////
// Count to 1: the simplest case, with ONE child thread.
//
// The parent creates an atomic counter (value 0) and launches a child thread that adds 1.
// After the child finishes, the parent wants to know that the counter is 1.
//
// Only one thread ever touches the counter at a time, so the parent can simply hand the
// counter AND its permission to the child, and get both back when the child is done.
// No atomic invariant and no fractional ghost value is needed.
////////////////////////////////////////////////////////////////////////////////

fn parent() {
    let (x, Tracked(perm)) = PAtomicU64::new(0);

    // Hand the counter and its permission to the child. Nobody else can touch the counter.
    let handle = spawn(
        move || -> (r: (PAtomicU64, Tracked<PermissionU64>))
            ensures
                r.1@.id() == r.0.id(),
                r.1@.value() == 1,
            {
                let tracked mut perm = perm;
                x.fetch_add_wrapping(Tracked(&mut perm), 1);
                (x, Tracked(perm))
            },
    );

    // The child hands them back, and says what the permission now records.
    if let Ok((x, perm)) = handle.join() {
        let Tracked(perm) = perm;
        let v = x.load(Tracked(&perm));
        assert(v == 1);
    }
}

fn main() {
    parent();
}

} // verus!
