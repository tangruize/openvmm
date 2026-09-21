// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

// Unapproved experimental interface; never included by live production.
use parking_lot::lock_api;
use std::ops::Deref;
use vstd::prelude::*;

verus! {

#[verifier::external_trait_specification]
pub trait ExRawMutexTrait {
    type ExternalTraitSpecificationFor: lock_api::RawMutex;
}

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExRawMutex(parking_lot::RawMutex);

#[verifier::external_type_specification]
#[verifier::external_body]
#[verifier::reject_recursive_types(R)]
#[verifier::reject_recursive_types(T)]
pub struct ExMutex<R, T: ?Sized>(lock_api::Mutex<R, T>);

#[verifier::external_type_specification]
#[verifier::external_body]
#[verifier::reject_recursive_types(R)]
#[verifier::reject_recursive_types(T)]
pub struct ExMutexGuard<'a, R: lock_api::RawMutex, T: ?Sized>(lock_api::MutexGuard<'a, R, T>);

/// The mutex reference stored in this acquired guard, not its current contents.
pub uninterp spec fn guard_origin<'a, R: lock_api::RawMutex, T: ?Sized>(
    guard: &lock_api::MutexGuard<'a, R, T>,
) -> &'a lock_api::Mutex<R, T>;

/// This immutable guard borrow's protected data reference.
pub uninterp spec fn guard_observation<'a, 'b, R: lock_api::RawMutex, T: ?Sized>(
    guard: &'b lock_api::MutexGuard<'a, R, T>,
) -> &'b T;

#[verifier::external_fn_specification]
pub fn mutex_lock<'a, R: lock_api::RawMutex, T: ?Sized>(
    mutex: &'a lock_api::Mutex<R, T>,
) -> (guard: lock_api::MutexGuard<'a, R, T>)
    ensures guard_origin(&guard) == mutex,
{
    mutex.lock()
}

#[verifier::external_fn_specification]
#[verifier::when_used_as_spec(guard_observation)]
pub fn guard_deref<'a, 'b, R: lock_api::RawMutex + 'a, T: ?Sized + 'a>(
    guard: &'b lock_api::MutexGuard<'a, R, T>,
) -> (value: &'b T)
    ensures value == guard_observation(guard),
{
    guard.deref()
}

}
