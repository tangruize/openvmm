// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

//! Provides an `Option`-like type for constructing values in place.

use alloc::boxed::Box;
use alloc::sync::Arc;
use core::mem::MaybeUninit;
use vstd::prelude::*;

#[cfg(verus_keep_ghost)]
include!("inplace.proof.rs");

/// A type with methods like `Option` but that operates on a mutable reference
/// to possibly-initialized data.
///
/// This is used to initialize data in place without copying to/from `Option`
/// types.
#[verus_verify]
pub struct InplaceOption<'a, T> {
    val: &'a mut MaybeUninit<T>,
    init: bool,
}

#[verus_verify]
impl<'a, T> InplaceOption<'a, T> {
    /// Creates an option in the uninitialized state.
    #[verus_spec(result =>
        ensures
            !result.initialized(),
            result.contents() == old(val).mem_contents(),
            result.storage_valid(),
            final(val).mem_contents() == final(result.storage()).mem_contents(),
    )]
    pub fn uninit(val: &'a mut MaybeUninit<T>) -> Self {
        Self { val, init: false }
    }

    /// Creates an option in the initialized state.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that the value referenced by `val` is
    /// initialized.
    #[verus_spec(result =>
        requires old(val).mem_contents().is_init(),
        ensures
            result.initialized(),
            result.contents() == old(val).mem_contents(),
            result.storage_valid(),
            final(val).mem_contents() == final(result.storage()).mem_contents(),
    )]
    pub unsafe fn new_init_unchecked(val: &'a mut MaybeUninit<T>) -> Self {
        Self { val, init: true }
    }

    /// Sets the value to the initialized state.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that the underlying data has been fully
    /// initialized.
    #[verus_spec(result =>
        requires old(self).contents().is_init(),
        ensures
            *result == old(self).contents().value(),
            final(self).initialized(),
            final(self).contents() == vstd::raw_ptr::MemContents::Init(*final(result)),
            final(self).storage_valid(),
            final(old(self).storage()).mem_contents()
                == final(final(self).storage()).mem_contents(),
    )]
    pub unsafe fn set_init_unchecked(&mut self) -> &mut T {
        self.init = true;
        // SAFETY: the caller guarantees val is initialized.
        unsafe { self.val.assume_init_mut() }
    }
}

