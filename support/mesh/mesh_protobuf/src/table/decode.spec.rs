// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

verus! {

/// Successful saved-time decoding through the singleton scalar table.
pub open spec fn saved_time_table(
    numbers: Seq<u32>, decoders: Seq<ErasedDecoderEntry>, offsets: Seq<usize>,
    bytes: Seq<u8>,
) -> bool {
    numbers.len() == 1 && numbers[0] == 1 && decoders.len() == 1
    && offsets.len() == 1 && offsets[0] == 0
    && is_u64_read_entry(decoders[0])
    && crate::protobuf::saved_time_values(bytes) is Some
}

/// Empty messages preserve initialized contents; present fields overwrite.
pub open spec fn saved_time_result(
    bytes: Seq<u8>, before: vstd::raw_ptr::MemContents<u64>, initialized: bool,
) -> vstd::raw_ptr::MemContents<u64> {
    let values = crate::protobuf::saved_time_values(bytes)->Some_0;
    if values.len() > 0 {
        vstd::raw_ptr::MemContents::Init(values.last())
    } else if initialized {
        before
    } else {
        vstd::raw_ptr::MemContents::Init(0)
    }
}

/// The scalar read entry, including its concrete sideband resource type.
pub open spec fn is_u64_read_entry(entry: ErasedDecoderEntry) -> bool {
    native_const_matches(entry, const {
        <crate::encoding::VarintField as FieldDecode<'static, u64, crate::NoResources>>::ENTRY.erase()
    })
}

/// The concrete scalar entry covered by the empty singleton reader.
pub open spec fn is_u64_default_entry(entry: ErasedDecoderEntry) -> bool {
    native_const_matches(entry, const {
        <crate::encoding::VarintField as FieldDecode<'static, u64, ()>>::ENTRY.erase()
    })
}

/// Entries whose actual default adapters are checked by the scalar reader.
pub open spec fn is_supported_u64_default_entry(entry: ErasedDecoderEntry) -> bool {
    is_u64_default_entry(entry)
    || native_const_matches(entry, const {
        <crate::encoding::VarintField as FieldDecode<'static, u64, crate::NoResources>>::ENTRY.erase()
    })
}

/// An exclusive callback transition, conditional on the state actually passed
/// by run_inplace. This is an obligation for the callback, not a decoder axiom.
#[verifier::prophetic]
pub open spec fn inplace_callback<T, R, F>(
    f: F,
    before: vstd::raw_ptr::MemContents<T>,
    initialized: bool,
    post: spec_fn(vstd::raw_ptr::MemContents<T>, bool, R) -> bool,
) -> bool
    where F: FnOnce(*mut u8, &mut bool, Tracked<&mut NativeMutLoan<'_, T>>) -> R,
{
    (forall|base: *mut u8, flag: &mut bool, loan: &mut NativeMutLoan<'_, T>|
        loan.ptr() as *mut u8 == base
        && loan.storage().mem_contents() == before
        && *flag == initialized
        ==> #[trigger] call_requires(f, (base, flag, Tracked(loan))))
    && (forall|base: *mut u8, flag: &mut bool, loan: &mut NativeMutLoan<'_, T>, r: R|
        #[trigger] call_ensures(f, (base, flag, Tracked(loan)), r)
            && loan.ptr() as *mut u8 == base
            && loan.storage().mem_contents() == before
            && *flag == initialized
        ==> final(loan).ptr() == loan.ptr()
            && final(final(loan).storage()).mem_contents()
                == final(loan.storage()).mem_contents()
            && (*final(flag) ==> final(loan).storage().mem_contents().is_init())
            && post(final(loan).storage().mem_contents(), *final(flag), r))
}
}
