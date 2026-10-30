// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

// Examples showing that the trusted specification can be used; no human
// review needed.

use crate::CloseableMutex;
use crate::LockInvariant;
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

/// Creates a mutex whose counter never exceeds `limit`.
pub(crate) fn example_new(limit: u64) -> (m: CloseableMutex<ExampleCounter>)
    ensures
        m.param() == limit,
{
    CloseableMutex::new_verified(ExampleCounter { n: 0, limit }, Ghost(limit))
}

/// Closes the mutex, increments the counter if below the limit, and reopens.
pub(crate) fn example_increment(m: Arc<CloseableMutex<ExampleCounter>>) -> (r: Arc<
    CloseableMutex<ExampleCounter>,
>)
    ensures
        (*r).param() == (*m).param(),
{
    let mut g = CloseableMutex::close_verified(&m);
    let c = g.borrow_mut();
    if c.n < c.limit {
        c.n = c.n + 1;
    }
    g.release()
}

} // verus!
