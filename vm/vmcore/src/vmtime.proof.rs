// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

use mesh::payload::MessageDecode;
use mesh::payload::FieldDecode;
use mesh::payload::Error;
use mesh::payload::{DefaultEncoding, NoResources, decode, inplace_none};
use mesh::payload::protobuf::{self, DecodeState, decode_with};
use mesh::payload::encoding::VarintField;
use mesh::payload::inplace::InplaceOption;
use mesh::payload::protobuf::MessageReader;
use mesh::payload::protobuf::{FieldReader, parse_saved_time_prefix, varint_prefix, varint_value};
use mesh::payload::protobuf::{
    saved_time_payloads, saved_time_bytes, saved_time_decoded, saved_time_sequence_coverage,
    saved_time_values, saved_time_last,
};
use mesh::payload::table::{StructMetadata, TableEncoder};
use mesh::payload::DescribedProtobuf;
use mesh::payload::message::{ProtobufAny, ProtobufMessage, TypeMismatch};
use mesh::payload::protofile::TypeUrl;
use crate::save_restore::SavedStateBlob;
use mesh::payload;
use mesh::payload::table::decode::{
    StructDecodeMetadata, _VERUS_VERIFIED_read_fields, _VERUS_VERIFIED_run_inplace,
    finish_inplace_loan,
    read_fields,
};

type EmptyResources = ();
type EmptyMessageResult = Result<(), Error>;
mesh::payload::specialize_table_read_message!(read_empty_saved_message_body, SavedState, EmptyResources);

type PublicResources = mesh::payload::NoResources;
mesh::payload::specialize_table_read_message!(read_empty_saved_public_message_body, SavedState, PublicResources);
mesh::payload::specialize_table_read_message!(saved_time, read_saved_sequence_message_body, SavedState, PublicResources);
mesh::payload::specialize_decode_with!(decode_saved_with_body, TableEncoder, SavedState, PublicResources);
mesh::payload::specialize_public_decode!(decode_saved_body, SavedState);
mesh::payload::specialize_message_parse!(parse_saved_message_body, SavedState);
mesh::payload::specialize_any_parse!(parse_saved_any_body, SavedState);
crate::save_restore::specialize_blob_parse!(pub parse_saved_blob_body, SavedState);

verus! {
impl VmTimeKeeper {
    /// The local stopped time, without a claim about remote keepers.
    pub closed spec fn local_stopped_time(&self) -> Option<u64> {
        match self.time {
            TimeState::Stopped(time) => Some(time@),
            TimeState::Started(_) => None,
        }
    }
}

proof fn check_local_clock_fields(keeper: &VmTimeKeeper)
    ensures
        (keeper.local_stopped_time() is Some) == (keeper.time is Stopped),
        match keeper.time {
            TimeState::Stopped(VmTime(ticks)) => keeper.local_stopped_time() == Some(ticks),
            TimeState::Started(_) => keeper.local_stopped_time() == None,
        },
{
    #[cfg(verus_keeper_negative_clock_value)]
    assert(keeper.time is Stopped ==> keeper.local_stopped_time() == Some(73));
    #[cfg(verus_keeper_negative_clock_started)]
    assert(keeper.time is Started ==> keeper.local_stopped_time() is Some);
}

fn check_stopped_save(keeper: &VmTimeKeeper, ticks: u64) -> (saved: SavedState)
    requires keeper.time == TimeState::Stopped(VmTime(ticks)),
    ensures saved.saved_time() == ticks,
{
    keeper.save()
}

#[cfg(verus_keeper_negative_save_started)]
fn check_started_save(keeper: &VmTimeKeeper)
    requires keeper.time is Started,
{
    let _ = keeper.save();
}

impl SavedState {
    /// The saved local clock in 100ns units.
    pub closed spec fn saved_time(&self) -> u64 {
        self.vmtime@
    }
}

pub closed spec fn saved_type_url() -> TypeUrl<'static> {
    native_const_value(const { <SavedState as DescribedProtobuf>::TYPE_URL })
}

fn parse_empty_saved_blob(blob: &SavedStateBlob) -> (result: Result<SavedState, Error>)
    requires blob@.bytes.len() == 0, saved_type_url().accepts(blob@.type_url),
    ensures result is Ok, result->Ok_0.vmtime@ == 0,
{
    native_specialized_call(blob.parse::<SavedState>(), parse_saved_blob_body)
}

/// Public parsing for every finite sequence of terminating saved-time fields.
fn parse_saved_blob_sequence(
    blob: &SavedStateBlob, Ghost(payloads): Ghost<Seq<Seq<u8>>>,
) -> (result: Result<SavedState, Error>)
    requires
        saved_type_url().accepts(blob@.type_url),
        saved_time_payloads(payloads),
        blob@.bytes == saved_time_bytes(payloads),
    ensures
        result is Ok,
        result->Ok_0.vmtime@ == if payloads.len() == 0 {
            0
        } else { saved_time_decoded(payloads).last() },
{
    proof { saved_time_sequence_coverage(payloads); }
    let result = native_specialized_call(blob.parse::<SavedState>(), parse_saved_blob_body);
    #[cfg(verus_blob_sequence_negative_first)]
    assert(payloads.len() > 0 ==> result->Ok_0.vmtime@ == saved_time_decoded(payloads)[0]);
    #[cfg(verus_blob_sequence_negative_default)]
    assert(result->Ok_0.vmtime@ == 0);
    result
}

#[cfg(verus_blob_sequence_negative_domain)]
fn check_saved_sequence_requires_domain(blob: &SavedStateBlob)
    requires saved_type_url().accepts(blob@.type_url),
{
    native_specialized_call(blob.parse::<SavedState>(), parse_saved_blob_body);
}