impl<'a, T> InplaceOption<'a, T> {
    /// Takes the value, returning `Some(_)` if the value is initialized and
    /// `None` otherwise.
    #[verus_verify]
    #[verus_spec(result =>
        requires old(self).storage_valid(),
        ensures
            !final(self).initialized(),
            final(self).storage_valid(),
            final(old(self).storage()).mem_contents()
                == final(final(self).storage()).mem_contents(),
            result == if old(self).initialized() {
                Some(old(self).contents().value())
            } else {
                None
            },
    )]
    pub fn take(&mut self) -> Option<T> {
        if self.init {
            self.init = false;
            // SAFETY: val is initialized
            unsafe {
                let val = core::ptr::read(&*self.val);
                Some(val.assume_init())
            }
        } else {
            None
        }
    }

    /// Returns a reference to the data if it's initialized.
    pub fn as_ref(&self) -> Option<&T> {
        if self.init {
            // SAFETY: We have just checked that val is initialized
            unsafe { self.val.as_ptr().as_ref() }
        } else {
            None
        }
    }

    /// Returns a mutable reference to the data if it's initialized.
    #[verus_verify]
    #[verus_spec(result =>
        requires old(self).storage_valid(),
        ensures
            final(self).initialized() == old(self).initialized(),
            final(self).storage_valid(),
            match result {
                Some(value) => old(self).initialized()
                    && *value == old(self).contents().value()
                    && final(self).contents() == vstd::raw_ptr::MemContents::Init(*final(value)),
                None => !old(self).initialized()
                    && final(self).contents() == old(self).contents(),
            },
    )]
    pub fn as_mut(&mut self) -> Option<&mut T> {
        if self.init {
            // SAFETY: val is initialized
            Some(unsafe { self.val.assume_init_mut() })
        } else {
            None
        }
    }

    /// Clears the data to the uninitialized state.
    // This proof interface covers Copy values. Destructive non-Copy cleanup
    // needs its own contract; the executable operation remains unrestricted.
    #[verus_verify(preserve_source)]
    #[verus_spec(
        with Tracked(copy): Tracked<NativeCopy<T>>,
        requires old(self).storage_valid(),
        ensures
            !final(self).initialized(),
            final(self).storage_valid(),
            final(self).contents() == old(self).contents(),
            final(old(self).storage()).mem_contents()
                == final(final(self).storage()).mem_contents(),
    )]
    pub fn clear(&mut self) {
        if self.init {
            self.init = false;
            // SAFETY: val is initialized
            proof_with! { native_copy_drop(Tracked(copy)) }
            unsafe { self.val.assume_init_drop() };
        }
    }

    /// Resets the data to the uninitialized state without dropping any
    /// initialized value.
    #[verus_verify]
    #[verus_spec(result =>
        ensures
            result == old(self).initialized(),
            !final(self).initialized(),
            final(self).contents() == old(self).contents(),
            final(self).storage_valid(),
            final(old(self).storage()).mem_contents()
                == final(final(self).storage()).mem_contents(),
    )]
    pub fn forget(&mut self) -> bool {
        core::mem::take(&mut self.init)
    }

    /// Initializes the value to `v`, dropping any existing value first.
    #[verus_verify]
    #[verus_spec(result =>
        with Tracked(copy): Tracked<NativeCopy<T>>,
        requires old(self).storage_valid(),
        ensures
            *result == v,
            final(self).initialized(),
            final(self).storage_valid(),
            final(self).contents() == vstd::raw_ptr::MemContents::Init(*final(result)),
            final(old(self).storage()).mem_contents()
                == final(final(self).storage()).mem_contents(),
    )]
    pub fn set(&mut self, v: T) -> &mut T {
        proof_with! { Tracked(copy) }
        self.clear();
        self.init = true;
        self.val.write(v)
    }

    /// Gets a mutable reference to the value, setting it to `v` first if it's
    /// not initialized.
    pub fn get_or_insert(&mut self, v: T) -> &mut T {
        self.get_or_insert_with(|| v)
    }

    /// Gets a mutable reference to the value, setting it to `f()` first if it's
    /// not initialized.
    pub fn get_or_insert_with(&mut self, f: impl FnOnce() -> T) -> &mut T {
        if self.init {
            // SAFETY: val is initialized
            unsafe { self.val.assume_init_mut() }
        } else {
            self.init = true;
            self.val.write(f())
        }
    }

    /// Returns whether the value is initialized.
    pub fn is_some(&self) -> bool {
        self.init
    }

    /// Returns whether the value is uninitialized.
    pub fn is_none(&self) -> bool {
        !self.init
    }

    /// Returns a const pointer to the underlying value (initialized or not).
    pub fn as_ptr(&self) -> *const T {
        self.val.as_ptr()
    }

    /// Returns a mut pointer to the underlying value (initialized or not).
    #[verus_verify]
    #[verus_spec(result =>
        with -> loan: Tracked<NativeMutLoan<'_, T>>,
        ensures
            loan@.ptr() == result,
            loan@.storage().mem_contents() == old(self).contents(),
            final(self).contents() == final(loan@.storage()).mem_contents(),
            final(self).initialized() == old(self).initialized(),
            !old(self).initialized() ==> final(self).storage_valid(),
            final(old(self).storage()).mem_contents()
                == final(final(self).storage()).mem_contents(),
    )]
    pub fn as_mut_ptr(&mut self) -> *mut T {
        proof_with! { native_loan }
        self.val.as_mut_ptr()
    }
}

impl<T> InplaceOption<'_, Box<T>> {
    /// Updates a boxed value in place.
    ///
    /// N.B. This will allocate space for a value if one is not already present,
    ///      which is wasteful if `f` does not actually initialize the value.
    pub fn update_box<F, R>(&mut self, f: F) -> R
    where
        F: FnOnce(&mut InplaceOption<'_, T>) -> R,
    {
        let mut boxed;
        let mut inplace;

        if let Some(b) = self.take() {
            // SAFETY: MaybeUninit<T> has the same layout as T.
            boxed = unsafe { Box::from_raw(Box::into_raw(b).cast::<MaybeUninit<T>>()) };
            // SAFETY: the value is known to be initialized.
            inplace = unsafe { InplaceOption::new_init_unchecked(&mut *boxed) };
        } else {
            boxed = Box::new(MaybeUninit::uninit());
            inplace = InplaceOption::uninit(&mut *boxed);
        }

        let r = f(&mut inplace);
        if inplace.forget() {
            drop(inplace);
            // SAFETY: T has the same layout as MaybeUninit<T>, and the value is
            // known to be initialized.
            let b = unsafe { Box::from_raw(Box::into_raw(boxed).cast::<T>()) };
            self.set(b);
        }
        r
    }
}

