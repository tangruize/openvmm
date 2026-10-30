// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

// Examples showing that the trusted specification can be used; no human
// review needed.

use crate::CloseableMutex;
use crate::LockInvariant;
use vstd::resource::frac::FracGhost;
use std::sync::Arc;
use vstd::prelude::*;

verus! {

/// A counter that never exceeds its limit. Each mutex holding one fixes
/// the limit when it is created (its parameter).
pub(crate) struct ExampleCounter {
    pub(crate) n: u64,
    pub(crate) limit: u64,
}

impl LockInvariant for ExampleCounter {
    /// The mutex's limit.
    type Param = u64;

    open(crate) spec fn inv(&self, limit: u64) -> bool {
        &&& self.limit == limit
        &&& self.n <= self.limit
    }
}

/// Creates a mutex whose counter never exceeds `limit`, with its whole
/// close token.
pub(crate) fn example_new(limit: u64) -> (r: (CloseableMutex<ExampleCounter>, Tracked<FracGhost<()>>))
    ensures
        r.0.param() == limit,
        r.1@.valid(r.0.close_id(), 1real),
{
    CloseableMutex::new_verified(ExampleCounter { n: 0, limit }, Ghost(limit))
}

/// Closes the mutex (with the whole close token), increments the counter if
/// below the limit, and reopens it, getting the token back.
pub(crate) fn example_increment(m: Arc<CloseableMutex<ExampleCounter>>, token: Tracked<FracGhost<()>>) -> (r: (
    Arc<CloseableMutex<ExampleCounter>>,
    Tracked<FracGhost<()>>,
))
    requires
        token@.valid((*m).close_id(), 1real),
    ensures
        (*r.0).param() == (*m).param(),
        r.1@.valid((*r.0).close_id(), 1real),
{
    let mut g = CloseableMutex::close_verified(&m, token);
    let c = g.borrow_mut();
    if c.n < c.limit {
        c.n = c.n + 1;
    }
    g.release()
}

/// Locks the mutex with a piece of its close token (so it is not closed),
/// reads the counter, and unlocks: the counter is within the limit.
pub(crate) fn example_read(m: &CloseableMutex<ExampleCounter>, token: Tracked<&FracGhost<()>>) -> (n: u64)
    requires
        token@.id() == m.close_id(),
    ensures
        n <= m.param(),
{
    let mut g = m.lock_verified(token);
    let n = g.borrow_mut().n;
    g.release();
    n
}

} // verus!