#[cfg(any(verus_blob_sequence_negative_binding, verus_blob_sequence_negative_resource))]
#[allow(unsafe_code)]
fn check_saved_sequence_imported_binding(
    numbers: &[u32],
    decoders: &[mesh::payload::table::decode::ErasedDecoderEntry],
    offsets: &[usize],
    base: *mut u8,
    initialized: &mut bool,
    reader: MessageReader<'_, '_, ()>,
    Tracked(field): Tracked<&mut NativeMutLoan<'_, u64>>,
) {
    #[cfg(verus_blob_sequence_negative_binding)]
    unsafe {
        native_specialized_call_with(
            read_fields(numbers, decoders, offsets, base, initialized, reader),
            mesh::payload::table::decode::read_saved_time_struct, (Tracked(field),));
    }
    #[cfg(verus_blob_sequence_negative_resource)]
    unsafe {
        native_specialized_call_with(
            read_fields(numbers, decoders, offsets, base, initialized, reader),
            mesh::payload::table::decode::_VERUS_VERIFIED_saved_read_fields, (Tracked(field),));
    }
}

fn check_saved_blob_canonical(blob: &SavedStateBlob)
    requires
        blob@.bytes.len() == 0,
        blob@.type_url == "type.googleapis.com/vmtime.SavedState"@,
{
    proof! {
        reveal_strlit("SavedState");
        reveal_strlit("vmtime");
        reveal_strlit("type.googleapis.com/");
        reveal_strlit("type.googleapis.com/vmtime.SavedState");
        mesh::payload::protofile::lemma_type_url_spellings(&saved_type_url());
        assert("type.googleapis.com/"@ + "vmtime"@ + seq!['.'] + "SavedState"@
            =~= "type.googleapis.com/vmtime.SavedState"@);
    }
    let saved = parse_empty_saved_blob(blob).expect("accepted empty saved state");
    assert(saved.vmtime@ == 0);
}

fn check_saved_blob_https(blob: &SavedStateBlob)
    requires
        blob@.bytes.len() == 0,
        blob@.type_url == "https://type.googleapis.com/vmtime.SavedState"@,
{
    proof! {
        reveal_strlit("SavedState");
        reveal_strlit("vmtime");
        reveal_strlit("https://type.googleapis.com/");
        reveal_strlit("https://type.googleapis.com/vmtime.SavedState");
        mesh::payload::protofile::lemma_type_url_spellings(&saved_type_url());
        assert("https://type.googleapis.com/"@ + "vmtime"@ + seq!['.'] + "SavedState"@
            =~= "https://type.googleapis.com/vmtime.SavedState"@);
    }
    let saved = parse_empty_saved_blob(blob).expect("accepted empty saved state");
    assert(saved.vmtime@ == 0);
}

#[cfg(verus_parser_negative_time)]
fn check_saved_blob_wrong_time(blob: &SavedStateBlob)
    requires blob@.bytes.len() == 0, saved_type_url().accepts(blob@.type_url),
{
    let saved = parse_empty_saved_blob(blob).expect("accepted empty saved state");
    assert(saved.vmtime@ == 73);
}

#[cfg(verus_parser_negative_payload)]
fn check_saved_blob_requires_payload(blob: &SavedStateBlob)
    requires saved_type_url().accepts(blob@.type_url),
{
    let _ = parse_empty_saved_blob(blob);
}

#[cfg(verus_parser_negative_url)]
fn check_saved_blob_requires_type(blob: &SavedStateBlob)
    requires blob@.bytes.len() == 0,
{
    let _ = parse_empty_saved_blob(blob);
}

#[cfg(verus_parser_negative_source)]
fn check_saved_blob_wrong_source(blob: &SavedStateBlob)
    requires blob@.bytes.len() == 0, saved_type_url().accepts(blob@.type_url),
{
    native_specialized_call(blob.0.parse::<SavedState>(), parse_saved_blob_body);
}

#[cfg(verus_parser_negative_type)]
fn check_saved_blob_wrong_type(blob: &SavedStateBlob)
    requires blob@.bytes.len() == 0, saved_type_url().accepts(blob@.type_url),
{
    native_specialized_call(
        blob.parse::<crate::save_restore::NoSavedState>(), parse_saved_blob_body);
}

fn decode_empty_saved(data: &[u8]) -> (result: Result<SavedState, Error>)
    requires data@.len() == 0,
    ensures result is Ok, result->Ok_0.vmtime@ == 0,
{
    native_specialized_call(mesh::payload::decode::<SavedState>(data), decode_saved_body)
}

#[cfg(verus_public_decode_negative_time)]
fn check_public_decode_wrong_time(data: &[u8])
    requires data@.len() == 0,
{
    let saved = decode_empty_saved(data).expect("decoded");
    assert(saved.vmtime@ == 73);
}

#[cfg(verus_public_decode_negative_input)]
fn check_public_decode_requires_empty(data: &[u8]) {
    let _ = decode_empty_saved(data);
}

#[cfg(verus_public_decode_negative_nonempty)]
fn check_public_decode_nonempty() {
    let data = [8u8, 73u8];
    let _ = decode_empty_saved(&data);
}

#[cfg(verus_public_decode_negative_source)]
fn check_public_decode_wrong_source(data: &[u8])
    requires data@.len() == 0,
{
    native_specialized_call(
        mesh::payload::merge(SavedState { vmtime: VmTime(73) }, data),
        decode_saved_body);
}

