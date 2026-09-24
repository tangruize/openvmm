// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

//! Table-based decoding.

#![expect(clippy::missing_safety_doc)]

use super::StructMetadata;
use super::TableEncoder;
use crate::Error;
use crate::FieldDecode;
use crate::MessageDecode;
use crate::inplace::InplaceOption;
use crate::protobuf::FieldReader;
use crate::protobuf::MessageReader;
use alloc::slice;
use alloc::vec;
use core::marker::PhantomData;
use core::mem::MaybeUninit;
use vstd::prelude::*;

#[cfg(verus_keep_ghost)]
include!("decode.spec.rs");

/// Calls `f` on `item`, splitting the pointer and initialized flag out.
///
/// # Safety
///
/// The caller must ensure that on the return, the bool specifies the
/// initialized state of the item.
///
/// The conditional proof additionally requires an exclusive loan of this
/// storage through the callback. The table callbacks below still owe that
/// ownership/initialization proof outside the checked SavedState wire domain,
/// including their partial-error paths.
#[verus_verify(publish_verified)]
#[verus_spec(result =>
    with Ghost(post): Ghost<InplacePost<T, R>>,
    requires
        old(item).storage_valid(),
        inplace_callback(f, old(item).contents(), old(item).initialized(), post),
    ensures
        final(item).storage_valid(),
        post(final(item).contents(), final(item).initialized(), result),
        final(final(item).storage()).mem_contents()
            == final(old(item).storage()).mem_contents(),
)]
unsafe fn run_inplace<T, R>(
    item: &mut InplaceOption<'_, T>,
    #[verus_callback(Tracked<&mut NativeMutLoan<'_, T>>)]
    f: impl FnOnce(*mut u8, &mut bool) -> R,
) -> R {
    let mut initialized = item.forget();
    proof_decl! { let tracked mut loan; }
    let base = {
        proof_with! { => Tracked(loan) }
        item.as_mut_ptr()
    }.cast::<u8>();
    proof_with! { Tracked(&mut loan) }
    let r = (f)(base, &mut initialized);
    proof! { finish_inplace_loan(loan); }
    if initialized {
        // SAFETY: the caller ensures that `item` is initialized.
        unsafe { item.set_init_unchecked() };
    }
    r
}

// The annotations below are instantiated for SavedState in vmcore, not
// asserted for every implementation of StructDecodeMetadata.
#[verus_specialize(specialize_table_read_message)]
impl<'de, T, R> MessageDecode<'de, T, R> for TableEncoder
where
    T: StructDecodeMetadata<'de, R>,
{
    #[verus_verify]
    #[verus_spec_case(saved_time, result =>
        requires
            old(item).storage_valid(),
            saved_time_values(reader.reader_bytes()) is Some,
        ensures
            result is Ok,
            final(item).storage_valid(),
            final(item).initialized(),
            final(item).contents() == if saved_time_values(reader.reader_bytes())->Some_0.len() > 0 {
                vstd::raw_ptr::MemContents::Init(T { vmtime: VmTime(saved_time_last(reader.reader_bytes())) })
            } else if old(item).initialized() {
                old(item).contents()
            } else { vstd::raw_ptr::MemContents::Init(T { vmtime: VmTime(0) }) },
            final(final(item).storage()).mem_contents()
                == final(old(item).storage()).mem_contents(),
    )]
    #[verus_spec(result =>
        requires
            old(item).storage_valid(),
            reader.reader_bytes().len() == 0,
        ensures
            result is Ok,
            final(item).storage_valid(),
            final(item).initialized(),
            final(item).contents() == if old(item).initialized() {
                old(item).contents()
            } else { vstd::raw_ptr::MemContents::Init(T { vmtime: VmTime(0) }) },
            final(final(item).storage()).mem_contents()
                == final(old(item).storage()).mem_contents(),
    )]
    fn read_message(
        item: &mut InplaceOption<'_, T>,
        reader: MessageReader<'de, '_, R>,
    ) -> crate::Result<()> {
        // SAFETY: T guarantees that the metadata is valid.
        unsafe {
            proof! { item.storage_observation(); }
            proof_decl! {
                let ghost before = item.contents();
                let ghost was_initialized = item.initialized();
            }
            proof_case! { saved_time {
                proof_decl! { let ghost bytes = reader.reader_bytes(); }
                proof_with! { Ghost(
                    |contents: vstd::raw_ptr::MemContents<T>, initialized: bool, result: Result<(), Error>|
                        initialized && result.is_ok()
                        && contents == if saved_time_values(bytes)->Some_0.len() > 0 {
                            vstd::raw_ptr::MemContents::Init(T { vmtime: VmTime(saved_time_last(bytes)) })
                        } else if was_initialized { before }
                        else { vstd::raw_ptr::MemContents::Init(T { vmtime: VmTime(0) }) })
                }
            } else {
                proof_with! { Ghost(
                    |contents: vstd::raw_ptr::MemContents<T>, initialized: bool, result: Result<(), Error>|
                        initialized && result.is_ok()
                        && contents == if was_initialized { before }
                            else { vstd::raw_ptr::MemContents::Init(T { vmtime: VmTime(0) }) })
                }
            } }
            run_inplace(item, #[verus_spec_case(saved_time, result: EmptyMessageResult =>
                with permission: Tracked<&mut NativeMutLoan<'_, T>>,
                requires
                    saved_time_values(reader.reader_bytes()) is Some,
                    permission@.ptr() as *mut u8 == base,
                    !*initialized || permission@.storage().mem_contents().is_init(),
                ensures
                    result is Ok, *final(initialized),
                    final(permission@).ptr() == permission@.ptr(),
                    final(final(permission@).storage()).mem_contents()
                        == final(permission@.storage()).mem_contents(),
                    final(permission@).storage().mem_contents()
                        == if saved_time_values(reader.reader_bytes())->Some_0.len() > 0 {
                            vstd::raw_ptr::MemContents::Init(T { vmtime: VmTime(saved_time_last(reader.reader_bytes())) })
                        } else if *old(initialized) {
                            permission@.storage().mem_contents()
                        } else { vstd::raw_ptr::MemContents::Init(T { vmtime: VmTime(0) }) },
            )]
            #[verus_spec(result: EmptyMessageResult =>
                with permission: Tracked<&mut NativeMutLoan<'_, T>>,
                requires
                    permission@.ptr() as *mut u8 == base,
                    !*initialized || permission@.storage().mem_contents().is_init(),
                ensures
                    result is Ok,
                    *final(initialized),
                    final(permission@).ptr() == permission@.ptr(),
                    final(final(permission@).storage()).mem_contents()
                        == final(permission@.storage()).mem_contents(),
                    final(permission@).storage().mem_contents() == if *old(initialized) {
                        permission@.storage().mem_contents()
                    } else { vstd::raw_ptr::MemContents::Init(T { vmtime: VmTime(0) }) },
            )] |base, initialized| {
                proof_case! { saved_time {
                    proof_with! { native_specialized_call(
                        crate::table::decode::_VERUS_VERIFIED_saved_read_fields) }
                } else {} }
                proof_with! { native_single_field_path(
                    permission, T, VmTime, u64, "vmtime", "0", finish_inplace_loan, field)
                }
                read_fields(
                    {
                        proof_case! { saved_time {
                            proof_with! { native_singleton_slice_bound(const { 1u32 }) }
                        } else {
                            proof_with! { native_singleton_slice }
                        } }
                        T::NUMBERS
                    },
                    {
                        proof_with! { native_singleton_slice_bound(
                            const { <VarintField as FieldDecode<'static, u64, R>>::ENTRY.erase() })
                        }
                        <T as StructDecodeMetadata<'de, R>>::DECODERS
                    },
                    {
                        proof_with! { native_singleton_slice_bound(const { 0usize }) }
                        T::OFFSETS
                    },
                    base,
                    initialized,
                    reader,
                )
            })
        }
    }
}

