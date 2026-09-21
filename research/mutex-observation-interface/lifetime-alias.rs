// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

include!("candidate.spec.rs");
use parking_lot::Mutex;

verus! {
pub fn alias(mutex: &Mutex<u64>) -> u64 {
    let mut guard = mutex.lock();
    let borrowed = &*guard;
    *guard = 7;
    *borrowed
}
}
