// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

verus! {

impl<'a, T> InplaceOption<'a, T> {
    /// The borrowed storage, including its eventual writeback.
    pub closed spec fn storage(&self) -> &'a mut MaybeUninit<T> {
        self.val
    }

    /// Relates the public ownership observations without exposing the fields.
    pub proof fn storage_observation(&self)
        ensures
            self.contents() == self.storage().mem_contents(),
            self.storage_valid() == (self.initialized() ==> self.contents().is_init()),
    {
    }

    pub(crate) fn finish_cleared(self)
        requires !self.initialized(),
        ensures final(self.storage()).mem_contents() == self.contents(),
    {
        native_noop_drop(self);
    }

    /// The initialization flag, separate from ownership of the storage contents.
    pub closed spec fn initialized(&self) -> bool {
        self.init
    }

    /// Logical storage ownership; this does not describe physically cleared bytes.
    pub closed spec fn contents(&self) -> vstd::raw_ptr::MemContents<T> {
        self.val.mem_contents()
    }

    /// An explicit lifecycle premise, not an invariant established for every instance.
    pub closed spec fn storage_valid(&self) -> bool {
        self.init ==> self.val.mem_contents().is_init()
    }
}

fn check_native_take<T>(
    storage: &mut MaybeUninit<T>,
) -> (result: (Option<T>, InplaceOption<'_, T>))
    requires old(storage).mem_contents().is_init(),
    ensures
        result.0 == Some(old(storage).mem_contents().value()),
        !result.1.initialized(),
        result.1.storage_valid(),
{
    let mut owner = unsafe { InplaceOption::new_init_unchecked(storage) };
    let value = owner.take();
    let second = owner.take();
    assert(second is None);
    (value, owner)
}

fn check_as_mut_writeback<T>(
    storage: &mut MaybeUninit<T>,
    replacement: T,
) -> (owner: InplaceOption<'_, T>)
    requires old(storage).mem_contents().is_init(),
    ensures
        owner.initialized(),
        owner.contents() == vstd::raw_ptr::MemContents::Init(replacement),
        owner.storage_valid(),
{
    let mut owner = unsafe { InplaceOption::new_init_unchecked(storage) };
    match owner.as_mut() {
        Some(value) => { *value = replacement; }
        None => { assert(false); }
    }
    owner
}

fn check_as_mut_uninitialized<T>(
    storage: &mut MaybeUninit<T>,
) -> (owner: InplaceOption<'_, T>)
    ensures
        !owner.initialized(),
        owner.contents() == old(storage).mem_contents(),
        owner.storage_valid(),
{
    let mut owner = InplaceOption::uninit(storage);
    let value = owner.as_mut();
    assert(value is None);
    owner
}

fn check_forget_initialized<T>(
    storage: &mut MaybeUninit<T>,
) -> (owner: InplaceOption<'_, T>)
    requires old(storage).mem_contents().is_init(),
    ensures
        !owner.initialized(),
        owner.contents() == old(storage).mem_contents(),
        owner.storage_valid(),
{
    let mut owner = unsafe { InplaceOption::new_init_unchecked(storage) };
    let initialized = owner.forget();
    assert(initialized);
    owner
}

fn check_forget_uninitialized<T>(
    storage: &mut MaybeUninit<T>,
) -> (owner: InplaceOption<'_, T>)
    ensures
        !owner.initialized(),
        owner.contents() == old(storage).mem_contents(),
        owner.storage_valid(),
{
    let mut owner = InplaceOption::uninit(storage);
    let initialized = owner.forget();
    assert(!initialized);
    owner
}

fn check_set_writeback(
    storage: &mut MaybeUninit<u64>,
    initialized: bool,
) -> (owner: InplaceOption<'_, u64>)
    requires initialized ==> old(storage).mem_contents().is_init(),
    ensures
        owner.initialized(),
        owner.storage_valid(),
        owner.contents() == vstd::raw_ptr::MemContents::Init(11),
        final(storage).mem_contents() == final(owner.storage()).mem_contents(),
{
    let mut owner = if initialized {
        unsafe { InplaceOption::new_init_unchecked(storage) }
    } else {
        InplaceOption::uninit(storage)
    };
    let tracked copy = native_copy::<u64>();
    let value = owner._VERUS_VERIFIED_set(0, Tracked(copy));
    assert(*value == 0);
    *value = 11;
    owner
}

} // verus!