impl<'de, T, R> FieldDecode<'de, T, R> for TableEncoder
where
    T: StructDecodeMetadata<'de, R>,
{
    // Override the default implementation to use the table decoder directly.
    // This saves code size by avoiding extra stub functions and vtables.
    const ENTRY: DecoderEntry<'de, T, R> = DecoderEntry::table();

    fn read_field(
        item: &mut InplaceOption<'_, T>,
        reader: FieldReader<'de, '_, R>,
    ) -> crate::Result<()> {
        // SAFETY: T guarantees that the metadata is valid.
        unsafe {
            run_inplace(item, |base, initialized| {
                read_message(
                    T::NUMBERS,
                    T::DECODERS,
                    T::OFFSETS,
                    base,
                    initialized,
                    reader,
                )
            })
        }
    }

    fn default_field(item: &mut InplaceOption<'_, T>) -> crate::Result<()> {
        // SAFETY: T guarantees that the metadata is valid.
        unsafe {
            run_inplace(item, |base, initialized| {
                default_fields(T::DECODERS, T::OFFSETS, base, initialized)
            })
        }
    }
}

/// Read a field as a message from the provided field metadata.
///
/// # Safety
///
/// The caller must ensure that `base` points to a location that can be written
/// to, that `struct_initialized` is set correctly, and that the metadata is
/// correct and complete for the type of the object pointed to by `base`.
#[doc(hidden)] // only used publicly in mesh_derive
pub unsafe fn read_message<R>(
    numbers: &[u32],
    decoders: &[ErasedDecoderEntry],
    offsets: &[usize],
    base: *mut u8,
    struct_initialized: &mut bool,
    reader: FieldReader<'_, '_, R>,
) -> Result<(), Error> {
    assert_eq!(numbers.len(), decoders.len());
    assert_eq!(numbers.len(), offsets.len());
    // SAFETY: guaranteed by caller and by the assertions above.
    unsafe {
        // Convert the slices to pointers and a single length to shrink
        // code size.
        read_message_by_ptr(
            numbers.len(),
            numbers.as_ptr(),
            decoders.as_ptr(),
            offsets.as_ptr(),
            base,
            struct_initialized,
            reader,
        )
    }
}

// Don't inline this since it is used by every table decoder instantiation.
#[inline(never)]
unsafe fn read_message_by_ptr<R>(
    count: usize,
    numbers: *const u32,
    decoders: *const ErasedDecoderEntry,
    offsets: *const usize,
    base: *mut u8,
    struct_initialized: &mut bool,
    reader: FieldReader<'_, '_, R>,
) -> Result<(), Error> {
    // SAFETY: guaranteed by caller.
    unsafe {
        read_fields_inline(
            slice::from_raw_parts(numbers, count),
            slice::from_raw_parts(decoders, count),
            slice::from_raw_parts(offsets, count),
            base,
            struct_initialized,
            reader.message()?,
        )
    }
}

