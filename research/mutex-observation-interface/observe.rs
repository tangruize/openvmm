// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

include!("candidate.spec.rs");
use parking_lot::Mutex;

verus! {

pub fn observe(mutex: &Mutex<u64>) -> (value: u64)
{
    let guard = mutex.lock();
    assert(guard_origin(&guard) == mutex);
    let value = *guard;
    assert(value == *guard_observation(&guard));
    value
}

pub fn unchanged_expression(mutex: &Mutex<u64>) -> u64 {
    *mutex.lock()
}

}
