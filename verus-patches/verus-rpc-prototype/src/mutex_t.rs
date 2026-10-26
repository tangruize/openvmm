//! Trusted wrapper around `parking_lot::Mutex` with a lock invariant, in the
//! style of `vstd::rwlock::RwLock`.
//!
//! Everything in this file is TRUSTED. The runtime lock is the real
//! `parking_lot::Mutex`; the value is kept as `Option<V>` so that verified
//! code can move it out while it holds the lock.
//!
//! - Verified code calls `acquire`, which gives the value (satisfying the
//!   invariant) and a handle, and `MutexHandle::release`, which requires the
//!   invariant of the value put back.
//! - A handle dropped without `release` leaves `None` behind, so the next
//!   `lock`/`acquire` panics. This is safe, not unsound: no one observes a
//!   value that breaks the invariant.
//! - Unverified code calls `lock`, which returns an ordinary guard. It is
//!   `#[verifier::external]`, so verified code cannot call it. ASSUMPTION:
//!   unverified code keeps the invariant.

use parking_lot::MappedMutexGuard;
use parking_lot::MutexGuard;
use vstd::prelude::*;
use vstd::rwlock::RwLockPredicate;

verus! {

/// A mutex whose value always satisfies `pred.inv(v)` while unlocked.
#[verifier::external_body]
#[verifier::reject_recursive_types(V)]
#[verifier::reject_recursive_types(Pred)]
pub struct Mutex<V, Pred: RwLockPredicate<V>> {
    inner: parking_lot::Mutex<Option<V>>,
    pred: Ghost<Pred>,
}

/// Holds the lock of a `Mutex` whose value has been taken out by `acquire`.
#[verifier::external_body]
#[verifier::reject_recursive_types(V)]
#[verifier::reject_recursive_types(Pred)]
pub struct MutexHandle<'a, V, Pred: RwLockPredicate<V>> {
    guard: MutexGuard<'a, Option<V>>,
    pred: Ghost<Pred>,
}

impl<V, Pred: RwLockPredicate<V>> Mutex<V, Pred> {
    /// The invariant.
    pub uninterp spec fn pred(&self) -> Pred;

    pub open spec fn inv(&self, v: V) -> bool {
        self.pred().inv(v)
    }

    #[verifier::external_body]
    pub fn new(v: V, pred: Ghost<Pred>) -> (s: Self)
        requires
            pred@.inv(v),
        ensures
            s.pred() == pred@,
    {
        Mutex { inner: parking_lot::Mutex::new(Some(v)), pred }
    }

    /// Locks, and takes the value out until `release`.
    #[verifier::external_body]
    pub fn acquire(&self) -> (ret: (V, MutexHandle<'_, V, Pred>))
        ensures
            self.inv(ret.0),
            ret.1.pred() == self.pred(),
    {
        let mut guard = self.inner.lock();
        let v = guard.take().expect("mutex value not released");
        (v, MutexHandle { guard, pred: self.pred })
    }
}

impl<'a, V, Pred: RwLockPredicate<V>> MutexHandle<'a, V, Pred> {
    pub uninterp spec fn pred(&self) -> Pred;

    /// Puts `v` back and unlocks.
    #[verifier::external_body]
    pub fn release(self, v: V)
        requires
            self.pred().inv(v),
    {
        let mut guard = self.guard;
        *guard = Some(v);
    }
}

} // verus!

impl<V, Pred: RwLockPredicate<V>> Mutex<V, Pred> {
    /// Locks, for unverified code. ASSUMPTION: the caller keeps the invariant.
    #[cfg_attr(verus_keep_ghost, verifier::external)]
    pub fn lock(&self) -> MappedMutexGuard<'_, V> {
        MutexGuard::map(self.inner.lock(), |v| v.as_mut().expect("mutex value not released"))
    }
}