/// Read a message from the provided field metadata.
///
/// # Safety
///
/// The caller must ensure that `base` points to a location that can be written
/// to, that `struct_initialized` is set correctly, and that the metadata is
/// correct and complete for the type of the object pointed to by `base`.
#[verus_verify(publish_source)]
#[verus_specialize(specialize_saved_read_fields, saved_time)]
#[verus_verify(publish_verified)]
#[verus_spec_case(saved_time, result =>
    with Tracked(field_loan): Tracked<&mut NativeMutLoan<'_, u64>>,
    requires
        saved_time_table(numbers@, decoders@, offsets@, reader.reader_bytes()),
        old(field_loan).ptr() as *mut u8 == base,
        !*old(struct_initialized) || old(field_loan).storage().mem_contents().is_init(),
    ensures
        result is Ok, *final(struct_initialized),
        final(field_loan).ptr() == old(field_loan).ptr(),
        final(field_loan).storage().mem_contents() == saved_time_result(
            reader.reader_bytes(), old(field_loan).storage().mem_contents(), *old(struct_initialized)),
        final(final(field_loan).storage()).mem_contents()
            == final(old(field_loan).storage()).mem_contents(),
)]
#[verus_spec(result =>
    with Tracked(field_loan): Tracked<&mut NativeMutLoan<'_, u64>>,
    requires
        numbers@.len() == 1,
        decoders@.len() == 1,
        offsets@.len() == 1,
        is_supported_u64_default_entry(decoders[0]),
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
)]
unsafe fn read_fields<R>(
    numbers: &[u32],
    decoders: &[ErasedDecoderEntry],
    offsets: &[usize],
    base: *mut u8,
    struct_initialized: &mut bool,
    reader: MessageReader<'_, '_, R>,
) -> Result<(), Error> {
    assert_eq!(numbers.len(), decoders.len());
    assert_eq!(numbers.len(), offsets.len());
    proof_decl! {
        let tracked numbers_loan;
        let tracked decoders_loan;
        let tracked offsets_loan;
    }
    // SAFETY: guaranteed by caller and by the assertions above.
    unsafe {
        // Convert the slices to pointers and a single length to shrink
        // code size.
        proof_case! { saved_time {
            proof_with! { native_specialized_call_with(_VERUS_VERIFIED_saved_read_fields_by_ptr,
                (Tracked(&numbers_loan), Tracked(&decoders_loan), Tracked(&offsets_loan),
                    Tracked(&mut *field_loan))) }
        } else {
            proof_with! {
                Tracked(&numbers_loan), Tracked(&decoders_loan), Tracked(&offsets_loan),
                Tracked(&mut *field_loan)
            }
        } }
        read_fields_by_ptr(
            numbers.len(),
            {
                proof_with! { native_slice_loan(numbers, numbers_loan) }
                numbers.as_ptr()
            },
            {
                proof_with! { native_slice_loan(decoders, decoders_loan) }
                decoders.as_ptr()
            },
            {
                proof_with! { native_slice_loan(offsets, offsets_loan) }
                offsets.as_ptr()
            },
            base,
            struct_initialized,
            reader,
        )
    }
}

// Don't inline this since it is used by every table decoder instantiation.
#[inline(never)]
#[verus_specialize(specialize_saved_read_fields_by_ptr, saved_time)]
#[verus_verify]
#[verus_spec_case(saved_time, result =>
    with
        Tracked(numbers_loan): Tracked<&NativeSliceLoan<'_, u32>>,
        Tracked(decoders_loan): Tracked<&NativeSliceLoan<'_, ErasedDecoderEntry>>,
        Tracked(offsets_loan): Tracked<&NativeSliceLoan<'_, usize>>,
        Tracked(field_loan): Tracked<&mut NativeMutLoan<'_, u64>>,
    requires
        count == 1,
        numbers == numbers_loan.ptr(), decoders == decoders_loan.ptr(), offsets == offsets_loan.ptr(),
        saved_time_table(numbers_loan.storage()@, decoders_loan.storage()@,
            offsets_loan.storage()@, reader.reader_bytes()),
        old(field_loan).ptr() as *mut u8 == base,
        !*old(struct_initialized) || old(field_loan).storage().mem_contents().is_init(),
    ensures
        result is Ok, *final(struct_initialized),
        final(field_loan).ptr() == old(field_loan).ptr(),
        final(field_loan).storage().mem_contents() == saved_time_result(
            reader.reader_bytes(), old(field_loan).storage().mem_contents(), *old(struct_initialized)),
        final(final(field_loan).storage()).mem_contents()
            == final(old(field_loan).storage()).mem_contents(),
)]
#[verus_spec(result =>
    with
        Tracked(numbers_loan): Tracked<&NativeSliceLoan<'_, u32>>,
        Tracked(decoders_loan): Tracked<&NativeSliceLoan<'_, ErasedDecoderEntry>>,
        Tracked(offsets_loan): Tracked<&NativeSliceLoan<'_, usize>>,
        Tracked(field_loan): Tracked<&mut NativeMutLoan<'_, u64>>,
    requires
        count == 1,
        numbers == numbers_loan.ptr(),
        decoders == decoders_loan.ptr(),
        offsets == offsets_loan.ptr(),
        numbers_loan.storage()@.len() == count,
        decoders_loan.storage()@.len() == count,
        offsets_loan.storage()@.len() == count,
        is_supported_u64_default_entry(decoders_loan.storage()[0]),
        offsets_loan.storage()[0] == 0,
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
)]
unsafe fn read_fields_by_ptr<R>(
    count: usize,
    numbers: *const u32,
    decoders: *const ErasedDecoderEntry,
    offsets: *const usize,
    base: *mut u8,
    struct_initialized: &mut bool,
    reader: MessageReader<'_, '_, R>,
) -> Result<(), Error> {
    // SAFETY: guaranteed by caller.
    unsafe {
        proof_case! { saved_time {
            proof_with! { native_specialized_call_with(_VERUS_VERIFIED_saved_read_fields_inline,
                (Tracked(&mut *field_loan),)) }
        } else {
            proof_with! { Tracked(&mut *field_loan) }
        } }
        read_fields_inline(
            {
                proof_with! { native_slice_from_raw_parts(Tracked(numbers_loan)) }
                slice::from_raw_parts(numbers, count)
            },
            {
                proof_with! { native_slice_from_raw_parts(Tracked(decoders_loan)) }
                slice::from_raw_parts(decoders, count)
            },
            {
                proof_with! { native_slice_from_raw_parts(Tracked(offsets_loan)) }
                slice::from_raw_parts(offsets, count)
            },
            base,
            struct_initialized,
            reader,
        )
    }
}

