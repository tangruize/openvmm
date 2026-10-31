use vstd::prelude::*;
use vstd::resource::frac::FracGhost;

verus! {

////////////////////////////////////////////////////////////////////////////////
// Fractional ownership of a ghost value.
//
// A `FracGhost` is a proof-only value with an id, a value, and a fraction of ownership
// between 0 (exclusive) and 1. It can be split into halves and combined again, and the
// value can only be changed by someone who holds all of it (fraction 1).
////////////////////////////////////////////////////////////////////////////////

proof fn frac_ghost_demo() {
    let tracked mut x = FracGhost::new(100int);
    let tracked y = x.split();
    assert(x.frac() == 0.5real);
    assert(y.frac() == 0.5real);
    assert(x@ == 100);
    assert(y@ == 100);
    x.combine(y);
    assert(x.frac() == 1.0real);
    x.update(200);
    assert(x@ == 200);
}

fn main() {
}

} // verus!
