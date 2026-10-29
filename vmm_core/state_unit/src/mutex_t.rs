// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

//! Trusted wrapper around `parking_lot::Mutex` with a lock invariant, in the
//! style of `vstd::rwlock::RwLock`. Used only for Verus; the runtime lock is
//! the real `parking_lot::Mutex`.
//!
//! TRUSTED, do not attempt to prove anything in this file: Verus uses only
//! the contracts here; the wrapper is a lock primitive we do not verify, over
//! `parking_lot`, which is external (`SNAPSHOT-SPEC-TRUSTED.md`, Part 1).
//! Its unverified users are `external_body`, to prove (Part 3, "The locks'
//! unverified users").
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
//!   unverified code keeps the invariant and the key.
//! - A mutex has a key, `Pred::key` of its value, which every holder keeps
//!   (`release` requires it), so that verified code can relate the values it
//!   sees on different acquisitions (as `closeable_mutex::LockInvariant`).

use parking_lot::MappedMutexGuard;
use parking_lot::MutexGuard;
use std::fmt::Debug;
use std::marker::PhantomData;
use vstd::prelude::*;

verus! {

/// The invariant of a `Mutex<V, Pred>`'s value while unlocked, and its key.
pub trait MutexPredicate<V> {
    /// A constant of the value that every holder keeps.
    type Key;

    spec fn key(v: V) -> Self::Key;

    spec fn inv(v: V) -> bool;
}

/// A mutex whose value satisfies `Pred::inv` while unlocked.
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
    /// The key of the value, the same on every acquisition.
    pub uninterp spec fn key(&self) -> Pred::Key;

    #[verifier::external_body]
    pub fn new(v: V) -> (s: Self)
        requires
            Pred::inv(v),
        ensures
            s.key() == Pred::key(v),
    {
        Mutex { inner: parking_lot::Mutex::new(Some(v)), _pred: PhantomData }
    }

    /// Locks, and takes the value out until `release`.
    #[verifier::external_body]
    pub fn acquire(&self) -> (ret: (V, MutexHandle<'_, V, Pred>))
        ensures
            Pred::inv(ret.0),
            Pred::key(ret.0) == self.key(),
            ret.1.key() == self.key(),
    {
        let mut guard = self.inner.lock();
        let v = guard.take().expect("mutex value not released");
        (v, MutexHandle { guard, _pred: PhantomData })
    }
}

impl<'a, V, Pred: MutexPredicate<V>> MutexHandle<'a, V, Pred> {
    /// The key of the mutex.
    pub uninterp spec fn key(&self) -> Pred::Key;

    /// Puts `v` back and unlocks; requires the invariant and the key.
    #[verifier::external_body]
    pub fn release(self, v: V)
        requires
            Pred::inv(v),
            Pred::key(v) == self.key(),
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
    type Key = ();
}

impl<V, Pred> Mutex<V, Pred> {
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