/// Reads fields from the provided field metadata.
///
/// # Safety
///
/// The caller must ensure that `base` points to a location that can be written
/// to, that `initialized` is set correctly, and that the metadata is correct
/// and complete for the type of the object pointed to by `base`.
///
/// The singleton proofs receive an exclusive projection of the whole object.
/// Field flags come from the actual struct flag.
#[verus_specialize(specialize_saved_read_fields_inline, saved_time)]
#[verus_verify]
#[verus_spec_case(saved_time, result =>
    with Tracked(field_loan): Tracked<&mut NativeMutLoan<'_, u64>>,
    requires
        saved_time_table(numbers@, decoders@, offsets@, reader.reader_bytes()),
        old(field_loan).ptr() as *mut u8 == base,
        !*old(struct_initialized) || old(field_loan).storage().mem_contents().is_init(),
    ensures
        result is Ok, *final(struct_initialized),
        final(field_loan).ptr() == old(field_loan).ptr(),
        final(field_loan).storage().mem_contents() == saved_time_result(
            reader.reader_bytes(), old(field_loan).storage().mem_contents(), *old(struct_initialized)),
        final(final(field_loan).storage()).mem_contents()
            == final(old(field_loan).storage()).mem_contents(),
)]
#[verus_spec(result =>
    with Tracked(field_loan): Tracked<&mut NativeMutLoan<'_, u64>>,
    requires
        numbers@.len() == 1,
        decoders@.len() >= 1,
        offsets@.len() >= 1,
        is_supported_u64_default_entry(decoders[0]),
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
)]
unsafe fn read_fields_inline<R>(
    numbers: &[u32],
    decoders: &[ErasedDecoderEntry],
    offsets: &[usize],
    base: *mut u8,
    struct_initialized: &mut bool,
    reader: MessageReader<'_, '_, R>,
) -> Result<(), Error> {
    const STACK_LIMIT: usize = 32;
    let mut field_init_static;
    let mut field_init_dynamic;
    let field_inits = if numbers.len() <= STACK_LIMIT {
        field_init_static = [false; STACK_LIMIT];
        field_init_static[..numbers.len()].fill(*struct_initialized);
        &mut field_init_static[..numbers.len()]
    } else {
        proof_with! { native_unreachable }
        field_init_dynamic = vec![*struct_initialized; numbers.len()];
        &mut field_init_dynamic[..]
    };

    // SAFETY: guaranteed by caller.
    let r = unsafe {
        proof_case! { saved_time {
            proof_with! { native_specialized_call_with(_VERUS_VERIFIED_saved_read_fields_inner,
                (Tracked(&mut *field_loan),)) }
        } else {
            proof_with! { Tracked(&mut *field_loan) }
        } }
        read_fields_inner(numbers, decoders, offsets, base, field_inits, reader)
    };
    *struct_initialized = true;
    if r.is_err() && {
        proof_with! { native_unreachable }
        !field_inits.iter().all(|&b| b)
    } {
        // Drop any initialized fields.
        proof_with! { native_unreachable }
        {
            for ((field_init, &offset), decoder) in field_inits.iter_mut().zip(offsets).zip(decoders) {
                if *field_init {
                    // SAFETY: guaranteed by the caller.
                    unsafe {
                        decoder.drop_field(base.add(offset));
                    }
                }
            }
            *struct_initialized = false;
        }
    }
    r
}

