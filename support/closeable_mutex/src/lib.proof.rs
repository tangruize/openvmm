// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

// Examples showing that the trusted specification can be used; no human
// review needed.

use crate::CloseableMutex;
use crate::LockInvariant;
use std::sync::Arc;
use vstd::prelude::*;

verus! {

/// A counter that never exceeds its limit; the limit is the key.
pub(crate) struct ExampleCounter {
    pub(crate) n: u64,
    pub(crate) limit: u64,
}

impl LockInvariant for ExampleCounter {
    type Key = u64;

    open(crate) spec fn key(&self) -> u64 {
        self.limit
    }

    open(crate) spec fn inv(&self) -> bool {
        self.n <= self.limit
    }
}

/// Closes the mutex, increments the counter if below the limit, and reopens.
pub(crate) fn example_increment(m: Arc<CloseableMutex<ExampleCounter>>) -> (r: Arc<
    CloseableMutex<ExampleCounter>,
>)
    ensures
        (*r).key() == (*m).key(),
{
    let mut g = CloseableMutex::close_verified(m);
    let c = g.borrow_mut();
    if c.n < c.limit {
        c.n = c.n + 1;
    }
    g.release()
}

} // verus!