impl<T: Clone> InplaceOption<'_, Arc<T>> {
    /// Updates a reference counted value in place.
    ///
    /// N.B. This will allocate space for a value if one is not already present,
    ///      which is wasteful if `f` does not actually initialize the value.
    pub fn update_arc<F, R>(&mut self, f: F) -> R
    where
        F: FnOnce(&mut InplaceOption<'_, T>) -> R,
    {
        let mut arced;
        let mut inplace;

        if let Some(mut a) = self.take() {
            // Ensure there is only a single reference.
            Arc::make_mut(&mut a);
            // SAFETY: MaybeUninit<T> has the same layout as T.
            arced = unsafe { Arc::from_raw(Arc::into_raw(a).cast::<MaybeUninit<T>>()) };
            // SAFETY: the value is known to be initialized.
            unsafe {
                inplace = InplaceOption::new_init_unchecked(Arc::get_mut(&mut arced).unwrap())
            };
        } else {
            arced = Arc::new(MaybeUninit::uninit());
            inplace = InplaceOption::uninit(Arc::get_mut(&mut arced).unwrap());
        }

        let r = f(&mut inplace);
        if inplace.forget() {
            drop(inplace);
            // SAFETY: T has the same layout as MaybeUninit<T>, and the value is
            // known to be initialized.
            let a = unsafe { Arc::from_raw(Arc::into_raw(arced).cast::<T>()) };
            self.set(a);
        }
        r
    }
}

impl<T> Drop for InplaceOption<'_, T> {
    fn drop(&mut self) {
        self.clear();
    }
}

/// Constructs a possibly-initialized [`crate::inplace::InplaceOption`] on the stack
/// from an `Option<T>`.
#[macro_export]
macro_rules! inplace {
    ($v:ident) => {
        let opt = $v;
        let mut $v;
        let mut $v = match opt {
            Some(v) => {
                $v = core::mem::MaybeUninit::new(v);
                // SAFETY: We just initialized the value.
                unsafe { $crate::inplace::InplaceOption::new_init_unchecked(&mut $v) }
            }
            None => {
                $v = core::mem::MaybeUninit::uninit();
                $crate::inplace::InplaceOption::uninit(&mut $v)
            }
        };
    };
}

/// Constructs an initialized [`crate::inplace::InplaceOption`] on the stack from a `T`.
#[macro_export]
macro_rules! inplace_some {
    ($v:ident) => {
        let mut $v = core::mem::MaybeUninit::new($v);
        // SAFETY: We just initialized the value.
        let mut $v = unsafe { $crate::inplace::InplaceOption::new_init_unchecked(&mut $v) };
    };
}

/// Constructs an uninitialized [`crate::inplace::InplaceOption`] on the stack.
#[macro_export]
macro_rules! inplace_none {
    ($v:ident) => {
        let mut $v = core::mem::MaybeUninit::uninit();
        let mut $v = $crate::inplace::InplaceOption::uninit(&mut $v);
    };
    ($v:ident : $t:ty) => {
        let mut $v = core::mem::MaybeUninit::<$t>::uninit();
        let mut $v = $crate::inplace::InplaceOption::uninit(&mut $v);
    };
}

#[cfg(test)]
mod tests {
    use alloc::boxed::Box;
    use alloc::string::String;
    use alloc::string::ToString;
    use alloc::sync::Arc;
    use test_with_tracing::test;

    #[test]
    fn test_take_retains_backing_bytes() {
        let mut storage = core::mem::MaybeUninit::new(37u64);
        let mut owner = unsafe { super::InplaceOption::new_init_unchecked(&mut storage) };
        assert_eq!(owner.take(), Some(37));
        assert_eq!(owner.take(), None);
        drop(owner);
        // The Copy value was initialized above; taking relinquishes logical
        // ownership without physically clearing or overwriting its bytes.
        assert_eq!(unsafe { storage.assume_init() }, 37);
    }

    #[test]
    fn test_take_transfers_non_copy_ownership() {
        let value = Arc::new(());
        let weak = Arc::downgrade(&value);
        inplace_some!(value);
        let taken = value.take().unwrap();
        assert!(value.take().is_none());
        drop(value);
        assert_eq!(weak.strong_count(), 1);
        drop(taken);
        assert!(weak.upgrade().is_none());
    }

    #[test]
    fn test_inplace_some() {
        let v = "test".to_string();
        inplace_some!(v);
        assert_eq!(&v.take().unwrap(), "test");
    }

    #[test]
    fn test_inplace_none() {
        inplace_none!(v: String);
        v.set("test".to_string());
        assert_eq!(&v.take().unwrap(), "test");
    }

    #[test]
    fn test_inplace() {
        let v = Some("test".to_string());
        inplace!(v);
        assert_eq!(&v.take().unwrap(), "test");
    }

    #[test]
    fn test_inplace_replace() {
        let v = "old".to_string();
        inplace_some!(v);
        v.set("new".to_string());
        assert_eq!(&v.take().unwrap(), "new");
    }

    #[test]
    fn test_updates() {
        let v = Arc::new(Box::new(1234));
        inplace_some!(v);
        v.update_arc(|v| {
            v.update_box(|v| {
                v.set(5678);
            });
        });
        assert_eq!(**v.take().unwrap(), 5678);
    }
}