/// Reads fields from the provided field metadata, but does not drop any fields
/// of a partially initialized message on failure.
///
/// The default proof covers empty readers. The saved-time case also covers
/// repeated field-one varints with NoResources and preserves the same loan.
#[verus_specialize(specialize_saved_read_fields_inner, saved_time)]
#[verus_verify]
#[verus_spec_case(saved_time, result =>
    with Tracked(field_loan): Tracked<&mut NativeMutLoan<'_, u64>>,
    requires
        saved_time_table(numbers@, decoders@, offsets@, reader.reader_bytes()),
        old(field_init)@.len() == 1,
        old(field_loan).ptr() as *mut u8 == base,
        !old(field_init)[0] || old(field_loan).storage().mem_contents().is_init(),
    ensures
        result is Ok, final(field_init)@ == seq![true],
        final(field_loan).ptr() == old(field_loan).ptr(),
        final(field_loan).storage().mem_contents() == saved_time_result(
            reader.reader_bytes(), old(field_loan).storage().mem_contents(), old(field_init)[0]),
        final(final(field_loan).storage()).mem_contents()
            == final(old(field_loan).storage()).mem_contents(),
)]
#[verus_spec(result =>
    with Tracked(field_loan): Tracked<&mut NativeMutLoan<'_, u64>>,
    requires
        numbers@.len() == 1,
        decoders@.len() >= 1,
        offsets@.len() >= 1,
        is_supported_u64_default_entry(decoders[0]),
        offsets[0] == 0,
        old(field_init)@.len() == 1,
        reader.reader_bytes().len() == 0,
        old(field_loan).ptr() as *mut u8 == base,
        old(field_init)[0] ==> old(field_loan).storage().mem_contents().is_init(),
    ensures
        result is Ok,
        final(field_init)@ == seq![true],
        final(field_loan).ptr() == old(field_loan).ptr(),
        final(field_loan).storage().mem_contents() == if old(field_init)[0] {
            old(field_loan).storage().mem_contents()
        } else { vstd::raw_ptr::MemContents::Init(0) },
        final(final(field_loan).storage()).mem_contents()
            == final(old(field_loan).storage()).mem_contents(),
)]
unsafe fn read_fields_inner<R>(
    numbers: &[u32],
    decoders: &[ErasedDecoderEntry],
    offsets: &[usize],
    base: *mut u8,
    field_init: &mut [bool],
    reader: MessageReader<'_, '_, R>,
) -> Result<(), Error> {
    let decoders = &decoders[..numbers.len()];
    let offsets = &offsets[..numbers.len()];
    let field_init = &mut field_init[..numbers.len()];
    proof_decl! {
        let ghost before = field_loan.storage().mem_contents();
        let ghost was_initialized = field_init[0];
    }
    proof_case! { saved_time {
        proof_decl! {
            let ghost values = crate::protobuf::saved_time_values(reader.reader_bytes())->Some_0;
            let ghost input = reader.reader_bytes();
            let ghost resources = reader.resource_range();
            let ghost state = reader.decode_state();
            let ghost mut consumed: int = 0;
        }
        proof_with! { native_iterator(crate::protobuf::saved_time_iterator_next::<R>) }
    } else {
        proof_with! { native_empty_iterator(crate::protobuf::empty_iterator_step::<R>) }
    } }
    #[verus_spec_case(saved_time, iter =>
        invariant
            0 <= consumed <= values.len(),
            values == crate::protobuf::saved_time_values(input)->Some_0,
            crate::protobuf::saved_time_values(iter.reader_bytes()) == Some(values.skip(consumed)),
            iter.resource_range() == resources,
            iter.decode_state() == state,
            numbers@ == seq![1u32], decoders@.len() == 1, offsets@ == seq![0usize],
            is_u64_read_entry(decoders[0]),
            field_init@.len() == 1,
            field_loan.ptr() as *mut u8 == base,
            final(field_loan.storage()).mem_contents()
                == final(old(field_loan).storage()).mem_contents(),
            field_init[0] == (was_initialized || consumed > 0),
            !field_init[0] || field_loan.storage().mem_contents().is_init(),
            field_loan.storage().mem_contents() == if consumed == 0 { before }
                else { vstd::raw_ptr::MemContents::Init(values[consumed - 1]) },
        ensures consumed == values.len(),
        decreases values.len() - consumed,
    )]
    for field in reader {
        let (number, reader) = field?;
        proof_case! { saved_time {
            proof_decl! { let ghost value = reader.varint_payload()->Some_0; }
            proof_with! { native_singleton_position }
        } else { } }
        if let Some(index) = numbers.iter().position(|&n| n == number) {
            let decoder = &decoders[index];
            // SAFETY: the decoder is valid according to the caller.
            unsafe {
                proof_case! { saved_time {
                    proof_with! { native_dynamic_dispatch(
                        const { <crate::encoding::VarintField as FieldDecode<'static, u64, crate::NoResources>>::ENTRY.erase() },
                        const { unsafe {
                            &*const {
                                if cfg!(verus_table_negative_entry) {
                                    <crate::encoding::VarintField as FieldDecode<'static, u32, crate::NoResources>>::ENTRY.erase()
                                } else {
                                    <crate::encoding::VarintField as FieldDecode<'static, u64, crate::NoResources>>::ENTRY.erase()
                                }
                            }.0.cast::<StaticDecoderVtable<'static, SavedReadResources>>()
                        }.read_fn },
                        const { read_field_dyn::<u64, SavedReadResources, crate::encoding::VarintField>
                            as unsafe fn(*mut u8, *mut bool, FieldReader<'static, '_, SavedReadResources>) -> Result<(), Error> },
                        _VERUS_VERIFIED_read_u64_field_dyn_body,
                        Tracked(&mut *field_loan),
                        Ghost(|contents: vstd::raw_ptr::MemContents<u64>, flag: bool, result: Result<(), Error>|
                            contents == vstd::raw_ptr::MemContents::Init(value) && flag && result.is_ok()),
                        finish_read_flag_loan
                    ) }
                } else { } }
                decoder.read_field(base.add(offsets[index]), &mut field_init[index], reader)?;
            }
        }
        proof_case! { saved_time {
            proof! {
                assert(values.skip(consumed).skip(1) == values.skip(consumed + 1));
                consumed = consumed + 1;
            }
        } else { } }
    }
    proof_case! { saved_time {
        proof! {
            assert(consumed == values.len());
            assert(field_loan.storage().mem_contents() == if values.len() > 0 {
                vstd::raw_ptr::MemContents::Init(values.last())
            } else { before });
        }
        proof_decl! {
            let ghost expected = saved_time_result(input, before, was_initialized);
        }
    } else { } }
    proof_decl! {
        let ghost before = field_loan.storage().mem_contents();
        let ghost was_initialized = field_init[0];
    }
    #[verus_spec(iter =>
        invariant
            iter.seq().len() == 1,
            *iter.seq()[0].0.1 == 0,
            is_supported_u64_default_entry(*iter.seq()[0].1),
            iter.index() != 0 || *iter.seq()[0].0.0 == was_initialized,
            iter.index() <= 0 || *final(iter.seq()[0].0.0),
            field_loan.ptr() as *mut u8 == base,
            final(field_loan.storage()).mem_contents()
                == final(old(field_loan).storage()).mem_contents(),
            iter.index() != 0 || field_loan.storage().mem_contents() == before,
            iter.index() <= 0 || field_loan.storage().mem_contents()
                == if was_initialized { before } else { vstd::raw_ptr::MemContents::Init(0) },
            !*iter.seq()[0].0.0 || before.is_init(),
    )]
    #[verus_spec_case(saved_time, iter =>
        invariant
            iter.seq().len() == 1,
            *iter.seq()[0].0.1 == 0,
            is_supported_u64_default_entry(*iter.seq()[0].1),
            iter.index() != 0 || *iter.seq()[0].0.0 == was_initialized,
            iter.index() <= 0 || *final(iter.seq()[0].0.0),
            field_loan.ptr() as *mut u8 == base,
            final(field_loan.storage()).mem_contents()
                == final(old(field_loan).storage()).mem_contents(),
            iter.index() != 0 || field_loan.storage().mem_contents() == before,
            iter.index() <= 0 || field_loan.storage().mem_contents() == expected,
            expected == if was_initialized { before } else { vstd::raw_ptr::MemContents::Init(0) },
            !*iter.seq()[0].0.0 || before.is_init(),
    )]
    for ((field_init, &offset), decoder) in field_init.iter_mut().zip(offsets).zip(decoders) {
        if !*field_init {
            // SAFETY: the decoder is valid according to the caller.
            unsafe {
                proof! {
                    reveal(is_u64_default_entry);
                    reveal(is_supported_u64_default_entry);
                    u64_typed_default::<()>(field_loan.storage().mem_contents(), *field_init);
                    u64_typed_default::<crate::NoResources>(field_loan.storage().mem_contents(), *field_init);
                }
                proof_with! { native_dynamic_dispatch_pair(
                    const { <crate::encoding::VarintField as FieldDecode<'static, u64, ()>>::ENTRY.erase() },
                    const { unsafe {
                        &*const { <crate::encoding::VarintField as FieldDecode<'static, u64, ()>>::ENTRY.erase() }
                            .0.cast::<StaticDecoderVtable<'static, ()>>()
                    }.default_fn },
                    const { default_field_dyn::<u64, (), crate::encoding::VarintField>
                        as unsafe fn(*mut u8, *mut bool) -> Result<(), Error> },
                    _VERUS_VERIFIED_default_field_dyn::<u64, (), crate::encoding::VarintField>,
                    const { <crate::encoding::VarintField as FieldDecode<'static, u64, crate::NoResources>>::ENTRY.erase() },
                    const { unsafe {
                        &*const {
                            if cfg!(verus_pair_negative_entry) {
                                <crate::encoding::VarintField as FieldDecode<'static, u32, crate::NoResources>>::ENTRY.erase()
                            } else {
                                <crate::encoding::VarintField as FieldDecode<'static, u64, crate::NoResources>>::ENTRY.erase()
                            }
                        }
                            .0.cast::<StaticDecoderVtable<'static, ()>>()
                    }.default_fn },
                    const { default_field_dyn::<u64, crate::NoResources, crate::encoding::VarintField>
                        as unsafe fn(*mut u8, *mut bool) -> Result<(), Error> },
                    _VERUS_VERIFIED_default_field_dyn::<u64, crate::NoResources, crate::encoding::VarintField>,
                    Tracked(&mut *field_loan),
                    Ghost(|contents: vstd::raw_ptr::MemContents<u64>, flag: bool, result: Result<(), Error>|
                        contents == vstd::raw_ptr::MemContents::Init(0) && flag && result.is_ok()),
                    finish_init_loan::<bool>
                ) }
                decoder.default_field(base.add(offset), field_init)?;
            }
            assert!(*field_init);
        }
    }
    Ok(())
}

/// Sets fields to their default values from the provided field metadata.
///
/// # Safety
///
/// The caller must ensure that `base` points to a location that can be written
/// to, that `struct_initialized` is set correctly, and that the metadata is
/// correct and complete for the type of the object pointed to by `base`.
unsafe fn default_fields(
    decoders: &[ErasedDecoderEntry],
    offsets: &[usize],
    base: *mut u8,
    struct_initialized: &mut bool,
) -> Result<(), Error> {
    assert_eq!(decoders.len(), offsets.len());
    // SAFETY: guaranteed by caller and by the assertion above.
    unsafe {
        default_fields_by_ptr(
            decoders.len(),
            decoders.as_ptr(),
            offsets.as_ptr(),
            base,
            struct_initialized,
        )
    }
}

#[inline(never)]
unsafe fn default_fields_by_ptr(
    count: usize,
    decoders: *const ErasedDecoderEntry,
    offsets: *const usize,
    base: *mut u8,
    struct_initialized: &mut bool,
) -> Result<(), Error> {
    // SAFETY: guaranteed by caller.
    unsafe {
        default_fields_inline(
            slice::from_raw_parts(decoders, count),
            slice::from_raw_parts(offsets, count),
            base,
            struct_initialized,
        )
    }
}

unsafe fn default_fields_inline(
    decoders: &[ErasedDecoderEntry],
    offsets: &[usize],
    base: *mut u8,
    struct_initialized: &mut bool,
) -> Result<(), Error> {
    for (i, (&offset, decoder)) in offsets.iter().zip(decoders).enumerate() {
        let mut field_initialized = *struct_initialized;
        // SAFETY: the decoder is valid according to the caller.
        let r = unsafe { decoder.default_field(base.add(offset), &mut field_initialized) };
        if let Err(err) = r {
            if !field_initialized || !*struct_initialized {
                // Drop initialized fields.
                let initialized_until = i;
                for (i, (&offset, decoder)) in offsets.iter().zip(decoders).enumerate() {
                    if i < initialized_until
                        || (i == initialized_until && field_initialized)
                        || (i > initialized_until && *struct_initialized)
                    {
                        // SAFETY: the decoder is valid according to the caller, and the field is initialized.
                        unsafe {
                            decoder.drop_field(base.add(offset));
                        }
                    }
                }
                *struct_initialized = false;
            }
            return Err(err);
        }
        assert!(field_initialized);
    }
    *struct_initialized = true;
    Ok(())
}

/// The struct metadata for decoding a struct.
///
/// # Safety
///
/// The implementor must ensure that the `DECODERS` are correct and complete for
/// `Self`, such that if every field is decoded, then the struct value is valid.
pub unsafe trait StructDecodeMetadata<'de, R>: StructMetadata {
    /// The list of decoder vtables.
    const DECODERS: &'static [ErasedDecoderEntry];
}

/// An entry in the decoder table.
///
/// This contains the metadata necessary to apply an decoder to a field.
///
/// This cannot be instantiated directly; use [`FieldDecode::ENTRY`] to get an
/// instance for a particular decoder.
pub struct DecoderEntry<'a, T, R>(
    ErasedDecoderEntry,
    PhantomData<fn(&mut T, &mut R, &'a mut ())>,
);

impl<'a, T, R> DecoderEntry<'a, T, R> {
    /// # Safety
    /// The caller must ensure that the erased entry is an valid entry for `T`
    /// and `R`.
    pub(crate) const unsafe fn new_unchecked(entry: ErasedDecoderEntry) -> Self {
        Self(entry, PhantomData)
    }

    pub(crate) const fn custom<E: FieldDecode<'a, T, R>>() -> Self {
        Self(
            ErasedDecoderEntry(
                core::ptr::from_ref(
                    const {
                        &StaticDecoderVtable {
                            read_fn: read_field_dyn::<T, R, E>,
                            default_fn: default_field_dyn::<T, R, E>,
                            drop_fn: if core::mem::needs_drop::<T>() {
                                Some(drop_field_dyn::<T>)
                            } else {
                                None
                            },
                        }
                    },
                )
                .cast(),
            ),
            PhantomData,
        )
    }

    const fn table() -> Self
    where
        T: StructDecodeMetadata<'a, R>,
    {
        Self(
            ErasedDecoderEntry(
                core::ptr::from_ref(
                    const {
                        &DecoderTable {
                            count: T::NUMBERS.len(),
                            numbers: T::NUMBERS.as_ptr(),
                            decoders: T::DECODERS.as_ptr(),
                            offsets: T::OFFSETS.as_ptr(),
                        }
                    },
                )
                .cast::<()>()
                .wrapping_byte_add(ENTRY_IS_TABLE),
            ),
            PhantomData,
        )
    }

    /// Erases the type of the decoder entry.
    pub const fn erase(&self) -> ErasedDecoderEntry {
        self.0
    }
}

