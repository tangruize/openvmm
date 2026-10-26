// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

//! Trusted Verus specification of [`CloseableMutex`]: a lock invariant on the
//! value, given by the value's type ([`LockInvariant`]). Used only for Verus;
//! the runtime mutex is the real [`CloseableMutex`].
//!
//! Everything in this file is TRUSTED.
//!
//! - Verified code closes the mutex with [`CloseableMutex::close_verified`],
//!   which gives a [`VerifiedClosedGuard`] whose value satisfies the
//!   invariant, and reopens it with [`VerifiedClosedGuard::release`], which
//!   requires the invariant again.
//! - A [`VerifiedClosedGuard`] dropped without `release` leaves the mutex
//!   closed, so later `lock` and `close` calls panic. This is safe, not
//!   unsound: no one observes a value that breaks the invariant.
//! - Unverified code uses `lock`, `lock_if_open`, and `close` as before.
//!   ASSUMPTION: it keeps the invariant (for a device: it changes the device
//!   only through methods whose contracts keep the device's `wf()`).
//! - ASSUMPTION: a mutex created by unverified code with
//!   [`CloseableMutex::new`] is as if created by
//!   [`CloseableMutex::new_verified`]: its value satisfied the invariant, and
//!   its key is that value's key.

use super::ClosedGuard;
use super::CloseableMutex;
use std::sync::Arc;
use vstd::prelude::*;

verus! {

/// The invariant of a [`CloseableMutex<Self>`]'s value while the mutex is
/// neither locked nor closed.
pub trait LockInvariant {
    /// A constant of the value that holders of the lock keep (for a device:
    /// the id of its FracGhost), so that verified code can relate the values
    /// it sees on different acquisitions.
    type Key;

    /// The value's key.
    spec fn key(&self) -> Self::Key;

    /// The invariant.
    spec fn inv(&self) -> bool;
}

#[cfg(verus_keep_ghost)]
#[verifier::external_type_specification]
#[verifier::external_body]
#[verifier::reject_recursive_types(T)]
pub struct ExCloseableMutex<T: ?Sized>(CloseableMutex<T>);

/// A closed [`CloseableMutex`], for verified code. Reopen it with
/// [`VerifiedClosedGuard::release`].
#[verifier::external_body]
#[verifier::reject_recursive_types(T)]
pub struct VerifiedClosedGuard<T> {
    /// `None` once released.
    guard: Option<ClosedGuard<T>>,
}

impl<T: LockInvariant> CloseableMutex<T> {
    /// The key of the value, the same on every acquisition.
    pub uninterp spec fn key(&self) -> T::Key;

    /// Creates a mutex holding `value`, with `value`'s key.
    #[verifier::external_body]
    pub fn new_verified(value: T) -> (m: Self)
        requires
            value.inv(),
        ensures
            m.key() == value.key(),
    {
        CloseableMutex::new(value)
    }

    /// Closes the mutex, for verified code. Panics if it is already closed.
    #[verifier::external_body]
    pub fn close_verified(m: Arc<Self>) -> (g: VerifiedClosedGuard<T>)
        ensures
            g.key() == (*m).key(),
            g.value().key() == (*m).key(),
            g.value().inv(),
    {
        VerifiedClosedGuard { guard: Some(m.close()) }
    }
}

impl<T: LockInvariant> VerifiedClosedGuard<T> {
    /// The key of the mutex.
    pub uninterp spec fn key(&self) -> T::Key;

    /// The value in the mutex.
    pub uninterp spec fn value(&self) -> T;

    /// The value, to read.
    #[verifier::external_body]
    pub fn borrow(&self) -> (r: &T)
        ensures
            *r == self.value(),
    {
        self.guard.as_ref().unwrap()
    }

    /// The value, to change; the guard holds what it is changed to.
    #[verifier::external_body]
    pub fn borrow_mut(&mut self) -> (r: &mut T)
        ensures
            *r == old(self).value(),
            final(self).value() == *final(r),
            final(self).key() == old(self).key(),
    {
        self.guard.as_mut().unwrap()
    }

    /// Reopens the mutex; requires the invariant, and the key unchanged.
    #[verifier::external_body]
    pub fn release(self) -> (m: Arc<CloseableMutex<T>>)
        requires
            self.value().inv(),
            self.value().key() == self.key(),
        ensures
            (*m).key() == self.key(),
    {
        let mut this = self;
        this.guard.take().unwrap().open()
    }
}

} // verus!

impl<T> Drop for VerifiedClosedGuard<T> {
    fn drop(&mut self) {
        // Not released: keep the mutex closed (see the module comment).
        if let Some(guard) = self.guard.take() {
            std::mem::forget(guard);
        }
    }
}
