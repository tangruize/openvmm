// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

//! Verus specification of [`CloseableMutex`]: a lock invariant on the
//! value, chosen for each mutex when it is created, as for
//! `vstd::rwlock::RwLock`. Its form is given by the value's type
//! ([`LockInvariant`]: `inv(&self, param)`), and each mutex has its own
//! ghost parameter (`param()`, given to `new_verified`), so a mutex's
//! invariant can name that mutex's own ghost state (for a device: its
//! FracGhost's id). Used only for Verus; the runtime mutex is the real
//! [`CloseableMutex`].
//!
//! TRUSTED, do not attempt to prove anything in this file: Verus uses only
//! the contracts here. `CloseableMutex` is a lock primitive we do not verify
//! (the user's decision, 2026-10-30), so these contracts are in the trusted
//! base (`SNAPSHOT-SPEC-TRUSTED.md`, Part 1). Its unverified users (plain
//! `lock`, `lock_if_open`, `close`, `new`) are not: they are
//! `external_body` functions, to prove; once verified, Verus checks the invariant at each `release` and
//! at `new_verified`.
//!
//! - **Closing, and the panics.** A closed mutex panics on `lock`, and
//!   closing a closed one panics too. To make "not closed" provable, each
//!   mutex has a **close token** (a `FracGhost<()>`, id `close_id()`),
//!   handed out whole by `new_verified`. Closing requires the whole token
//!   and keeps it in the closed guard until it is reopened, which gives it
//!   back; locking requires a piece of it. So whoever holds a piece knows
//!   the mutex is not closed: neither `lock_verified` nor `close_verified`
//!   can panic.
//! - Verified code closes the mutex with [`CloseableMutex::close_verified`],
//!   which gives a [`VerifiedClosedGuard`] whose value satisfies the
//!   invariant, and reopens it with [`VerifiedClosedGuard::release`], which
//!   requires the invariant again.
//! - A [`VerifiedClosedGuard`] dropped without `release` leaves the mutex
//!   closed, so later `lock` and `close` calls panic. This is safe, not
//!   unsound: no one observes a value that breaks the invariant.
//! - Likewise, verified code locks the mutex (without closing it) with
//!   [`CloseableMutex::lock_verified`] and unlocks with
//!   [`VerifiedOpenGuard::release`]. A [`VerifiedOpenGuard`] dropped without
//!   `release` leaves the mutex locked: later lockers wait forever. Safe, not
//!   unsound, for the same reason.
//! - Unverified code uses `lock`, `lock_if_open`, `close`, and `new` as
//!   before; those users are `external_body`, to prove (see above).

use super::CloseableMutex;
use super::ClosedGuard;
use super::OpenGuard;
use std::sync::Arc;
use vstd::prelude::*;
use vstd::resource::Loc;
use vstd::resource::frac::FracGhost;

