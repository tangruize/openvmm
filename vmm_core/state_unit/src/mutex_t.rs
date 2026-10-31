// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

//! Trusted wrapper around `parking_lot::Mutex` with a lock invariant, in the
//! style of `vstd::rwlock::RwLock`: each mutex has its own invariant, chosen
//! when it is created. Its form is given by the predicate type
//! (`MutexPredicate::inv(param, v)`), and each mutex has its own ghost
//! parameter (`param()`, given to `new_verified`), so a mutex's invariant can
//! name that mutex's own ghost state. Used only for Verus; the runtime lock
//! is the real `parking_lot::Mutex`.
//!
//! TRUSTED, do not attempt to prove anything in this file: Verus uses only
//! the contracts here; the wrapper is a lock primitive we do not verify, over
//! `parking_lot`, which is external (`SNAPSHOT-SPEC-TRUSTED.md`, Part 1).
//! Its unverified users are `external_body`, to prove.
//!
//! Everything in this file is TRUSTED.
//!
//! - The value is kept as `Option<V>`, so that verified code can move it out
//!   while it holds the lock: `acquire` gives the value, which satisfies the
//!   invariant `Pred::inv`, and `MutexHandle::release` puts a value back and
//!   requires the invariant.
//! - A handle dropped without `release` leaves `None` behind, so the next
//!   `lock` or `acquire` panics. This is safe, not unsound: no one observes a
//!   value that breaks the invariant.
//! - Unverified code calls `lock`, which returns an ordinary guard. It is
//!   hidden from Verus, so verified code cannot call it. ASSUMPTION:
//!   unverified code keeps the invariant.
//! - Unverified code may also create a mutex with `new` (hidden from Verus);
//!   its parameter is then unknown. ASSUMPTION: its value satisfies the
//!   invariant for its parameter. Verified code creates one with
//!   `new_verified`, which requires it.

use parking_lot::MappedMutexGuard;
use parking_lot::MutexGuard;
use std::fmt::Debug;
use std::marker::PhantomData;
use vstd::prelude::*;

verus! {

/// The invariant of a `Mutex<V, Pred>`'s value while unlocked, given each
/// mutex's own parameter.
pub trait MutexPredicate<V> {
    /// The parameter each mutex gives the invariant when it is created.
    type Param;

    /// The invariant, for a mutex with parameter `param`.
    spec fn inv(param: Self::Param, v: V) -> bool;
}

/// A mutex whose value satisfies `Pred::inv(self.param(), _)` while
/// unlocked.
#[verifier::external_body]
#[verifier::reject_recursive_types(V)]
#[verifier::reject_recursive_types(Pred)]
pub struct Mutex<V, Pred> {
    inner: parking_lot::Mutex<Option<V>>,
    _pred: PhantomData<Pred>,
}

/// Holds the lock of a `Mutex` whose value was taken out by `acquire`.
#[verifier::external_body]
#[verifier::reject_recursive_types(V)]
#[verifier::reject_recursive_types(Pred)]
pub struct MutexHandle<'a, V, Pred> {
    guard: MutexGuard<'a, Option<V>>,
    _pred: PhantomData<Pred>,
}

impl<V, Pred: MutexPredicate<V>> Mutex<V, Pred> {
    /// The mutex's parameter of its invariant, given when it was created.
    pub uninterp spec fn param(&self) -> Pred::Param;

    /// Creates a mutex holding `v`, whose invariant has parameter `param`.
    #[verifier::external_body]
    #[allow(unused_variables)]  // the ghost parameter is erased at runtime
    pub fn new_verified(v: V, param: Ghost<Pred::Param>) -> (s: Self)
        requires
            Pred::inv(param@, v),
        ensures
            s.param() == param@,
    {
        Mutex { inner: parking_lot::Mutex::new(Some(v)), _pred: PhantomData }
    }

    /// Locks, and takes the value out until `release`.
    #[verifier::external_body]
    pub fn acquire(&self) -> (ret: (V, MutexHandle<'_, V, Pred>))
        ensures
            Pred::inv(self.param(), ret.0),
            ret.1.param() == self.param(),
    {
        let mut guard = self.inner.lock();
        let v = guard.take().expect("mutex value not released");
        (v, MutexHandle { guard, _pred: PhantomData })
    }
}

impl<'a, V, Pred: MutexPredicate<V>> MutexHandle<'a, V, Pred> {
    /// The mutex's parameter of its invariant.
    pub uninterp spec fn param(&self) -> Pred::Param;

    /// Puts `v` back and unlocks; requires the invariant.
    #[verifier::external_body]
    pub fn release(self, v: V)
        requires
            Pred::inv(self.param(), v),
    {
        let mut guard = self.guard;
        *guard = Some(v);
    }
}

} // verus!

// In a normal build the trait has no spec functions, so every type
// satisfies it.
#[cfg(not(verus_keep_ghost))]
impl<V, T> MutexPredicate<V> for T {
    type Param = ();
}

impl<V, Pred> Mutex<V, Pred> {
    /// Creates a mutex, for unverified code. ASSUMPTION: `v` satisfies the
    /// invariant for the mutex's (unknown) parameter.
    #[cfg_attr(verus_keep_ghost, verifier::external)]
    pub fn new(v: V) -> Self {
        Mutex {
            inner: parking_lot::Mutex::new(Some(v)),
            _pred: PhantomData,
        }
    }

    /// Locks, for unverified code. ASSUMPTION: the caller keeps the invariant.
    #[cfg_attr(verus_keep_ghost, verifier::external)]
    pub fn lock(&self) -> MappedMutexGuard<'_, V> {
        MutexGuard::map(self.inner.lock(), |v| {
            v.as_mut().expect("mutex value not released")
        })
    }
}

impl<V: Debug, Pred> Debug for Mutex<V, Pred> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.inner.fmt(f)
    }
}
