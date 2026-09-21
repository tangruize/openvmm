// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

include!("candidate.spec.rs");
use parking_lot::Mutex;

verus! {

pub fn same_guard(mutex: &Mutex<u64>) {
    let guard = mutex.lock();
    let a = &*guard;
    let b = &*guard;
    assert(a == b);
    assert(*a == *guard_observation(&guard));
    assert(guard_origin(&guard) == mutex);
}

pub fn distinguish_origins(a: &Mutex<u64>, b: &Mutex<u64>)
    requires a != b,
{
    let guard = a.lock();
    assert(guard_origin(&guard) != b);
}

pub fn mutable_borrow(mutex: &Mutex<u64>) {
    let mut guard = mutex.lock();
    let borrowed = &mut *guard;
    *borrowed = 7;
    assert(*borrowed == 7);
}

pub fn mutation_has_no_guard_frame(mutex: &Mutex<u64>) {
    let mut guard = mutex.lock();
    *guard = 7;
    assert(*guard == 7);
    assert(guard_origin(&guard) == mutex);
}

pub fn cross_mutex_contents(a: &Mutex<u64>, b: &Mutex<u64>) {
    let ga = a.lock();
    let gb = b.lock();
    assert(*ga == *gb);
}

pub fn cross_mutex_origin(a: &Mutex<u64>, b: &Mutex<u64>) {
    let ga = a.lock();
    assert(guard_origin(&ga) == b);
}

pub fn reacquisition(mutex: &Mutex<u64>) {
    let before = {
        let guard = mutex.lock();
        *guard
    };
    let after = *mutex.lock();
    assert(after == before);
}

pub fn release_and_mutate(mutex: &Mutex<u64>) {
    let before = {
        let guard = mutex.lock();
        *guard
    };
    {
        let mut guard = mutex.lock();
        *guard = 7;
    }
    let after = *mutex.lock();
    assert(after == before);
}

pub fn observation_after_mutation(mutex: &Mutex<u64>) {
    let mut guard = mutex.lock();
    let ghost before = *guard_observation(&guard);
    *guard = 7;
    assert(*guard_observation(&guard) == before);
}

}