verus! {

/// The invariant of a [`CloseableMutex<Self>`]'s value while the mutex is
/// neither locked nor closed, given each mutex's own parameter.
pub trait LockInvariant {
    /// The parameter each mutex gives the invariant when it is created
    /// (for a device: the id of its FracGhost, and whether its unit omits
    /// its saved state).
    type Param;

    /// The invariant, for a mutex with parameter `param`.
    spec fn inv(&self, param: Self::Param) -> bool;
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

impl<T: ?Sized + LockInvariant> CloseableMutex<T> {
    /// The mutex's parameter of its invariant, given when it was created.
    /// (Also for an unsized value, such as a device seen through a trait
    /// object.)
    pub uninterp spec fn param(&self) -> T::Param;

    /// The id of the mutex's close token: the whole token is needed to
    /// close the mutex, a piece of it to lock it.
    pub uninterp spec fn close_id(&self) -> Loc;
}

impl<T: LockInvariant> CloseableMutex<T> {
    /// Creates a mutex holding `value`, whose invariant has parameter
    /// `param`, and hands out its whole close token.
    #[verifier::external_body]
    #[allow(unused_variables)]  // the ghost parameter is erased at runtime
    pub fn new_verified(value: T, param: Ghost<T::Param>) -> (r: (Self, Tracked<FracGhost<()>>))
        requires
            value.inv(param@),
        ensures
            r.0.param() == param@,
            r.1@.valid(r.0.close_id(), 1real),
    {
        // TRUSTED: keep this `assume_new()`. This trusted function's
        // contract hands out the close token; its body has none to give.
        (CloseableMutex::new(value), Tracked::assume_new())
    }

    /// Closes the mutex, for verified code. Requires the whole close token
    /// (so the mutex is not closed: closing a closed mutex panics), which
    /// the closed guard keeps until it is reopened.
    #[verifier::external_body]
    #[allow(unused_variables)]  // the token is erased at runtime
    pub fn close_verified(m: &Arc<Self>, token: Tracked<FracGhost<()>>) -> (g: VerifiedClosedGuard<T>)
        requires
            token@.valid((**m).close_id(), 1real),
        ensures
            g.param() == (**m).param(),
            g.close_id() == (**m).close_id(),
            g.value().inv(g.param()),
    {
        VerifiedClosedGuard { guard: Some(m.clone().close()) }
    }
}

/// A locked [`CloseableMutex`], for verified code. Unlock it with
/// [`VerifiedOpenGuard::release`].
#[verifier::external_body]
#[verifier::reject_recursive_types(T)]
pub struct VerifiedOpenGuard<'a, T> {
    /// `None` once released.
    guard: Option<OpenGuard<'a, T>>,
}

impl<T: LockInvariant> CloseableMutex<T> {
    /// Locks the mutex, for verified code. Requires a piece of the close
    /// token, so the mutex is not closed (locking a closed mutex panics):
    /// nobody else can hold the whole token, which closing needs.
    #[verifier::external_body]
    #[allow(unused_variables)]  // the token is erased at runtime
    pub fn lock_verified(&self, token: Tracked<&FracGhost<()>>) -> (g: VerifiedOpenGuard<'_, T>)
        requires
            token@.id() == self.close_id(),
        ensures
            g.param() == self.param(),
            g.value().inv(g.param()),
    {
        VerifiedOpenGuard { guard: Some(self.lock()) }
    }
}

impl<'a, T: LockInvariant> VerifiedOpenGuard<'a, T> {
    /// The mutex's parameter of its invariant.
    pub uninterp spec fn param(&self) -> T::Param;

    /// The value in the mutex.
    pub uninterp spec fn value(&self) -> T;

    /// The value, to change; the guard holds what it is changed to.
    #[verifier::external_body]
    pub fn borrow_mut(&mut self) -> (r: &mut T)
        ensures
            *r == old(self).value(),
            final(self).value() == *final(r),
            final(self).param() == old(self).param(),
    {
        self.guard.as_mut().unwrap()
    }

    /// Unlocks the mutex; requires the invariant.
    #[verifier::external_body]
    pub fn release(self)
        requires
            self.value().inv(self.param()),
    {
        let mut this = self;
        drop(this.guard.take());
    }
}

impl<T: LockInvariant> VerifiedClosedGuard<T> {
    /// The mutex's parameter of its invariant.
    pub uninterp spec fn param(&self) -> T::Param;

    /// The id of the mutex's close token, which the guard keeps.
    pub uninterp spec fn close_id(&self) -> Loc;

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
            final(self).param() == old(self).param(),
    {
        self.guard.as_mut().unwrap()
    }

    /// Reopens the mutex; requires the invariant. Gives back the whole
    /// close token.
    #[verifier::external_body]
    pub fn release(self) -> (r: (Arc<CloseableMutex<T>>, Tracked<FracGhost<()>>))
        requires
            self.value().inv(self.param()),
        ensures
            (*r.0).param() == self.param(),
            (*r.0).close_id() == self.close_id(),
            r.1@.valid(self.close_id(), 1real),
    {
        let mut this = self;
        // TRUSTED: keep this `assume_new()`. This trusted function's
        // contract gives back the close token; its body has none to give.
        (this.guard.take().unwrap().open(), Tracked::assume_new())
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

impl<T> Drop for VerifiedOpenGuard<'_, T> {
    fn drop(&mut self) {
        // Not released: keep the mutex locked (see the module comment).
        if let Some(guard) = self.guard.take() {
            std::mem::forget(guard);
        }
    }
}
