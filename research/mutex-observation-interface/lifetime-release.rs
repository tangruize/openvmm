// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

include!("candidate.spec.rs");
use parking_lot::Mutex;

verus! {
pub fn release(mutex: &Mutex<u64>) -> u64 {
    let borrowed;
    {
        let guard = mutex.lock();
        borrowed = &*guard;
    }
    *borrowed
}
}