/// An entry in a [`StructDecodeMetadata::DECODERS`] table.
//
// Internally, this is a pointer to either a vtable or a table.
// The low bit is used to distinguish between the two.
#[verus_verify]
#[derive(Copy, Clone, Debug)]
pub struct ErasedDecoderEntry(*const ());

// SAFETY: the entry represents a set of integers and function pointers, which
// have no cross-thread constraints.
unsafe impl Send for ErasedDecoderEntry {}
// SAFETY: the entry represents a set of integers and function pointers, which
// have no cross-thread constraints.
unsafe impl Sync for ErasedDecoderEntry {}

const ENTRY_IS_TABLE: usize = 1;

const _: () = assert!(align_of::<ErasedDecoderEntry>() > ENTRY_IS_TABLE);
const _: () = assert!(align_of::<StaticDecoderVtable<'_, ()>>() > ENTRY_IS_TABLE);

impl ErasedDecoderEntry {
    /// Decodes the entry into either a vtable or a table.
    ///
    /// # Safety
    /// The caller must ensure that the encoder was for resource type `R`.
    unsafe fn decode<'de, R>(&self) -> Result<&StaticDecoderVtable<'de, R>, &DecoderTable> {
        // SAFETY: guaranteed by caller.
        unsafe {
            if self.0 as usize & ENTRY_IS_TABLE == 0 {
                Ok(&*self.0.cast::<StaticDecoderVtable<'_, R>>())
            } else {
                Err(&*self
                    .0
                    .wrapping_byte_sub(ENTRY_IS_TABLE)
                    .cast::<DecoderTable>())
            }
        }
    }

    /// Reads a field using the decoder metadata.
    ///
    /// # Safety
    /// The caller must ensure that the decoder was for resource type `R` and
    /// the object type matches what `ptr` is pointing to. `*init` must be set
    /// if and only if the field is initialized.
    pub unsafe fn read_field<R>(
        &self,
        ptr: *mut u8,
        init: &mut bool,
        reader: FieldReader<'_, '_, R>,
    ) -> Result<(), Error> {
        // SAFETY: guaranteed by caller.
        unsafe {
            match self.decode::<R>() {
                Ok(vtable) => (vtable.read_fn)(ptr, init, reader),
                Err(table) => read_message_by_ptr(
                    table.count,
                    table.numbers,
                    table.decoders,
                    table.offsets,
                    ptr,
                    init,
                    reader,
                ),
            }
        }
    }

    /// Initializes a value to its default state using the decoder metadata.
    ///
    /// # Safety
    /// The caller must ensure that the decoder was for the object type matching
    /// what `ptr` is pointing to.
    pub unsafe fn default_field(&self, ptr: *mut u8, init: &mut bool) -> Result<(), Error> {
        // SAFETY: guaranteed by caller.
        unsafe {
            match self.decode::<()>() {
                Ok(vtable) => (vtable.default_fn)(ptr, init),
                Err(table) => {
                    default_fields_by_ptr(table.count, table.decoders, table.offsets, ptr, init)
                }
            }
        }
    }

    /// Drops a value in place using the decoder metadata.
    ///
    /// # Safety
    /// The caller must ensure that the decoder was for the object type matching
    /// what `ptr` is pointing to, and that `ptr` is ready to be dropped.
    pub unsafe fn drop_field(&self, ptr: *mut u8) {
        // SAFETY: guaranteed by caller.
        unsafe {
            match self.decode::<()>() {
                Ok(vtable) => {
                    if let Some(drop_fn) = vtable.drop_fn {
                        drop_fn(ptr);
                    }
                }
                Err(table) => {
                    for i in 0..table.count {
                        let offset = *table.offsets.add(i);
                        let decoder = &*table.decoders.add(i);
                        decoder.drop_field(ptr.add(offset));
                    }
                }
            }
        }
    }
}

struct DecoderTable {
    count: usize,
    numbers: *const u32,
    decoders: *const ErasedDecoderEntry,
    offsets: *const usize,
}

/// A vtable for decoding a message.
#[repr(C)] // to ensure the layout is the same regardless of R
struct StaticDecoderVtable<'de, R> {
    read_fn: unsafe fn(*mut u8, init: *mut bool, FieldReader<'de, '_, R>) -> Result<(), Error>,
    default_fn: unsafe fn(*mut u8, init: *mut bool) -> Result<(), Error>,
    drop_fn: Option<unsafe fn(*mut u8)>,
}

// The concrete successful-varint instance is checked below. Other encodings
// and error outcomes remain obligations of their own decoder instances.
#[verus_specialize(specialize_read_field_dyn)]
#[verus_verify]
#[verus_spec(result =>
    with
        Tracked(field_loan): Tracked<&mut NativeMutLoan<'_, T>>,
        Tracked(init_loan): Tracked<&mut NativeInitLoan<'_, bool>>,
        Ghost(post): Ghost<InplacePost<T, Result<(), Error>>>,
    requires
        old(field_loan).ptr() as *mut u8 == field,
        old(init_loan).ptr() == init,
        !*old(init_loan).storage()
            || old(field_loan).storage().mem_contents().is_init(),
        reader.varint_payload() is Some,
        post(vstd::raw_ptr::MemContents::Init(reader.varint_payload()->Some_0), true, Ok(())),
    ensures
        result == Ok(()),
        *final(init_loan).storage(),
        final(field_loan).storage().mem_contents()
            == vstd::raw_ptr::MemContents::Init(reader.varint_payload()->Some_0),
        final(field_loan).ptr() == old(field_loan).ptr(),
        final(init_loan).ptr() == old(init_loan).ptr(),
        final(final(field_loan).storage()).mem_contents()
            == final(old(field_loan).storage()).mem_contents(),
        *final(final(init_loan).storage()) == *final(old(init_loan).storage()),
        post(final(field_loan).storage().mem_contents(), *final(init_loan).storage(), result),
)]
unsafe fn read_field_dyn<'a, T, R, E: FieldDecode<'a, T, R>>(
    field: *mut u8,
    init: *mut bool,
    reader: FieldReader<'a, '_, R>,
) -> Result<(), Error> {
    // SAFETY: `init` is valid according to the caller.
    proof_with! { native_init_loan_borrow(Tracked(&mut *init_loan)) }
    let init = unsafe { &mut *init };
    // SAFETY: `field` is valid and points to a valid `MaybeUninit<T>` according
    // to the caller.
    proof_with! { native_loan_borrow_storage(Tracked(&mut *field_loan)) }
    let field = unsafe { &mut *field.cast::<MaybeUninit<T>>() };
    let mut field = if *init {
        // SAFETY: the caller attests that the field is initialized.
        unsafe { InplaceOption::new_init_unchecked(field) }
    } else {
        InplaceOption::uninit(field)
    };
    proof_with! { native_specialized_call(read_u64_field_body) }
    let r = E::read_field(&mut field, reader);
    proof! { field.storage_observation(); }
    *init = field.forget();
    proof! { field.storage_observation(); }
    proof_with! { native_drop(field, InplaceOption::finish_cleared) }
    r
}

