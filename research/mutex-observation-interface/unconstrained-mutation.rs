// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

include!("candidate.spec.rs");
use parking_lot::Mutex;

verus! {
pub fn mutate(mutex: &Mutex<u64>) {
    let mut guard = mutex.lock();
    *guard = 7;
}
}