fn read_empty_saved_public_message(
    owner: &mut InplaceOption<'_, SavedState>,
    reader: MessageReader<'_, '_, PublicResources>,
) -> (result: Result<(), mesh::payload::Error>)
    requires old(owner).storage_valid(), reader.reader_bytes().len() == 0,
    ensures
        result is Ok,
        final(owner).storage_valid(),
        final(owner).initialized(),
        final(owner).contents() == if old(owner).initialized() {
            old(owner).contents()
        } else { vstd::raw_ptr::MemContents::Init(SavedState { vmtime: VmTime(0) }) },
        final(final(owner).storage()).mem_contents()
            == final(old(owner).storage()).mem_contents(),
{
    native_specialized_call(
        <TableEncoder as MessageDecode<'_, SavedState, PublicResources>>::read_message(owner, reader),
        read_empty_saved_public_message_body)
}

#[cfg(verus_public_message_negative_time)]
fn check_public_message_wrong_time(reader: MessageReader<'_, '_, PublicResources>)
    requires reader.reader_bytes().len() == 0,
{
    let mut storage = core::mem::MaybeUninit::<SavedState>::uninit();
    let mut owner = InplaceOption::uninit(&mut storage);
    let result = read_empty_saved_public_message(&mut owner, reader);
    assert(result is Ok);
    let saved = owner.take().expect("constructed");
    assert(saved.vmtime@ == 73);
}

#[cfg(verus_public_message_negative_reader)]
fn check_public_message_requires_empty(
    owner: &mut InplaceOption<'_, SavedState>,
    reader: MessageReader<'_, '_, PublicResources>,
)
    requires old(owner).storage_valid(),
{
    let _ = read_empty_saved_public_message(owner, reader);
}

#[cfg(verus_public_message_negative_source)]
fn check_public_message_wrong_source(
    owner: &mut InplaceOption<'_, SavedState>,
    reader: MessageReader<'_, '_, PublicResources>,
)
    requires old(owner).storage_valid(), reader.reader_bytes().len() == 0,
{
    native_specialized_call(
        <TableEncoder as MessageDecode<'_, SavedState, PublicResources>>::read_message(owner, reader),
        read_empty_saved_message_body);
}

fn read_empty_saved_message(
    owner: &mut InplaceOption<'_, SavedState>,
    reader: MessageReader<'_, '_, ()>,
) -> (result: Result<(), mesh::payload::Error>)
    requires old(owner).storage_valid(), reader.reader_bytes().len() == 0,
    ensures
        result is Ok,
        final(owner).storage_valid(),
        final(owner).initialized(),
        final(owner).contents() == if old(owner).initialized() {
            old(owner).contents()
        } else { vstd::raw_ptr::MemContents::Init(SavedState { vmtime: VmTime(0) }) },
        final(final(owner).storage()).mem_contents()
            == final(old(owner).storage()).mem_contents(),
{
    native_specialized_call(
        <TableEncoder as MessageDecode<'_, SavedState, ()>>::read_message(owner, reader),
        read_empty_saved_message_body)
}

fn check_message_uninitialized(reader: MessageReader<'_, '_, ()>)
    requires reader.reader_bytes().len() == 0,
{
    let mut storage = core::mem::MaybeUninit::<SavedState>::uninit();
    let mut owner = InplaceOption::uninit(&mut storage);
    let result = read_empty_saved_message(&mut owner, reader);
    assert(result is Ok);
    let saved = owner.take();
    assert(saved is Some);
    assert(saved->Some_0.vmtime@ == 0);
    #[cfg(verus_message_negative_second_take)]
    {
        let again = owner.take();
        assert(again is Some);
    }
}

fn check_message_retained_nonzero(reader: MessageReader<'_, '_, ()>)
    requires reader.reader_bytes().len() == 0,
{
    let mut storage = core::mem::MaybeUninit::new(SavedState { vmtime: VmTime(73) });
    let mut owner = InplaceOption::uninit(&mut storage);
    let result = read_empty_saved_message(&mut owner, reader);
    assert(result is Ok);
    let saved = owner.take();
    assert(saved is Some);
    assert(saved->Some_0.vmtime@ == 0);
    #[cfg(verus_message_negative_stale)]
    assert(saved->Some_0.vmtime@ == 73);
}

#[allow(unsafe_code)]
fn check_message_initialized_nonzero(reader: MessageReader<'_, '_, ()>)
    requires reader.reader_bytes().len() == 0,
{
    let mut storage = core::mem::MaybeUninit::new(SavedState { vmtime: VmTime(73) });
    let mut owner = unsafe { InplaceOption::new_init_unchecked(&mut storage) };
    let result = read_empty_saved_message(&mut owner, reader);
    assert(result is Ok);
    let saved = owner.take();
    assert(saved is Some);
    assert(saved->Some_0.vmtime@ == 73);
    #[cfg(verus_message_negative_default)]
    assert(saved->Some_0.vmtime@ == 0);
}

#[cfg(verus_message_negative_owner)]
#[allow(unsafe_code)]
fn check_message_invalid_owner(reader: MessageReader<'_, '_, ()>)
    requires reader.reader_bytes().len() == 0,
{
    let mut storage = core::mem::MaybeUninit::<SavedState>::uninit();
    let mut owner = unsafe { InplaceOption::new_init_unchecked(&mut storage) };
    let _ = read_empty_saved_message(&mut owner, reader);
}

#[cfg(verus_message_negative_reader)]
fn check_message_requires_empty(owner: &mut InplaceOption<'_, SavedState>, reader: MessageReader<'_, '_, ()>)
    requires old(owner).storage_valid(),
{
    let _ = read_empty_saved_message(owner, reader);
}
}