// The erased callers still owe the two exclusive loans and typed_default.
// Decoder guarantees are conditional; metadata and dispatch are separate proofs.
#[verus_verify]
#[verus_spec(result =>
    with
        Tracked(field_loan): Tracked<&mut NativeMutLoan<'_, T>>,
        Tracked(init_loan): Tracked<&mut NativeInitLoan<'_, bool>>,
        Ghost(post): Ghost<InplacePost<T, Result<(), Error>>>,
    requires
        old(field_loan).ptr() as *mut u8 == field,
        old(init_loan).ptr() == init,
        *old(init_loan).storage()
            ==> old(field_loan).storage().mem_contents().is_init(),
        typed_default::<'a, T, R, E>(old(field_loan).storage().mem_contents(),
            *old(init_loan).storage(), post),
    ensures
        final(field_loan).ptr() == old(field_loan).ptr(),
        final(init_loan).ptr() == old(init_loan).ptr(),
        final(final(field_loan).storage()).mem_contents()
            == final(old(field_loan).storage()).mem_contents(),
        *final(final(init_loan).storage()) == *final(old(init_loan).storage()),
        *final(init_loan).storage()
            ==> final(field_loan).storage().mem_contents().is_init(),
        result is Ok ==> *final(init_loan).storage(),
        post(final(field_loan).storage().mem_contents(),
            *final(init_loan).storage(), result),
)]
unsafe fn default_field_dyn<'a, T, R, E: FieldDecode<'a, T, R>>(
    field: *mut u8,
    init: *mut bool,
) -> Result<(), Error> {
    // SAFETY: `init` is valid according to the caller.
    proof_with! { native_init_loan_borrow(Tracked(&mut *init_loan)) }
    let init = unsafe { &mut *init };
    // SAFETY: `field` is valid and points to a valid `MaybeUninit<T>` according
    // to the caller.
    proof_with! { native_loan_borrow_storage(Tracked(&mut *field_loan)) }
    let field = unsafe { &mut *field.cast::<MaybeUninit<T>>() };
    let mut field = if *init {
        // SAFETY: the caller attests that the field is initialized.
        unsafe { InplaceOption::new_init_unchecked(field) }
    } else {
        InplaceOption::uninit(field)
    };
    let r = E::default_field(&mut field);
    proof! { field.storage_observation(); }
    *init = field.forget();
    proof! { field.storage_observation(); }
    proof_with! { native_drop(field, InplaceOption::finish_cleared) }
    r
}

unsafe fn drop_field_dyn<T>(field: *mut u8) {
    let field = field.cast::<T>();
    // SAFETY: `field` is valid and points to a valid `T` according to the
    // caller.
    unsafe { field.drop_in_place() }
}

#[cfg(verus_keep_ghost)]
include!("decode.proof.rs");
