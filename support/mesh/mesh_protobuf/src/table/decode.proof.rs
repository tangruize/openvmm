// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

use crate::encoding::{FromNumber, VarintField};
use crate::NoResources;

type Result<T, E = Error> = core::result::Result<T, E>;
#[cfg(not(verus_read_negative_resource))]
type ReadResources = NoResources;
#[cfg(verus_read_negative_resource)]
type ReadResources = ();
#[cfg(not(verus_table_negative_resource))]
type SavedReadResources = NoResources;
#[cfg(verus_table_negative_resource)]
type SavedReadResources = ();

specialize_varint_read_field!(read_u64_field_body, u64, NoResources);
specialize_read_field_dyn!(read_u64_field_dyn_body, u64, NoResources, VarintField);
specialize_saved_read_fields!(pub saved_read_fields, NoResources);
specialize_saved_read_fields_by_ptr!(saved_read_fields_by_ptr, NoResources);
specialize_saved_read_fields_inline!(saved_read_fields_inline, NoResources);
specialize_saved_read_fields_inner!(saved_read_fields_inner, NoResources);

verus! {

/// Concrete-resource table proof retaining the caller's exclusive field loan.
pub fn read_saved_time_struct(
    numbers: &[u32], decoders: &[ErasedDecoderEntry], offsets: &[usize],
    base: *mut u8, initialized: &mut bool, reader: MessageReader<'_, '_, NoResources>,
    Tracked(field): Tracked<&mut NativeMutLoan<'_, u64>>,
) -> (result: Result<(), Error>)
    requires
        saved_time_table(numbers@, decoders@, offsets@, reader.reader_bytes()),
        old(field).ptr() as *mut u8 == base,
        *old(initialized) ==> old(field).storage().mem_contents().is_init(),
    ensures
        result is Ok, *final(initialized),
        final(field).ptr() == old(field).ptr(),
        final(field).storage().mem_contents() == saved_time_result(
            reader.reader_bytes(), old(field).storage().mem_contents(), *old(initialized)),
        final(final(field).storage()).mem_contents() == final(old(field).storage()).mem_contents(),
{
    unsafe {
        native_specialized_call_with(
            read_fields(numbers, decoders, offsets, base, initialized, reader),
            _VERUS_VERIFIED_saved_read_fields, (Tracked(field),))
    }
}

proof fn finish_read_flag_loan(tracked loan: NativeInitLoan<'_, bool>)
    ensures *final(loan.storage()) == *loan.storage(),
{
    finish_init_loan(loan);
}

/// Dispatch the actual entry with the caller's field storage and flag.
pub fn read_u64_erased_field(
    decoder: &ErasedDecoderEntry,
    pointer: *mut u8,
    initialized: &mut bool,
    reader: FieldReader<'_, '_, NoResources>,
    Tracked(field): Tracked<&mut NativeMutLoan<'_, u64>>,
) -> (result: Result<(), Error>)
    requires
        is_u64_read_entry(*decoder),
        old(field).ptr() as *mut u8 == pointer,
        *old(initialized) ==> old(field).storage().mem_contents().is_init(),
        reader.varint_payload() is Some,
    ensures
        result is Ok,
        *final(initialized),
        final(field).ptr() == old(field).ptr(),
        final(field).storage().mem_contents()
            == vstd::raw_ptr::MemContents::Init(reader.varint_payload()->Some_0),
        final(final(field).storage()).mem_contents()
            == final(old(field).storage()).mem_contents(),
{
    let ghost value = reader.varint_payload()->Some_0;
    #[cfg(verus_read_negative_pointer)]
    let pointer = core::ptr::null_mut();
    native_dynamic_dispatch(
        unsafe { decoder.read_field(pointer, initialized, reader) },
        const { <VarintField as FieldDecode<'static, u64, NoResources>>::ENTRY.erase() },
        const { unsafe {
            &*const {
                if cfg!(verus_read_negative_entry) {
                    <VarintField as FieldDecode<'static, u32, NoResources>>::ENTRY.erase()
                } else {
                    <VarintField as FieldDecode<'static, u64, NoResources>>::ENTRY.erase()
                }
            }
                .0.cast::<StaticDecoderVtable<'static, ReadResources>>()
        }.read_fn },
        const { read_field_dyn::<u64, ReadResources, VarintField>
            as unsafe fn(*mut u8, *mut bool, FieldReader<'static, '_, ReadResources>) -> Result<(), Error> },
        _VERUS_VERIFIED_read_u64_field_dyn_body,
        Tracked(&mut *field),
        Ghost(|contents: vstd::raw_ptr::MemContents<u64>, flag: bool, result: Result<(), Error>|
            contents == vstd::raw_ptr::MemContents::Init(value) && flag && result.is_ok()),
        finish_read_flag_loan,
    )
}

pub proof fn u64_default_entry_value(entry: ErasedDecoderEntry)
    ensures is_u64_default_entry(entry) == native_const_matches(entry, const {
        <crate::encoding::VarintField as FieldDecode<'static, u64, ()>>::ENTRY.erase()
    }),
{
}

/// Enter the production slice/pointer wrappers with a projection of the
/// caller's whole storage. The inline reader creates its own field flags.
pub fn read_empty_struct<R>(
    numbers: &[u32],
    decoders: &[ErasedDecoderEntry],
    offsets: &[usize],
    base: *mut u8,
    struct_initialized: &mut bool,
    reader: MessageReader<'_, '_, R>,
    Tracked(field_loan): Tracked<&mut NativeMutLoan<'_, u64>>,
) -> (result: Result<(), Error>)
    requires
        numbers@.len() == 1,
        decoders@.len() == 1,
        offsets@.len() == 1,
        is_u64_default_entry(decoders[0]),
        offsets[0] == 0,
        reader.reader_bytes().len() == 0,
        old(field_loan).ptr() as *mut u8 == base,
        *old(struct_initialized) ==> old(field_loan).storage().mem_contents().is_init(),
    ensures
        result is Ok,
        *final(struct_initialized),
        final(field_loan).ptr() == old(field_loan).ptr(),
        final(field_loan).storage().mem_contents() == if *old(struct_initialized) {
            old(field_loan).storage().mem_contents()
        } else { vstd::raw_ptr::MemContents::Init(0) },
        final(final(field_loan).storage()).mem_contents()
            == final(old(field_loan).storage()).mem_contents(),
{
    unsafe {
        _VERUS_VERIFIED_read_fields(
            numbers, decoders, offsets, base, struct_initialized, reader, Tracked(field_loan))
    }
}

#[cfg(verus_reader_negative_metadata_pointer)]
fn negative_reader_metadata_pointer<R>(
    numbers: &[u32], decoders: &[ErasedDecoderEntry], offsets: &[usize],
    unrelated: *const ErasedDecoderEntry,
    storage: &mut MaybeUninit<u64>, reader: MessageReader<'_, '_, R>,
)
    requires
        numbers@.len() == 1, decoders@.len() == 1, offsets@.len() == 1,
        is_u64_default_entry(decoders[0]), offsets[0] == 0,
        reader.reader_bytes().len() == 0,
{
    let (numbers_ptr, Tracked(numbers_loan)) = native_slice_loan(numbers.as_ptr(), numbers);
    let (_, Tracked(decoders_loan)) = native_slice_loan(decoders.as_ptr(), decoders);
    let (offsets_ptr, Tracked(offsets_loan)) = native_slice_loan(offsets.as_ptr(), offsets);
    let (base, Tracked(mut field)) = native_mut_loan(storage.as_mut_ptr(), storage);
    let mut initialized = false;
    unsafe {
        _VERUS_VERIFIED_read_fields_by_ptr(1, numbers_ptr, unrelated, offsets_ptr,
            base.cast::<u8>(), &mut initialized, reader,
            Tracked(&numbers_loan), Tracked(&decoders_loan), Tracked(&offsets_loan),
            Tracked(&mut field));
    }
}

#[cfg(verus_reader_negative_metadata_extent)]
fn negative_reader_metadata_extent<R>(
    numbers: &[u32], decoders: &[ErasedDecoderEntry], offsets: &[usize],
    storage: &mut MaybeUninit<u64>, reader: MessageReader<'_, '_, R>,
)
    requires
        numbers@.len() == 1, decoders@.len() == 2, offsets@.len() == 1,
        is_u64_default_entry(decoders[0]), offsets[0] == 0,
        reader.reader_bytes().len() == 0,
{
    let (numbers_ptr, Tracked(numbers_loan)) = native_slice_loan(numbers.as_ptr(), numbers);
    let (decoders_ptr, Tracked(decoders_loan)) = native_slice_loan(decoders.as_ptr(), decoders);
    let (offsets_ptr, Tracked(offsets_loan)) = native_slice_loan(offsets.as_ptr(), offsets);
    let (base, Tracked(mut field)) = native_mut_loan(storage.as_mut_ptr(), storage);
    let mut initialized = false;
    unsafe {
        _VERUS_VERIFIED_read_fields_by_ptr(1, numbers_ptr, decoders_ptr, offsets_ptr,
            base.cast::<u8>(), &mut initialized, reader,
            Tracked(&numbers_loan), Tracked(&decoders_loan), Tracked(&offsets_loan),
            Tracked(&mut field));
    }
}

/// The caller supplies the field loan and the real table's slices. Whole-object
/// decomposition and the outer reader composition are not established here.
pub fn read_empty_field<R>(
    numbers: &[u32],
    decoders: &[ErasedDecoderEntry],
    offsets: &[usize],
    storage: &mut MaybeUninit<u64>,
    field_init: &mut [bool],
    reader: MessageReader<'_, '_, R>,
) -> (result: Result<(), Error>)
    requires
        numbers@.len() == 1,
        decoders@.len() >= 1,
        offsets@.len() >= 1,
        is_u64_default_entry(decoders[0]),
        offsets[0] == 0,
        old(field_init)@.len() == 1,
        old(field_init)[0] ==> old(storage).mem_contents().is_init(),
        reader.reader_bytes().len() == 0,
    ensures
        result is Ok,
        final(storage).mem_contents() == if old(field_init)[0] {
            old(storage).mem_contents()
        } else { vstd::raw_ptr::MemContents::Init(0) },
        final(field_init)@ == seq![true],
{
    let (pointer, Tracked(mut loan)) = native_mut_loan(storage.as_mut_ptr(), storage);
    let result = unsafe {
        _VERUS_VERIFIED_read_fields_inner(numbers, decoders, offsets,
            pointer.cast::<u8>(), field_init, reader, Tracked(&mut loan))
    };
    proof { finish_inplace_loan(loan); }
    result
}

#[cfg(verus_reader_negative_field_loan)]
fn negative_reader_field_loan<R>(
    numbers: &[u32], decoders: &[ErasedDecoderEntry], offsets: &[usize],
    storage: &mut MaybeUninit<u64>, flags: &mut [bool], base: *mut u8,
    reader: MessageReader<'_, '_, R>,
)
    requires
        numbers@.len() == 1, decoders@.len() >= 1, offsets@.len() >= 1,
        is_u64_default_entry(decoders[0]), offsets[0] == 0,
        old(storage).mem_contents().is_init(), old(flags)@ == seq![true],
        reader.reader_bytes().len() == 0,
{
    let (_, Tracked(mut loan)) = native_mut_loan(storage.as_mut_ptr(), storage);
    unsafe {
        _VERUS_VERIFIED_read_fields_inner(numbers, decoders, offsets, base,
            flags, reader, Tracked(&mut loan));
    }
}

#[cfg(verus_reader_negative_wrong_decoder)]
fn negative_reader_wrong_decoder<R>(
    numbers: &[u32], decoders: &[ErasedDecoderEntry], offsets: &[usize],
    storage: &mut MaybeUninit<u64>, flags: &mut [bool],
    reader: MessageReader<'_, '_, R>,
)
    requires
        numbers@.len() == 1, decoders@.len() >= 1, offsets@.len() >= 1,
        !is_u64_default_entry(decoders[0]), offsets[0] == 0,
        old(storage).mem_contents().is_init(), old(flags)@ == seq![false],
        reader.reader_bytes().len() == 0,
{
    read_empty_field(numbers, decoders, offsets, storage, flags, reader);
}

#[cfg(verus_reader_negative_uninit)]
fn negative_reader_uninitialized_true_flag<R>(
    numbers: &[u32], decoders: &[ErasedDecoderEntry], offsets: &[usize],
    flags: &mut [bool], reader: MessageReader<'_, '_, R>,
)
    requires
        numbers@.len() == 1, decoders@.len() >= 1, offsets@.len() >= 1,
        is_u64_default_entry(decoders[0]), offsets[0] == 0,
        old(flags)@ == seq![true], reader.reader_bytes().len() == 0,
{
    let mut storage = MaybeUninit::<u64>::uninit();
    read_empty_field(numbers, decoders, offsets, &mut storage, flags, reader);
}

/// The observable storage, initialized flag, and callback result.
pub type InplacePost<T, R> = spec_fn(vstd::raw_ptr::MemContents<T>, bool, R) -> bool;

// This is a caller obligation for each real decoder, not a trait-wide axiom.
// The trait documents initialized success; an error need not roll back.
#[verifier::prophetic]
closed spec fn typed_default<'a, T, R, E: FieldDecode<'a, T, R>>(
    before: vstd::raw_ptr::MemContents<T>,
    initialized: bool,
    post: InplacePost<T, Result<(), Error>>,
) -> bool {
    (forall|item: &mut InplaceOption<'_, T>|
        item.storage_valid() && item.contents() == before && item.initialized() == initialized
        ==> #[trigger] call_requires(E::default_field, (item,)))
    && (forall|item: &mut InplaceOption<'_, T>, r: Result<(), Error>|
        #[trigger] call_ensures(E::default_field, (item,), r)
            && item.storage_valid() && item.contents() == before && item.initialized() == initialized
        ==> final(item).storage_valid()
            && (r is Ok ==> final(item).initialized())
            && final(item.storage()).mem_contents()
                == final(final(item).storage()).mem_contents()
            && post(final(item).contents(), final(item).initialized(), r))
}

/// Resolve the exclusive reborrow back into its original storage.
pub proof fn finish_inplace_loan<T>(tracked loan: NativeMutLoan<'_, T>)
    ensures
        final(loan.storage()).mem_contents() == loan.storage().mem_contents(),
{
}

proof fn finish_init_loan<T>(tracked loan: NativeInitLoan<'_, T>)
    ensures *final(loan.storage()) == *loan.storage(),
{
}

fn initialized_flag_loan(
    flag: &mut MaybeUninit<bool>,
) -> (result: (*mut bool, Tracked<NativeInitLoan<'_, bool>>))
    requires old(flag).mem_contents().is_init(),
    ensures
        result.0 == result.1@.ptr(),
        *result.1@.storage() == old(flag).mem_contents().value(),
        final(flag).mem_contents() == vstd::raw_ptr::MemContents::Init(*final(result.1@.storage())),
{
    let value = unsafe { flag.assume_init_mut() };
    native_init_loan(core::ptr::from_mut(value), value)
}

fn check_run_inplace_write<T>(
    storage: &mut MaybeUninit<T>,
    replacement: T,
    initially_initialized: bool,
    returned_initialized: bool,
    fail: bool,
) -> (owner: InplaceOption<'_, T>)
    requires initially_initialized ==> old(storage).mem_contents().is_init(),
    ensures
        owner.initialized() == returned_initialized,
        owner.contents() == vstd::raw_ptr::MemContents::Init(replacement),
        owner.storage_valid(),
{
    let mut owner = if initially_initialized {
        unsafe { InplaceOption::new_init_unchecked(storage) }
    } else {
        InplaceOption::uninit(storage)
    };
    let callback = move |base: *mut u8, flag: &mut bool,
        permission: Tracked<&mut NativeMutLoan<'_, T>>| -> (r: Result<(), ()>)
        requires permission@.ptr() as *mut u8 == base,
        ensures
            final(permission@).ptr() == permission@.ptr(),
            final(final(permission@).storage()).mem_contents()
                == final(permission@.storage()).mem_contents(),
            final(permission@).storage().mem_contents()
                == vstd::raw_ptr::MemContents::Init(replacement),
            *final(flag) == returned_initialized,
            r == if fail { Err(()) } else { Ok(()) },
    {
        let tracked loan = permission.get();
        unsafe {
            native_loan_write::<T>(base.cast::<T>().write(replacement), Tracked(loan));
        }
        *flag = returned_initialized;
        if fail { Err(()) } else { Ok(()) }
    };
    let r = unsafe {
        _VERUS_VERIFIED_run_inplace(&mut owner, callback, Ghost(
            |contents: vstd::raw_ptr::MemContents<T>, initialized: bool, r: Result<(), ()>|
                contents == vstd::raw_ptr::MemContents::Init(replacement)
                && initialized == returned_initialized
                && r == if fail { Err(()) } else { Ok(()) }
        ))
    };
    assert(r == if fail { Err(()) } else { Ok(()) });
    owner
}

fn check_initialized_error_retains_new_value() {
    let mut storage = MaybeUninit::new(7u64);
    let mut owner = check_run_inplace_write(&mut storage, 9, true, true, true);
    let value = owner.take();
    assert(value == Some(9));
}

fn check_uninitialized_success() {
    let mut storage = MaybeUninit::<u64>::uninit();
    let mut owner = check_run_inplace_write(&mut storage, 9, false, true, false);
    let value = owner.take();
    assert(value == Some(9));
}

fn check_false_flag_retains_contents() {
    let mut storage = MaybeUninit::<u64>::uninit();
    let owner = check_run_inplace_write(&mut storage, 9, false, false, true);
    assert(!owner.initialized());
    assert(owner.contents() == vstd::raw_ptr::MemContents::Init(9));
}

#[cfg(verus_inplace_negative_stale)]
fn negative_stale() {
    let mut storage = MaybeUninit::new(7u64);
    let owner = check_run_inplace_write(&mut storage, 9, true, true, true);
    assert(owner.contents() == vstd::raw_ptr::MemContents::Init(7));
}

#[cfg(verus_inplace_negative_slot)]
fn negative_slot() {
    let mut left = MaybeUninit::<u64>::uninit();
    let mut right = MaybeUninit::<u64>::uninit();
    let mut left = InplaceOption::uninit(&mut left);
    let mut right = InplaceOption::uninit(&mut right);
    let (ptr, Tracked(left_loan)) = left._VERUS_VERIFIED_as_mut_ptr();
    let (_, Tracked(mut right_loan)) = right._VERUS_VERIFIED_as_mut_ptr();
    unsafe {
        native_loan_write::<u64>(ptr.write(9), Tracked(&mut right_loan));
    }
    proof {
        finish_inplace_loan(left_loan);
        finish_inplace_loan(right_loan);
    }
}

#[cfg(verus_inplace_negative_duplicate)]
fn negative_duplicate() {
    let mut storage = MaybeUninit::<u64>::uninit();
    let mut owner = InplaceOption::uninit(&mut storage);
    let (_, Tracked(loan)) = owner._VERUS_VERIFIED_as_mut_ptr();
    let tracked duplicate = loan;
    proof {
        finish_inplace_loan(loan);
        finish_inplace_loan(duplicate);
    }
}

#[cfg(verus_inplace_negative_live)]
fn negative_live() {
    let mut storage = MaybeUninit::<u64>::uninit();
    let mut owner = InplaceOption::uninit(&mut storage);
    let (_, Tracked(loan)) = owner._VERUS_VERIFIED_as_mut_ptr();
    owner.forget();
    proof { finish_inplace_loan(loan); }
}

} // verus!

verus! {

/// Checks the allocated metadata and function relocation, not just entry equality.
/// This does not discharge the runtime decode or indirect-call obligations.
pub fn check_u64_default_metadata() {
    native_assert_const_eq(
        const {
            unsafe {
                &*<crate::encoding::VarintField as FieldDecode<'static, u64, ()>>::ENTRY
                    .erase().0.cast::<StaticDecoderVtable<'static, ()>>()
            }.default_fn
        },
        const {
            default_field_dyn::<u64, (), crate::encoding::VarintField>
                as unsafe fn(*mut u8, *mut bool) -> Result<(), Error>
        },
    );
    native_assert_const_eq(
        const { align_of::<StaticDecoderVtable<'static, ()>>() & ENTRY_IS_TABLE },
        const { 0usize },
    );
    native_assert_const_eq(const { ENTRY_IS_TABLE }, const { 1usize });
}

#[cfg(verus_dispatch_negative_target)]
fn negative_default_metadata_target() {
    native_assert_const_eq(
        const {
            unsafe {
                &*<crate::encoding::VarintField as FieldDecode<'static, u64, ()>>::ENTRY
                    .erase().0.cast::<StaticDecoderVtable<'static, ()>>()
            }.default_fn
        },
        const {
            default_field_dyn::<u32, (), crate::encoding::VarintField>
                as unsafe fn(*mut u8, *mut bool) -> Result<(), Error>
        },
    );
}

#[cfg(verus_dispatch_negative_provenance)]
fn negative_default_metadata_provenance() {
    native_assert_const_eq(
        const {
            unsafe {
                &*core::ptr::without_provenance::<StaticDecoderVtable<'static, ()>>(8)
            }.default_fn
        },
        const {
            default_field_dyn::<u64, (), crate::encoding::VarintField>
                as unsafe fn(*mut u8, *mut bool) -> Result<(), Error>
        },
    );
}

#[cfg(verus_dispatch_negative_tag)]
fn negative_default_metadata_tag() {
    native_assert_const_eq(
        const {
            unsafe {
                &*<crate::encoding::VarintField as FieldDecode<'static, u64, ()>>::ENTRY
                    .erase().0.wrapping_byte_add(ENTRY_IS_TABLE)
                    .cast::<StaticDecoderVtable<'static, ()>>()
            }.default_fn
        },
        const {
            default_field_dyn::<u64, (), crate::encoding::VarintField>
                as unsafe fn(*mut u8, *mut bool) -> Result<(), Error>
        },
    );
}

proof fn u64_typed_default<'a, R>(
    before: vstd::raw_ptr::MemContents<u64>,
    initialized: bool,
)
    ensures typed_default::<'a, u64, R, crate::encoding::VarintField>(
        before, initialized, |contents, flag, result|
            contents == vstd::raw_ptr::MemContents::Init(0) && flag && result is Ok),
{
}

/// Composes the verified production default adapter with exclusively borrowed storage.
/// Metadata dispatch is a separate obligation.
pub fn default_u64_storage<R>(
    storage: &mut MaybeUninit<u64>,
    flag: &mut MaybeUninit<bool>,
) -> (result: Result<(), Error>)
    requires
        old(flag).mem_contents().is_init(),
        old(flag).mem_contents().value() ==> old(storage).mem_contents().is_init(),
    ensures
        result is Ok,
        final(storage).mem_contents() == vstd::raw_ptr::MemContents::Init(0),
        final(flag).mem_contents() == vstd::raw_ptr::MemContents::Init(true),
{
    proof { u64_typed_default::<R>(storage.mem_contents(), flag.mem_contents().value()); }
    let (pointer, Tracked(mut loan)) = native_mut_loan(storage.as_mut_ptr(), storage);
    let (init, Tracked(mut flag_loan)) = initialized_flag_loan(flag);
    let result = unsafe {
        _VERUS_VERIFIED_default_field_dyn::<u64, R, crate::encoding::VarintField>(
            pointer.cast::<u8>(), init, Tracked(&mut loan), Tracked(&mut flag_loan),
            Ghost(|contents, flag, result|
                contents == vstd::raw_ptr::MemContents::Init(0) && flag && result is Ok))
    };
    proof {
        finish_inplace_loan(loan);
        finish_init_loan(flag_loan);
    }
    result
}

/// Executes the production erased entry with exclusive storage and flag loans.
/// Native specialization checks both dispatch bodies and the constant metadata;
/// the resulting adapter call still has to establish every verified precondition.
pub fn default_u64_erased_storage(
    storage: &mut MaybeUninit<u64>,
    flag: &mut MaybeUninit<bool>,
) -> (result: Result<(), Error>)
    requires
        old(flag).mem_contents().is_init(),
        old(flag).mem_contents().value() ==> old(storage).mem_contents().is_init(),
    ensures
        result is Ok,
        final(storage).mem_contents() == vstd::raw_ptr::MemContents::Init(0),
        final(flag).mem_contents() == vstd::raw_ptr::MemContents::Init(true),
{
    proof { u64_typed_default::<()>(storage.mem_contents(), flag.mem_contents().value()); }
    let (pointer, Tracked(mut loan)) = native_mut_loan(storage.as_mut_ptr(), storage);
    let (init, Tracked(mut flag_loan)) = initialized_flag_loan(flag);
    #[cfg(verus_erased_negative_flag)]
    let init = pointer.cast::<bool>();
    #[cfg(verus_erased_negative_live)]
    { *storage = MaybeUninit::new(73); }
    #[cfg(verus_erased_negative_duplicate)]
    { proof { finish_inplace_loan(loan); } }
    let result = native_const_dispatch(
        unsafe {
            const {
                if cfg!(verus_erased_negative_entry) {
                    <crate::encoding::VarintField as FieldDecode<'static, u32, ()>>::ENTRY.erase()
                } else {
                    <crate::encoding::VarintField as FieldDecode<'static, u64, ()>>::ENTRY.erase()
                }
            }
                .default_field(pointer.cast::<u8>(), &mut *init)
        },
        const {
            unsafe {
                &*const { <crate::encoding::VarintField as FieldDecode<'static, u64, ()>>::ENTRY.erase() }
                    .0.cast::<StaticDecoderVtable<'static, ()>>()
            }.default_fn
        },
        const {
            default_field_dyn::<u64, (), crate::encoding::VarintField>
                as unsafe fn(*mut u8, *mut bool) -> Result<(), Error>
        },
        _VERUS_VERIFIED_default_field_dyn::<u64, (), crate::encoding::VarintField>,
        Tracked(&mut loan), Tracked(&mut flag_loan),
        Ghost(|contents: vstd::raw_ptr::MemContents<u64>, flag: bool, result: Result<(), Error>|
            contents == vstd::raw_ptr::MemContents::Init(0) && flag && result is Ok),
    );
    proof {
        finish_inplace_loan(loan);
        finish_init_loan(flag_loan);
    }
    #[cfg(verus_erased_negative_stale)]
    assert(storage.mem_contents() == vstd::raw_ptr::MemContents::Init(73));
    result
}

#[cfg(verus_erased_negative_type)]
fn negative_erased_wrong_type() {
    let mut storage = MaybeUninit::<u32>::uninit();
    let mut flag = MaybeUninit::new(false);
    let (pointer, Tracked(mut loan)) = native_mut_loan(storage.as_mut_ptr(), &mut storage);
    let (init, Tracked(mut flag_loan)) = initialized_flag_loan(&mut flag);
    let result = native_const_dispatch(
        unsafe {
            const { <crate::encoding::VarintField as FieldDecode<'static, u64, ()>>::ENTRY.erase() }
                .default_field(pointer.cast::<u8>(), &mut *init)
        },
        const {
            unsafe {
                &*const { <crate::encoding::VarintField as FieldDecode<'static, u64, ()>>::ENTRY.erase() }
                    .0.cast::<StaticDecoderVtable<'static, ()>>()
            }.default_fn
        },
        const {
            default_field_dyn::<u64, (), crate::encoding::VarintField>
                as unsafe fn(*mut u8, *mut bool) -> Result<(), Error>
        },
        _VERUS_VERIFIED_default_field_dyn::<u64, (), crate::encoding::VarintField>,
        Tracked(&mut loan), Tracked(&mut flag_loan),
        Ghost(|contents: vstd::raw_ptr::MemContents<u64>, flag: bool, result: Result<(), Error>|
            contents == vstd::raw_ptr::MemContents::Init(0) && flag && result is Ok),
    );
}

fn check_varint_uninitialized<R>() {
    let mut storage = MaybeUninit::<u64>::uninit();
    let mut flag = MaybeUninit::new(false);
    let result = default_u64_storage::<R>(&mut storage, &mut flag);
    assert(result is Ok);
    assert(storage.mem_contents() == vstd::raw_ptr::MemContents::Init(0));
    assert(flag.mem_contents() == vstd::raw_ptr::MemContents::Init(true));
}

fn check_varint_initialized_nonzero<R>() {
    let mut storage = MaybeUninit::new(73u64);
    let mut flag = MaybeUninit::new(true);
    let result = default_u64_storage::<R>(&mut storage, &mut flag);
    assert(result is Ok);
    assert(storage.mem_contents() == vstd::raw_ptr::MemContents::Init(0));
    assert(flag.mem_contents() == vstd::raw_ptr::MemContents::Init(true));
    #[cfg(verus_varint_negative_stale)]
    assert(storage.mem_contents() == vstd::raw_ptr::MemContents::Init(73));
}

fn check_varint_false_flag_retained<R>() {
    let mut storage = MaybeUninit::new(73u64);
    let mut flag = MaybeUninit::new(false);
    let result = default_u64_storage::<R>(&mut storage, &mut flag);
    assert(result is Ok);
    assert(storage.mem_contents() == vstd::raw_ptr::MemContents::Init(0));
    assert(flag.mem_contents() == vstd::raw_ptr::MemContents::Init(true));
}

#[cfg(verus_default_negative_conversion)]
fn negative_generic_conversion<T: crate::encoding::FromNumber>() {
    let result = T::from_u64(0);
    assert(result is Ok);
}

} // verus!

// These are concrete witnesses for the conditional adapter, not replacements
// for any production decoder or for the adapter body.
#[verus_verify]
struct DefaultWitness;

impl<'a> FieldDecode<'a, (u64, Option<Error>, bool), ()> for DefaultWitness {
    fn read_field(
        _item: &mut InplaceOption<'_, (u64, Option<Error>, bool)>,
        _reader: FieldReader<'a, '_, ()>,
    ) -> Result<(), Error> {
        unreachable!()
    }

    vstd::prelude::verus_trait_impl! {
    fn default_field(item: &mut InplaceOption<'_, (u64, Option<Error>, bool)>) -> (result: Result<(), Error>)
        ensures
            final(item).storage_valid(),
            final(item).contents() == vstd::raw_ptr::MemContents::Init((9, None, true)),
            final(item).initialized() == witness_flag(old(item).contents(), old(item).initialized()),
            result == witness_result(old(item).contents(), old(item).initialized()),
            final(old(item).storage()).mem_contents()
                == final(final(item).storage()).mem_contents(),
    {
        let (result, initialize) = match item.take() {
            Some((_, Some(error), initialize)) => (Err(error), initialize),
            _ => (Ok(()), true),
        };
        let (pointer, Tracked(mut loan)) = item._VERUS_VERIFIED_as_mut_ptr();
        unsafe {
            native_loan_write::<(u64, Option<Error>, bool)>(
                pointer.write((9, None, true)), Tracked(&mut loan));
        }
        proof { finish_inplace_loan(loan); }
        if initialize {
            unsafe { item.set_init_unchecked(); }
        }
        result
    }
    }
}

verus! {

/// The exact result returned by the concrete defaulting witness.
pub closed spec fn witness_result(
    before: vstd::raw_ptr::MemContents<(u64, Option<Error>, bool)>,
    initialized: bool,
) -> Result<(), Error> {
    if initialized && before.value().1 is Some {
        Err(before.value().1.unwrap())
    } else {
        Ok(())
    }
}

/// The flag returned by the concrete defaulting witness.
pub closed spec fn witness_flag(
    before: vstd::raw_ptr::MemContents<(u64, Option<Error>, bool)>,
    initialized: bool,
) -> bool {
    if initialized && before.value().1 is Some { before.value().2 } else { true }
}

fn check_default_adapter(
    storage: &mut MaybeUninit<(u64, Option<Error>, bool)>,
    flag: &mut MaybeUninit<bool>,
) -> (r: Result<(), Error>)
    requires
        old(flag).mem_contents().is_init(),
        old(flag).mem_contents().value() ==> old(storage).mem_contents().is_init(),
    ensures
        final(storage).mem_contents() == vstd::raw_ptr::MemContents::Init((9, None, true)),
        final(flag).mem_contents() == vstd::raw_ptr::MemContents::Init(
            witness_flag(old(storage).mem_contents(), old(flag).mem_contents().value())),
        r == witness_result(old(storage).mem_contents(), old(flag).mem_contents().value()),
{
    let ghost before = storage.mem_contents();
    let ghost initialized = flag.mem_contents().value();
    let (ptr, Tracked(mut loan)) = native_mut_loan(storage.as_mut_ptr(), storage);
    let (init, Tracked(mut init_loan)) = initialized_flag_loan(flag);
    #[cfg(verus_default_negative_storage)]
    { *storage = MaybeUninit::uninit(); }
    #[cfg(verus_default_negative_flag)]
    { *flag = MaybeUninit::new(false); }
    let result = unsafe { _VERUS_VERIFIED_default_field_dyn::<_, (), DefaultWitness>(
        ptr.cast::<u8>(), init, Tracked(&mut loan), Tracked(&mut init_loan),
        Ghost(|contents, flag, result|
            contents == vstd::raw_ptr::MemContents::Init((9, None, true))
                && flag == witness_flag(before, initialized)
                && result == witness_result(before, initialized)),
    ) };
    proof {
        finish_inplace_loan(loan);
        finish_init_loan(init_loan);
    }
    result
}

fn check_default_uninitialized_success() {
    let mut storage = MaybeUninit::uninit();
    let mut flag = MaybeUninit::new(false);
    let result = check_default_adapter(&mut storage, &mut flag);
    assert(result is Ok);
    assert(flag.mem_contents() == vstd::raw_ptr::MemContents::Init(true));
}

fn check_default_initialized_success() {
    let mut storage = MaybeUninit::new((7, None, false));
    let mut flag = MaybeUninit::new(true);
    let result = check_default_adapter(&mut storage, &mut flag);
    assert(result is Ok);
    assert(storage.mem_contents() == vstd::raw_ptr::MemContents::Init((9, None, true)));
    #[cfg(verus_default_negative_stale)]
    assert(storage.mem_contents() == vstd::raw_ptr::MemContents::Init((7, None, false)));
}

fn check_default_error(error: Error, keep_initialized: bool) {
    let ghost original_error = error;
    let mut storage = MaybeUninit::new((7, Some(error), keep_initialized));
    let mut flag = MaybeUninit::new(true);
    let result = check_default_adapter(&mut storage, &mut flag);
    assert(result == Err(original_error));
    assert(flag.mem_contents() == vstd::raw_ptr::MemContents::Init(keep_initialized));
    assert(storage.mem_contents() == vstd::raw_ptr::MemContents::Init((9, None, true)));
}

#[cfg(verus_default_negative_slot)]
fn negative_default_slot() {
    let mut left = MaybeUninit::<(u64, Option<Error>, bool)>::uninit();
    let mut right = MaybeUninit::<(u64, Option<Error>, bool)>::uninit();
    let mut flag = MaybeUninit::new(false);
    let (ptr, Tracked(left_loan)) = native_mut_loan(left.as_mut_ptr(), &mut left);
    let (_, Tracked(mut right_loan)) = native_mut_loan(right.as_mut_ptr(), &mut right);
    let (init, Tracked(mut flag_loan)) = initialized_flag_loan(&mut flag);
    unsafe {
        _VERUS_VERIFIED_default_field_dyn::<_, (), DefaultWitness>(
            ptr.cast::<u8>(), init, Tracked(&mut right_loan), Tracked(&mut flag_loan),
            Ghost(|_contents, _flag, _result| true));
    }
}

#[cfg(verus_default_negative_duplicate)]
fn negative_default_duplicate() {
    let mut storage = MaybeUninit::<(u64, Option<Error>, bool)>::uninit();
    let mut flag = MaybeUninit::new(false);
    let (ptr, Tracked(mut loan)) = native_mut_loan(storage.as_mut_ptr(), &mut storage);
    let (init, Tracked(mut flag_loan)) = initialized_flag_loan(&mut flag);
    let tracked duplicate = loan;
    unsafe {
        _VERUS_VERIFIED_default_field_dyn::<_, (), DefaultWitness>(
            ptr.cast::<u8>(), init, Tracked(&mut loan), Tracked(&mut flag_loan),
            Ghost(|_contents, _flag, _result| true));
    }
    proof { finish_inplace_loan(duplicate); }
}

#[cfg(verus_default_negative_unproved)]
fn negative_default_unproved<'a, E: FieldDecode<'a, u64, ()>>() {
    let mut storage = MaybeUninit::<u64>::uninit();
    let mut flag = MaybeUninit::new(false);
    let (ptr, Tracked(mut loan)) = native_mut_loan(storage.as_mut_ptr(), &mut storage);
    let (init, Tracked(mut flag_loan)) = initialized_flag_loan(&mut flag);
    unsafe {
        _VERUS_VERIFIED_default_field_dyn::<_, (), E>(
            ptr.cast::<u8>(), init, Tracked(&mut loan), Tracked(&mut flag_loan),
            Ghost(|_contents, _flag, _result| true));
    }
}

} // verus!