verus! {

/// Decode the actual finite saved-time message into the caller's whole object.
fn read_vmtime_sequence(
    storage: &mut core::mem::MaybeUninit<SavedState>,
    initialized: &mut bool,
    reader: MessageReader<'_, '_, NoResources>,
    Ghost(payloads): Ghost<Seq<Seq<u8>>>,
) -> (result: Result<(), Error>)
    requires
        *old(initialized) ==> old(storage).mem_contents().is_init(),
        saved_time_payloads(payloads),
        reader.reader_bytes() == saved_time_bytes(payloads),
    ensures
        result is Ok, *final(initialized),
        final(storage).mem_contents() == if payloads.len() > 0 {
            vstd::raw_ptr::MemContents::Init(
                SavedState { vmtime: VmTime(saved_time_decoded(payloads).last()) })
        } else if *old(initialized) {
            old(storage).mem_contents()
        } else {
            vstd::raw_ptr::MemContents::Init(SavedState { vmtime: VmTime(0) })
        },
{
    proof { saved_time_sequence_coverage(payloads); }
    native_assert_const_eq(
        const { <SavedState as StructMetadata>::NUMBERS[0] },
        const { if cfg!(verus_table_negative_number) { 2u32 } else { 1u32 } });
    let numbers = native_singleton_slice_bound(
        const { <SavedState as StructMetadata>::NUMBERS },
        const { const { <SavedState as StructMetadata>::NUMBERS }.len() },
        const { (const { <SavedState as StructMetadata>::NUMBERS })[0] },
        const { 1u32 });
    let offsets = native_singleton_slice_bound(
        const { <SavedState as StructMetadata>::OFFSETS },
        const { const { <SavedState as StructMetadata>::OFFSETS }.len() },
        const { (const { <SavedState as StructMetadata>::OFFSETS })[0] },
        const { 0usize });
    let decoders = native_singleton_slice_bound(
        const { <SavedState as StructDecodeMetadata<'static, NoResources>>::DECODERS },
        const { const { <SavedState as StructDecodeMetadata<'static, NoResources>>::DECODERS }.len() },
        const { (const { <SavedState as StructDecodeMetadata<'static, NoResources>>::DECODERS })[0] },
        const { <VarintField as FieldDecode<'static, u64, NoResources>>::ENTRY.erase() });
    assert(numbers@ =~= seq![1u32]);
    assert(offsets@ =~= seq![0usize]);
    let (base, Tracked(mut whole)) = native_mut_loan(storage.as_mut_ptr(), storage);
    let Tracked(mut time) = native_single_field_loan::<SavedState, VmTime>(
        Tracked(&mut whole), "vmtime", const { core::mem::offset_of!(SavedState, vmtime) });
    let Tracked(mut field) = native_single_field_loan::<VmTime, u64>(
        Tracked(&mut time), "0",
        const { <VmTime as mesh::payload::transparent::Transparent>::OFFSET });
    #[cfg(verus_table_negative_pointer)]
    let base = core::ptr::null_mut::<SavedState>();
    let result = mesh::payload::table::decode::read_saved_time_struct(
        numbers, decoders, offsets, base.cast::<u8>(), initialized, reader, Tracked(&mut field));
    proof {
        finish_saved_state_loan(field);
        finish_saved_state_loan(time);
        finish_saved_state_loan(whole);
    }
    result
}

/// The nonempty original-storage recovery witness, for every valid initial state.
#[allow(unsafe_code)]
fn recover_vmtime_sequence(
    storage: &mut core::mem::MaybeUninit<SavedState>,
    initialized: &mut bool,
    reader: MessageReader<'_, '_, NoResources>,
    Ghost(payloads): Ghost<Seq<Seq<u8>>>,
) -> (saved: SavedState)
    requires
        *old(initialized) ==> old(storage).mem_contents().is_init(),
        payloads.len() > 0, saved_time_payloads(payloads),
        reader.reader_bytes() == saved_time_bytes(payloads),
    ensures
        saved.vmtime@ == saved_time_decoded(payloads).last(),
        !*final(initialized),
{
    read_vmtime_sequence(storage, initialized, reader, Ghost(payloads))
        .expect("saved-time production table");
    assert(*initialized);
    let mut owner = unsafe { InplaceOption::new_init_unchecked(storage) };
    let saved = owner.take().expect("recover original SavedState");
    *initialized = false;
    #[cfg(verus_table_negative_first)]
    assert(saved.vmtime@ == saved_time_decoded(payloads)[0]);
    #[cfg(verus_table_negative_default)]
    assert(saved.vmtime@ == 0);
    #[cfg(verus_table_negative_second_take)]
    {
        let again = owner.take();
        assert(again is Some);
    }
    saved
}

fn check_sequence_uninitialized(
    reader: MessageReader<'_, '_, NoResources>, Ghost(payloads): Ghost<Seq<Seq<u8>>>,
)
    requires
        payloads.len() > 0, saved_time_payloads(payloads),
        reader.reader_bytes() == saved_time_bytes(payloads),
{
    let mut storage = core::mem::MaybeUninit::<SavedState>::uninit();
    let mut initialized = cfg!(verus_table_negative_uninit);
    let saved = recover_vmtime_sequence(&mut storage, &mut initialized, reader, Ghost(payloads));
    assert(saved.vmtime@ == saved_time_decoded(payloads).last());
    assert(!initialized);
}

fn check_sequence_retained(
    reader: MessageReader<'_, '_, NoResources>,
    previous: u64, initialized_before: bool, Ghost(payloads): Ghost<Seq<Seq<u8>>>,
)
    requires
        payloads.len() > 0, saved_time_payloads(payloads),
        reader.reader_bytes() == saved_time_bytes(payloads),
{
    let mut storage = core::mem::MaybeUninit::new(SavedState { vmtime: VmTime(previous) });
    let mut initialized = initialized_before;
    let saved = recover_vmtime_sequence(&mut storage, &mut initialized, reader, Ghost(payloads));
    assert(saved.vmtime@ == saved_time_decoded(payloads).last());
    assert(!initialized);
    #[cfg(verus_table_negative_stale)]
    assert(saved.vmtime@ == previous);
}

/// A present field overwrites the same original whole object, regardless of
/// its prior contents or initialization flag.
fn read_present_vmtime(
    storage: &mut core::mem::MaybeUninit<SavedState>,
    initialized: &mut bool,
    reader: FieldReader<'_, '_, NoResources>,
) -> (result: Result<(), Error>)
    requires
        *old(initialized) ==> old(storage).mem_contents().is_init(),
        reader.varint_payload() is Some,
    ensures
        result is Ok,
        *final(initialized),
        final(storage).mem_contents() == vstd::raw_ptr::MemContents::Init(
            SavedState { vmtime: VmTime(reader.varint_payload()->Some_0) }),
{
    native_assert_const_eq(
        const { <SavedState as StructMetadata>::NUMBERS[0] },
        const { 1u32 });
    native_assert_const_eq(
        const { <SavedState as StructMetadata>::OFFSETS[0] },
        const { core::mem::offset_of!(SavedState, vmtime) });
    native_assert_const_eq(
        const { <SavedState as StructDecodeMetadata<'static, NoResources>>::DECODERS[0] },
        const { <<VmTime as DefaultEncoding>::Encoding as FieldDecode<'static, VmTime, NoResources>>::ENTRY.erase() });
    let decoders = native_singleton_slice_bound(
        const { <SavedState as StructDecodeMetadata<'static, NoResources>>::DECODERS },
        const { const { <SavedState as StructDecodeMetadata<'static, NoResources>>::DECODERS }.len() },
        const { (const { <SavedState as StructDecodeMetadata<'static, NoResources>>::DECODERS })[0] },
        const { <VarintField as FieldDecode<'static, u64, NoResources>>::ENTRY.erase() });
    let (base, Tracked(mut whole)) = native_mut_loan(storage.as_mut_ptr(), storage);
    let Tracked(mut time) = native_single_field_loan::<SavedState, VmTime>(
        Tracked(&mut whole), "vmtime", const { core::mem::offset_of!(SavedState, vmtime) });
    let Tracked(mut field) = native_single_field_loan::<VmTime, u64>(
        Tracked(&mut time), "0",
        const { <VmTime as mesh::payload::transparent::Transparent>::OFFSET });
    let result = mesh::payload::table::decode::read_u64_erased_field(
        &decoders[0], base.cast::<u8>(), initialized, reader, Tracked(&mut field));
    proof {
        finish_saved_state_loan(field);
        finish_saved_state_loan(time);
        finish_saved_state_loan(whole);
    }
    result
}

/// Compose the real parser with the erased field read, without constructing a
/// replacement field reader or replacement saved-state storage.
fn parse_present_vmtime(
    storage: &mut core::mem::MaybeUninit<SavedState>,
    initialized: &mut bool,
    reader: &mut MessageReader<'_, '_, NoResources>,
    Ghost(prefix): Ghost<Seq<u8>>,
    Ghost(suffix): Ghost<Seq<u8>>,
) -> (result: Result<(), Error>)
    requires
        *old(initialized) ==> old(storage).mem_contents().is_init(),
        varint_prefix(prefix, prefix.len() as int),
        old(reader).reader_bytes() == seq![8u8] + prefix + suffix,
    ensures
        result is Ok,
        *final(initialized),
        final(storage).mem_contents() == vstd::raw_ptr::MemContents::Init(
            SavedState { vmtime: VmTime(varint_value(prefix, prefix.len() as int)) }),
        final(reader).reader_bytes() == suffix,
        final(reader).resource_range() == old(reader).resource_range(),
        final(reader).decode_state() == old(reader).decode_state(),
{
    let (number, field) = parse_saved_time_prefix(reader, Ghost(prefix), Ghost(suffix))?;
    assert(number == 1);
    assert(field.decode_state() == old(reader).decode_state());
    read_present_vmtime(storage, initialized, field)
}

#[allow(unsafe_code)]
fn check_present_uninitialized(
    reader: FieldReader<'_, '_, NoResources>,
    Ghost(value): Ghost<u64>,
)
    requires reader.varint_payload() == Some(value),
{
    let mut storage = core::mem::MaybeUninit::<SavedState>::uninit();
    let mut initialized = false;
    let result = read_present_vmtime(&mut storage, &mut initialized, reader);
    assert(result is Ok);
    assert(initialized);
    let mut owner = unsafe { InplaceOption::new_init_unchecked(&mut storage) };
    let saved = owner.take().expect("read initialized the original storage");
    assert(saved.vmtime@ == value);
}

#[allow(unsafe_code)]
fn check_present_retained(
    reader: FieldReader<'_, '_, NoResources>,
    previous: u64,
    initialized_before: bool,
    Ghost(value): Ghost<u64>,
)
    requires reader.varint_payload() == Some(value),
{
    let mut storage = core::mem::MaybeUninit::new(SavedState { vmtime: VmTime(previous) });
    let mut initialized = initialized_before;
    let result = read_present_vmtime(&mut storage, &mut initialized, reader);
    assert(result is Ok);
    assert(initialized);
    let mut owner = unsafe { InplaceOption::new_init_unchecked(&mut storage) };
    let saved = owner.take().expect("read initialized the original storage");
    assert(saved.vmtime@ == value);
    #[cfg(verus_read_negative_stale)]
    assert(saved.vmtime@ == previous);
    #[cfg(verus_read_negative_default)]
    assert(saved.vmtime@ == 0);
}

/// All terminating payload lengths and suffixes, and all valid initial states.
#[allow(unsafe_code)]
fn check_parsed_storage_recovery(
    storage: &mut core::mem::MaybeUninit<SavedState>,
    initialized: &mut bool,
    reader: &mut MessageReader<'_, '_, NoResources>,
    Ghost(prefix): Ghost<Seq<u8>>,
    Ghost(suffix): Ghost<Seq<u8>>,
) -> (saved: SavedState)
    requires
        *old(initialized) ==> old(storage).mem_contents().is_init(),
        varint_prefix(prefix, prefix.len() as int),
        old(reader).reader_bytes() == seq![8u8] + prefix + suffix,
    ensures
        saved.vmtime@ == varint_value(prefix, prefix.len() as int),
        final(reader).reader_bytes() == suffix,
        final(reader).resource_range() == old(reader).resource_range(),
        final(reader).decode_state() == old(reader).decode_state(),
        !*final(initialized),
{
    parse_present_vmtime(storage, initialized, reader, Ghost(prefix), Ghost(suffix))
        .expect("present saved-time field");
    assert(*initialized);
    let mut owner = unsafe { InplaceOption::new_init_unchecked(storage) };
    let saved = owner.take().expect("recover original saved state");
    *initialized = false;
    #[cfg(verus_read_negative_suffix)]
    assert(reader.reader_bytes() == suffix.push(0));
    saved
}

#[cfg(verus_read_negative_uninit)]
fn negative_present_invalid_initialization(reader: FieldReader<'_, '_, NoResources>)
    requires reader.varint_payload() is Some,
{
    let mut storage = core::mem::MaybeUninit::<SavedState>::uninit();
    let mut initialized = true;
    read_present_vmtime(&mut storage, &mut initialized, reader);
}

#[cfg(verus_read_negative_missing)]
fn negative_present_requires_varint(
    storage: &mut core::mem::MaybeUninit<SavedState>,
    reader: FieldReader<'_, '_, NoResources>,
) {
    let mut initialized = false;
    read_present_vmtime(storage, &mut initialized, reader);
}

impl View for VmTime {
    type V = u64;

    closed spec fn view(&self) -> u64 {
        self.0
    }
}

proof fn lemma_wrapping_add_100ns(time: u64, nanos: u128)
    ensures
        vstd::wrapping::u128_specs::wrapping_add(time as u128, nanos / 100) as u64
            == ((time as nat + nanos as nat / 100)
                % 0x1_0000_0000_0000_0000) as u64,
{
    assert(time as int + nanos / 100 <= u128::MAX);
    let sum = (time as nat + nanos as nat / 100) as u128;
    assert(sum as u64 == (sum % 0x1_0000_0000_0000_0000u128) as u64)
        by (bit_vector);
}

} // verus!

// These check the real derive output, not a second Transparent implementation.
const _: Option<<VmTime as mesh::payload::transparent::Transparent>::Inner> = None::<u64>;
const _: Option<<VmTime as mesh::payload::DefaultEncoding>::Encoding> =
    None::<mesh::payload::transparent::TransparentEncoding<mesh::payload::encoding::VarintField>>;
const _: [(); <VmTime as mesh::payload::transparent::Transparent>::OFFSET] = [];

verus! {

// All operands come from the real derive output. This is metadata identity,
// not a field loan or a proof of read_fields_inner.
fn check_saved_state_reader_metadata() {
    native_assert_const_eq(
        const { <SavedState as mesh::payload::table::StructMetadata>::NUMBERS.len() },
        const { 1usize },
    );
    native_assert_const_eq(
        const { <SavedState as mesh::payload::table::StructMetadata>::NUMBERS[0] },
        const { if cfg!(verus_reader_negative_number) { 2u32 } else { 1u32 } },
    );
    native_assert_const_eq(
        const { <SavedState as mesh::payload::table::StructMetadata>::OFFSETS.len() },
        const { 1usize },
    );
    native_assert_const_eq(
        const { <SavedState as mesh::payload::table::StructMetadata>::OFFSETS[0] },
        const { core::mem::offset_of!(SavedState, vmtime) },
    );
    native_assert_const_eq(
        const { <SavedState as mesh::payload::table::StructMetadata>::OFFSETS[0] },
        const { if cfg!(verus_reader_negative_offset) { 1usize } else { 0usize } },
    );
    native_assert_const_eq(
        const { <SavedState as mesh::payload::table::decode::StructDecodeMetadata<'static, ()>>::DECODERS.len() },
        const { 1usize },
    );
    native_assert_const_eq(
        const { <SavedState as mesh::payload::table::decode::StructDecodeMetadata<'static, ()>>::DECODERS[0] },
        const { <<VmTime as mesh::payload::DefaultEncoding>::Encoding
            as mesh::payload::FieldDecode<'static, VmTime, ()>>::ENTRY.erase() },
    );
    native_assert_const_eq(
        const { <SavedState as mesh::payload::table::decode::StructDecodeMetadata<'static, ()>>::DECODERS[0] },
        const {
            if cfg!(verus_reader_negative_decoder) {
                <mesh::payload::encoding::VarintField
                    as mesh::payload::FieldDecode<'static, u32, ()>>::ENTRY.erase()
            } else {
                <mesh::payload::encoding::VarintField
                    as mesh::payload::FieldDecode<'static, u64, ()>>::ENTRY.erase()
            }
        },
    );
}

fn read_empty_vmtime(
    storage: &mut core::mem::MaybeUninit<SavedState>,
    struct_initialized: &mut bool,
    reader: mesh::payload::protobuf::MessageReader<'_, '_, ()>,
) -> (result: Result<(), mesh::payload::Error>)
    requires
        *old(struct_initialized) ==> old(storage).mem_contents().is_init(),
        reader.reader_bytes().len() == 0,
    ensures
        result is Ok,
        final(storage).mem_contents() == if *old(struct_initialized) {
            old(storage).mem_contents()
        } else { vstd::raw_ptr::MemContents::Init(SavedState { vmtime: VmTime(0) }) },
        *final(struct_initialized),
{
    check_saved_state_reader_metadata();
    let numbers = native_singleton_slice(
        const { <SavedState as mesh::payload::table::StructMetadata>::NUMBERS },
        const { const { <SavedState as mesh::payload::table::StructMetadata>::NUMBERS }.len() });
    let offsets = native_singleton_slice_bound(
        const { <SavedState as mesh::payload::table::StructMetadata>::OFFSETS },
        const { const { <SavedState as mesh::payload::table::StructMetadata>::OFFSETS }.len() },
        const { (const { <SavedState as mesh::payload::table::StructMetadata>::OFFSETS })[0] },
        const { 0usize });
    let decoders = native_singleton_slice_bound(
        const { <SavedState as mesh::payload::table::decode::StructDecodeMetadata<'static, ()>>::DECODERS },
        const { const { <SavedState as mesh::payload::table::decode::StructDecodeMetadata<'static, ()>>::DECODERS }.len() },
        const { (const { <SavedState as mesh::payload::table::decode::StructDecodeMetadata<'static, ()>>::DECODERS })[0] },
        const { <mesh::payload::encoding::VarintField as mesh::payload::FieldDecode<'static, u64, ()>>::ENTRY.erase() });
    proof { mesh::payload::table::decode::u64_default_entry_value(decoders[0]); }
    let (base, Tracked(mut whole)) = native_mut_loan(storage.as_mut_ptr(), storage);
    let Tracked(mut time) = native_single_field_loan::<SavedState, VmTime>(
        Tracked(&mut whole), "vmtime", const { core::mem::offset_of!(SavedState, vmtime) });
    let Tracked(mut field) = native_single_field_loan::<VmTime, u64>(
        Tracked(&mut time), "0",
        const { <VmTime as mesh::payload::transparent::Transparent>::OFFSET });
    let result = mesh::payload::table::decode::read_empty_struct(
        numbers, decoders, offsets, base.cast::<u8>(), struct_initialized, reader,
        Tracked(&mut field));
    proof {
        finish_saved_state_loan(field);
        finish_saved_state_loan(time);
        finish_saved_state_loan(whole);
    }
    result
}

proof fn finish_saved_state_loan<T>(tracked loan: NativeMutLoan<'_, T>)
    ensures final(loan.storage()).mem_contents() == loan.storage().mem_contents(),
{
}

#[allow(unsafe_code)]
fn check_reader_preserves_nonzero(
    reader: mesh::payload::protobuf::MessageReader<'_, '_, ()>,
)
    requires reader.reader_bytes().len() == 0,
{
    let mut storage = core::mem::MaybeUninit::new(SavedState { vmtime: VmTime::from_100ns(73) });
    let mut initialized = true;
    let result = read_empty_vmtime(&mut storage, &mut initialized, reader);
    assert(result is Ok);
    let value = unsafe { storage.assume_init_ref() };
    let time = value.vmtime.as_100ns();
    assert(time == 73);
    assert(initialized);
    #[cfg(verus_reader_negative_preservation_zero)]
    assert(time == 0);
}

#[allow(unsafe_code)]
fn check_reader_initializes_uninitialized(
    reader: mesh::payload::protobuf::MessageReader<'_, '_, ()>,
)
    requires reader.reader_bytes().len() == 0,
{
    let mut storage = core::mem::MaybeUninit::<SavedState>::uninit();
    let mut initialized = false;
    let result = read_empty_vmtime(&mut storage, &mut initialized, reader);
    assert(result is Ok);
    let value = unsafe { storage.assume_init_ref() };
    let time = value.vmtime.as_100ns();
    assert(time == 0);
    assert(initialized);
}

#[allow(unsafe_code)]
fn check_reader_defaults_retained_nonzero(
    reader: mesh::payload::protobuf::MessageReader<'_, '_, ()>,
)
    requires reader.reader_bytes().len() == 0,
{
    let mut storage = core::mem::MaybeUninit::new(SavedState { vmtime: VmTime::from_100ns(73) });
    let mut initialized = false;
    let result = read_empty_vmtime(&mut storage, &mut initialized, reader);
    assert(result is Ok);
    let value = unsafe { storage.assume_init_ref() };
    let time = value.vmtime.as_100ns();
    assert(time == 0);
    assert(initialized);
    #[cfg(verus_reader_negative_stale)]
    assert(time == 73);
}

#[cfg(verus_reader_negative_whole_uninit)]
fn negative_reader_whole_uninitialized(
    reader: mesh::payload::protobuf::MessageReader<'_, '_, ()>,
)
    requires reader.reader_bytes().len() == 0,
{
    let mut storage = core::mem::MaybeUninit::<SavedState>::uninit();
    let mut initialized = true;
    read_empty_vmtime(&mut storage, &mut initialized, reader);
}

#[cfg(verus_reader_negative_whole_owner)]
fn negative_reader_whole_owner(
    storage: &mut core::mem::MaybeUninit<SavedState>,
    other: &mut core::mem::MaybeUninit<SavedState>,
) {
    let (_, Tracked(mut whole)) = native_mut_loan(storage.as_mut_ptr(), other);
    let _ = native_single_field_loan::<SavedState, VmTime>(
        Tracked(&mut whole), "vmtime", const { core::mem::offset_of!(SavedState, vmtime) });
}

#[cfg(verus_reader_negative_whole_live)]
#[allow(unsafe_code)]
fn negative_reader_whole_live(storage: &mut core::mem::MaybeUninit<SavedState>) {
    let (base, Tracked(mut whole)) = native_mut_loan(storage.as_mut_ptr(), storage);
    let Tracked(mut time) = native_single_field_loan::<SavedState, VmTime>(
        Tracked(&mut whole), "vmtime", const { core::mem::offset_of!(SavedState, vmtime) });
    let Tracked(mut field) = native_single_field_loan::<VmTime, u64>(
        Tracked(&mut time), "0",
        const { <VmTime as mesh::payload::transparent::Transparent>::OFFSET });
    *storage = core::mem::MaybeUninit::new(SavedState { vmtime: VmTime(73) });
    unsafe { native_loan_write::<u64>(base.cast::<u64>().write(0), Tracked(&mut field)); }
}

// The compiler checks entry identity, including allocation provenance, before
// this zero-offset storage projection enters the concrete erased dispatch.
fn default_vmtime_storage(
    storage: &mut core::mem::MaybeUninit<VmTime>,
    flag: &mut core::mem::MaybeUninit<bool>,
) -> (result: Result<(), mesh::payload::Error>)
    requires
        old(flag).mem_contents().is_init(),
        old(flag).mem_contents().value() ==> old(storage).mem_contents().is_init(),
    ensures
        result is Ok,
        final(storage).mem_contents() == vstd::raw_ptr::MemContents::Init(VmTime(0)),
        final(flag).mem_contents() == vstd::raw_ptr::MemContents::Init(true),
{
    native_assert_const_eq(
        const { <<VmTime as mesh::payload::DefaultEncoding>::Encoding
            as mesh::payload::FieldDecode<'static, VmTime, ()>>::ENTRY.erase() },
        const { <mesh::payload::encoding::VarintField
            as mesh::payload::FieldDecode<'static, u64, ()>>::ENTRY.erase() },
    );
    mesh::payload::table::decode::check_u64_default_metadata();
    let inner = native_single_field_storage::<VmTime, u64>(
        storage, "0",
        const { <VmTime as mesh::payload::transparent::Transparent>::OFFSET },
    );
    mesh::payload::table::decode::default_u64_erased_storage(inner, flag)
}

#[allow(unsafe_code)]
fn check_vmtime_uninitialized() {
    let mut storage = core::mem::MaybeUninit::<VmTime>::uninit();
    let mut flag = core::mem::MaybeUninit::new(false);
    let result = default_vmtime_storage(&mut storage, &mut flag);
    assert(result is Ok);
    let value = unsafe { storage.assume_init_ref() };
    let time = value.as_100ns();
    assert(time == 0);
    assert(flag.mem_contents() == vstd::raw_ptr::MemContents::Init(true));
}

#[allow(unsafe_code)]
fn check_vmtime_initialized_nonzero() {
    let mut storage = core::mem::MaybeUninit::new(VmTime::from_100ns(73));
    let mut flag = core::mem::MaybeUninit::new(true);
    let result = default_vmtime_storage(&mut storage, &mut flag);
    assert(result is Ok);
    let value = unsafe { storage.assume_init_ref() };
    let time = value.as_100ns();
    assert(time == 0);
    assert(flag.mem_contents() == vstd::raw_ptr::MemContents::Init(true));
    #[cfg(verus_vmtime_negative_stale)]
    assert(storage.mem_contents() == vstd::raw_ptr::MemContents::Init(VmTime(73)));
}

#[allow(unsafe_code)]
fn check_vmtime_false_flag_retained() {
    let mut storage = core::mem::MaybeUninit::new(VmTime::from_100ns(73));
    let mut flag = core::mem::MaybeUninit::new(false);
    let result = default_vmtime_storage(&mut storage, &mut flag);
    assert(result is Ok);
    let value = unsafe { storage.assume_init_ref() };
    let time = value.as_100ns();
    assert(time == 0);
    assert(flag.mem_contents() == vstd::raw_ptr::MemContents::Init(true));
}

#[cfg(verus_vmtime_negative_field)]
fn negative_vmtime_field(storage: &mut core::mem::MaybeUninit<VmTime>) {
    let inner = native_single_field_storage::<VmTime, u64>(storage, "1", const { 0 });
}

#[cfg(verus_vmtime_negative_offset)]
fn negative_vmtime_offset(storage: &mut core::mem::MaybeUninit<VmTime>) {
    let inner = native_single_field_storage::<VmTime, u64>(storage, "0", const { 1 });
}

#[cfg(verus_vmtime_negative_entry)]
fn negative_vmtime_entry() {
    native_assert_const_eq(
        const { <<VmTime as mesh::payload::DefaultEncoding>::Encoding
            as mesh::payload::FieldDecode<'static, VmTime, ()>>::ENTRY.erase() },
        const { <mesh::payload::encoding::VarintField
            as mesh::payload::FieldDecode<'static, u32, ()>>::ENTRY.erase() },
    );
}

#[cfg(verus_vmtime_negative_live)]
fn negative_vmtime_live(storage: &mut core::mem::MaybeUninit<VmTime>) {
    let inner = native_single_field_storage::<VmTime, u64>(storage, "0", const { 0 });
    *storage = core::mem::MaybeUninit::new(VmTime(7));
    *inner = core::mem::MaybeUninit::new(9);
}

#[cfg(verus_vmtime_negative_duplicate)]
proof fn consume_vmtime_loan(tracked loan: NativeMutLoan<'_, u64>) {}

#[cfg(verus_vmtime_negative_duplicate)]
fn negative_vmtime_duplicate(storage: &mut core::mem::MaybeUninit<VmTime>) {
    let inner = native_single_field_storage::<VmTime, u64>(storage, "0", const { 0 });
    let (_pointer, Tracked(loan)) = native_mut_loan(inner.as_mut_ptr(), inner);
    let tracked duplicate = loan;
    proof {
        consume_vmtime_loan(loan);
        consume_vmtime_loan(duplicate);
    }
}

} // verus!
