// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

specialize_field_iterator_next!(pub saved_time_iterator_next);

verus! {

impl<'a, 'b, R> FieldReader<'a, 'b, R> {
    /// The actual stored scalar, if this is a varint field.
    pub closed spec fn varint_payload(&self) -> Option<u64> {
        match self.field {
            Value::Varint(value) => Some(value),
            _ => None,
        }
    }

    /// The decode state inherited from the message reader.
    pub closed spec fn decode_state(&self) -> &'b DecodeState<'b, R> {
        self.state
    }
}

fn check_fresh_decode_state(resources: &mut [Option<crate::NoResources>])
    requires old(resources)@.len() == 0,
{
    let _state = native_fresh_reader(DecodeState::new(resources));
}

fn check_fresh_reader_body(data: &[u8], resources: &mut [Option<crate::NoResources>])
    requires old(resources)@.len() == 0,
{
    let state = native_fresh_reader(DecodeState::new(resources));
    let _reader = native_fresh_reader(MessageReader::new(data, &state));
    assert(_reader.reader_bytes() == data@);
    assert(_reader.decode_state() == &state);
    assert(_reader.resource_range().start == 0);
    assert(_reader.resource_range().end == 0);
    #[cfg(verus_constructor_negative_bytes)]
    assert(_reader.reader_bytes().len() == 0);
    #[cfg(verus_constructor_negative_range)]
    assert(_reader.resource_range().end == 1);
}

#[cfg(verus_constructor_negative_state)]
fn check_reader_requires_fresh_state(data: &[u8], state: &DecodeState<'_, crate::NoResources>)
    requires data@.len() == 0,
{
    let _reader = native_fresh_reader(MessageReader::new(data, state));
}

#[cfg(verus_constructor_negative_intervening)]
fn check_reader_rejects_intervening_use(data: &[u8], resources: &mut [Option<crate::NoResources>])
    requires data@.len() == 0, old(resources)@.len() == 0,
{
    let state = native_fresh_reader(DecodeState::new(resources));
    let _ = &state;
    let _reader = native_fresh_reader(MessageReader::new(data, &state));
}

#[cfg(verus_constructor_negative_resources)]
fn check_state_requires_empty_resources(resources: &mut [Option<crate::NoResources>]) {
    let _state = native_fresh_reader(DecodeState::new(resources));
}

proof fn varint_high_bit(byte: u8)
    ensures (byte & 0x80 == 0) == (byte < 128),
{
    assert((byte & 0x80 == 0) == (byte < 128)) by(bit_vector);
}

proof fn varint_key(bytes: Seq<u8>)
    requires bytes.len() > 0, bytes[0] == 8,
    ensures
        has_varint(bytes),
        varint_len(bytes) == 1,
        varint_value(bytes, 1) == 8,
{
    reveal_with_fuel(varint_value, 2);
    assert(varint_prefix(bytes, 1));
    let len = varint_len(bytes);
    assert(varint_prefix(bytes, len));
    if len > 1 {
        assert(bytes[0] >= 128);
    }
    assert((0u64 | ((8u64 & 0x7f) << 0u32)) == 8) by(bit_vector);
    assert(((8u64 & 0x7f) << 0u32) == 8) by(bit_vector);
}

proof fn varint_prefix_unique(bytes: Seq<u8>, left: int, right: int)
    requires varint_prefix(bytes, left), varint_prefix(bytes, right),
    ensures left == right,
{
    if left < right {
        assert(bytes[left - 1] >= 128);
    }
    if right < left {
        assert(bytes[right - 1] >= 128);
    }
}

proof fn varint_value_append(prefix: Seq<u8>, suffix: Seq<u8>, len: int)
    requires 0 <= len <= prefix.len(), len <= 10,
    ensures varint_value(prefix + suffix, len) == varint_value(prefix, len),
    decreases len,
{
    if len > 0 {
        varint_value_append(prefix, suffix, len - 1);
        assert((prefix + suffix)[len - 1] == prefix[len - 1]);
    }
}

proof fn varint_append(prefix: Seq<u8>, suffix: Seq<u8>)
    requires varint_prefix(prefix, prefix.len() as int),
    ensures
        has_varint(prefix + suffix),
        varint_len(prefix + suffix) == prefix.len(),
        varint_value(prefix + suffix, prefix.len() as int)
            == varint_value(prefix, prefix.len() as int),
        (prefix + suffix).skip(prefix.len() as int) == suffix,
{
    let bytes = prefix + suffix;
    let len = prefix.len() as int;
    assert forall|i: int| 0 <= i < len - 1 implies #[trigger] bytes[i] >= 128 by {
        assert(bytes[i] == prefix[i]);
    }
    assert(varint_prefix(bytes, len));
    varint_prefix_unique(bytes, varint_len(bytes), len);
    varint_value_append(prefix, suffix, len);
}

/// Every finite sequence of terminating payloads is in the iterator's domain.
pub proof fn saved_time_sequence_coverage(payloads: Seq<Seq<u8>>)
    requires saved_time_payloads(payloads),
    ensures
        saved_time_values(saved_time_bytes(payloads)) == Some(saved_time_decoded(payloads)),
        (saved_time_bytes(payloads).len() == 0) == (payloads.len() == 0),
    decreases payloads.len(),
{
    if payloads.len() == 0 {
        assert(saved_time_decoded(payloads) =~= Seq::<u64>::empty());
    } else {
        let rest = payloads.skip(1);
        assert forall|i: int| 0 <= i < rest.len()
            implies varint_prefix(#[trigger] rest[i], rest[i].len() as int) by {
            assert(rest[i] == payloads[i + 1]);
        }
        saved_time_sequence_coverage(rest);
        let suffix = saved_time_bytes(rest);
        let prefix = payloads[0];
        varint_append(prefix, suffix);
        let bytes = saved_time_bytes(payloads);
        assert(bytes.skip(1) == prefix + suffix);
        assert(bytes.skip(1 + prefix.len() as int) == suffix);
        assert(saved_time_decoded(payloads)
            =~= seq![varint_value(prefix, prefix.len() as int)] + saved_time_decoded(rest));
    }
}

proof fn saved_time_values_empty(bytes: Seq<u8>)
    requires saved_time_values(bytes) is Some,
    ensures (saved_time_values(bytes)->Some_0.len() == 0) == (bytes.len() == 0),
{
}

/// Source-bound next: no reader or field is reconstructed by this bridge.
pub fn saved_time_iterator_step<'a, 'b, R>(
    iterator: &mut FieldIterator<'a, 'b, R>,
) -> (result: Option<Result<(u32, FieldReader<'a, 'b, R>)>>)
    requires saved_time_values(old(iterator).reader_bytes()) is Some,
    ensures
        final(iterator).resource_range() == old(iterator).resource_range(),
        final(iterator).decode_state() == old(iterator).decode_state(),
        if old(iterator).reader_bytes().len() == 0 {
            result is None && *final(iterator) == *old(iterator)
        } else {
            result is Some && result->Some_0 is Ok
            && result->Some_0->Ok_0.0 == 1
            && result->Some_0->Ok_0.1.varint_payload()
                == Some(saved_time_values(old(iterator).reader_bytes())->Some_0[0])
            && result->Some_0->Ok_0.1.decode_state() == old(iterator).decode_state()
            && final(iterator).reader_bytes() == old(iterator).reader_bytes().skip(
                1 + varint_len(old(iterator).reader_bytes().skip(1)))
            && saved_time_values(final(iterator).reader_bytes())
                == Some(saved_time_values(old(iterator).reader_bytes())->Some_0.skip(1))
            && final(iterator).reader_bytes().len() < old(iterator).reader_bytes().len()
        },
{
    native_specialized_call(iterator.next(), saved_time_iterator_next::<R>)
}

// This witness checks iteration only, not table dispatch or SavedState storage.
fn check_saved_time_iteration<R>(
    reader: MessageReader<'_, '_, R>,
    Ghost(payloads): Ghost<Seq<Seq<u8>>>,
) -> (last: Option<u64>)
    requires
        saved_time_payloads(payloads),
        reader.reader_bytes() == saved_time_bytes(payloads),
    ensures last == if payloads.len() == 0 {
        None
    } else {
        Some(saved_time_decoded(payloads).last())
    },
{
    proof { saved_time_sequence_coverage(payloads); }
    let ghost values = saved_time_decoded(payloads);
    let ghost resources = reader.resource_range();
    let ghost state = reader.decode_state();
    let ghost mut consumed: int = 0;
    let mut iterator = reader.into_iter();
    let mut last = None;
    loop
        invariant
            0 <= consumed <= values.len(),
            values == saved_time_decoded(payloads),
            saved_time_values(iterator.reader_bytes()) == Some(values.skip(consumed)),
            iterator.resource_range() == resources,
            iterator.decode_state() == state,
            last == if consumed == 0 { None } else { Some(values[consumed - 1]) },
        decreases values.len() - consumed,
    {
        let ghost before = iterator.reader_bytes();
        proof { saved_time_values_empty(before); }
        let next = saved_time_iterator_step(&mut iterator);
        if let Some(field) = next {
            let (number, field) = field.expect("valid saved-time iterator domain");
            assert(number == 1);
            assert(field.decode_state() == state);
            last = Some(field.varint().expect("saved-time fields are varints"));
            proof {
                assert(values.skip(consumed).skip(1) == values.skip(consumed + 1));
                consumed = consumed + 1;
            }
        } else {
            assert(iterator.reader_bytes().len() == 0);
            #[cfg(verus_iteration_negative_first)]
            assert(payloads.len() > 1 ==> last == Some(values[0]));
            #[cfg(verus_iteration_negative_default)]
            assert(last == Some(0u64));
            return last;
        }
    }
}

#[cfg(verus_iteration_negative_number)]
fn negative_iterator_number<R>(iterator: &mut FieldIterator<'_, '_, R>)
    requires
        saved_time_values(old(iterator).reader_bytes()) is Some,
        old(iterator).reader_bytes().len() > 0,
{
    let next = saved_time_iterator_step(iterator);
    assert(next->Some_0->Ok_0.0 == 2);
}

#[cfg(verus_iteration_negative_stale)]
fn negative_iterator_stale<R>(iterator: &mut FieldIterator<'_, '_, R>)
    requires
        saved_time_values(old(iterator).reader_bytes()) is Some,
        old(iterator).reader_bytes().len() > 0,
{
    let ghost before = iterator.reader_bytes();
    saved_time_iterator_step(iterator);
    assert(iterator.reader_bytes() == before);
}

#[cfg(verus_iteration_negative_domain)]
fn negative_iterator_domain<R>(iterator: &mut FieldIterator<'_, '_, R>) {
    saved_time_iterator_step(iterator);
}

/// Return the real parsed field without consuming its scalar payload.
pub fn parse_saved_time_prefix<'a, 'b, R>(
    reader: &mut MessageReader<'a, 'b, R>,
    Ghost(prefix): Ghost<Seq<u8>>,
    Ghost(suffix): Ghost<Seq<u8>>,
) -> (result: Result<(u32, FieldReader<'a, 'b, R>)>)
    requires
        varint_prefix(prefix, prefix.len() as int),
        old(reader).reader_bytes() == seq![8u8] + prefix + suffix,
    ensures
        result is Ok,
        result->Ok_0.0 == 1,
        result->Ok_0.1.varint_payload() == Some(varint_value(prefix, prefix.len() as int)),
        result->Ok_0.1.decode_state() == old(reader).decode_state(),
        final(reader).reader_bytes() == suffix,
        final(reader).resource_range() == old(reader).resource_range(),
        final(reader).decode_state() == old(reader).decode_state(),
{
    proof {
        varint_append(prefix, suffix);
        assert(reader.reader_bytes().skip(1) == prefix + suffix);
    }
    reader.parse_field()
}

/// A symbolic production parse/accessor result for any terminating payload and suffix.
pub fn read_saved_time_prefix<R>(
    reader: &mut MessageReader<'_, '_, R>,
    Ghost(prefix): Ghost<Seq<u8>>,
    Ghost(suffix): Ghost<Seq<u8>>,
) -> (result: Result<u64>)
    requires
        varint_prefix(prefix, prefix.len() as int),
        old(reader).reader_bytes() == seq![8u8] + prefix + suffix,
    ensures
        result is Ok,
        result->Ok_0 == varint_value(prefix, prefix.len() as int),
        final(reader).reader_bytes() == suffix,
        final(reader).resource_range() == old(reader).resource_range(),
        final(reader).decode_state() == old(reader).decode_state(),
{
    proof {
        varint_append(prefix, suffix);
        assert(reader.reader_bytes().skip(1) == prefix + suffix);
    }
    read_saved_time_field(reader)
}

proof fn varint_boundaries()
{
    assert(varint_prefix(seq![0u8], 1));
    assert(varint_value(seq![0u8], 1) == 0) by(compute);
    assert(varint_prefix(seq![127u8], 1));
    assert(varint_value(seq![127u8], 1) == 127) by(compute);
    assert(varint_prefix(seq![172u8, 2], 2));
    assert(varint_value(seq![172u8, 2], 2) == 300) by(compute);
    assert(varint_prefix(seq![128u8, 0], 2));
    assert(varint_value(seq![128u8, 0], 2) == 0) by(compute);
    assert(varint_prefix(seq![255u8, 255, 255, 255, 255, 255, 255, 255, 255, 1], 10));
    assert(varint_value(seq![255u8, 255, 255, 255, 255, 255, 255, 255, 255, 1], 10)
        == u64::MAX) by(compute);
    assert(varint_prefix(seq![255u8, 255, 255, 255, 255, 255, 255, 255, 255, 127], 10));
    assert(varint_value(seq![255u8, 255, 255, 255, 255, 255, 255, 255, 255, 127], 10)
        == u64::MAX) by(compute);
    assert(varint_prefix(seq![255u8, 255, 255, 255, 255, 255, 255, 255, 255, 126], 10));
    assert(varint_value(seq![255u8, 255, 255, 255, 255, 255, 255, 255, 255, 126], 10)
        == 0x7fff_ffff_ffff_ffffu64) by(compute);
}

proof fn tenth_payload_truncation(byte: u8)
    ensures ((byte as u64 & 0x7f) << 63u32) == ((byte as u64 & 1) << 63u32),
{
    assert(((byte as u64 & 0x7f) << 63u32) == ((byte as u64 & 1) << 63u32)) by(bit_vector);
}

// A ten-byte witness covers every u64, including values with shorter encodings.
closed spec fn ten_byte_varint(value: u64) -> Seq<u8> {
    Seq::new(10, |i: int| if i < 9 {
        ((value >> (7 * i) as u32) as u8) | 128
    } else {
        (value >> 63u32) as u8
    })
}

proof fn ten_byte_varint_value(value: u64)
    ensures
        varint_prefix(ten_byte_varint(value), 10),
        varint_value(ten_byte_varint(value), 10) == value,
{
    let bytes = ten_byte_varint(value);
    assert forall|i: int| 0 <= i < 9 implies #[trigger] bytes[i] >= 128 by {
        let payload = (value >> (7 * i) as u32) as u8;
        assert((payload | 128u8) >= 128u8) by(bit_vector);
    }
    assert(((value >> 63u32) as u8) < 128u8) by(bit_vector);
    reveal_with_fuel(varint_value, 11);
    assert(
        (0u64
        | ((((value >> 0u32) as u8 | 128) as u64 & 127) << 0u32)
        | ((((value >> 7u32) as u8 | 128) as u64 & 127) << 7u32)
        | ((((value >> 14u32) as u8 | 128) as u64 & 127) << 14u32)
        | ((((value >> 21u32) as u8 | 128) as u64 & 127) << 21u32)
        | ((((value >> 28u32) as u8 | 128) as u64 & 127) << 28u32)
        | ((((value >> 35u32) as u8 | 128) as u64 & 127) << 35u32)
        | ((((value >> 42u32) as u8 | 128) as u64 & 127) << 42u32)
        | ((((value >> 49u32) as u8 | 128) as u64 & 127) << 49u32)
        | ((((value >> 56u32) as u8 | 128) as u64 & 127) << 56u32)
        | ((((value >> 63u32) as u8) as u64 & 127) << 63u32))
        == value
    ) by(bit_vector);
}

fn check_arbitrary_saved_time<R>(
    reader: &mut MessageReader<'_, '_, R>,
    value: u64,
    Ghost(suffix): Ghost<Seq<u8>>,
) -> (result: Result<u64>)
    requires old(reader).reader_bytes() == seq![8u8] + ten_byte_varint(value) + suffix,
    ensures result == Ok(value), final(reader).reader_bytes() == suffix,
{
    proof { ten_byte_varint_value(value); }
    read_saved_time_prefix(reader, Ghost(ten_byte_varint(value)), Ghost(suffix))
}

/// Consume the actual field-1 varint reader and its actual scalar accessor.
pub fn read_saved_time_field<R>(reader: &mut MessageReader<'_, '_, R>) -> (result: Result<u64>)
    requires saved_time_field(old(reader).reader_bytes()),
    ensures
        result is Ok,
        result->Ok_0 == varint_value(
            old(reader).reader_bytes().skip(1),
            varint_len(old(reader).reader_bytes().skip(1))),
        final(reader).reader_bytes() == old(reader).reader_bytes().skip(
            1 + varint_len(old(reader).reader_bytes().skip(1))),
        final(reader).resource_range() == old(reader).resource_range(),
        final(reader).decode_state() == old(reader).decode_state(),
{
    let (number, field) = reader.parse_field()?;
    assert(number == 1);
    assert(field.decode_state() == old(reader).decode_state());
    field.varint()
}

#[cfg(verus_varint_negative_value)]
fn negative_varint_value<R>(reader: &mut MessageReader<'_, '_, R>, Ghost(suffix): Ghost<Seq<u8>>)
    requires old(reader).reader_bytes() == seq![8u8, 1] + suffix,
{
    let ghost prefix = seq![1u8];
    let result = read_saved_time_prefix(reader, Ghost(prefix), Ghost(suffix));
    assert(result->Ok_0 == 2);
}

#[cfg(verus_varint_negative_suffix)]
fn negative_varint_suffix<R>(reader: &mut MessageReader<'_, '_, R>, Ghost(suffix): Ghost<Seq<u8>>)
    requires old(reader).reader_bytes() == seq![8u8, 1] + suffix,
{
    let ghost prefix = seq![1u8];
    read_saved_time_prefix(reader, Ghost(prefix), Ghost(suffix));
    assert(reader.reader_bytes() == suffix.push(0));
}

#[cfg(verus_varint_negative_unterminated)]
fn negative_varint_unterminated<R>(reader: &mut MessageReader<'_, '_, R>)
    requires old(reader).reader_bytes() == seq![8u8, 128],
{
    read_saved_time_field(reader);
}

#[cfg(verus_varint_negative_missing)]
fn negative_varint_missing<R>(reader: &mut MessageReader<'_, '_, R>) {
    read_saved_time_field(reader);
}

#[cfg(verus_varint_negative_overlong)]
fn negative_varint_overlong<R>(reader: &mut MessageReader<'_, '_, R>)
    requires old(reader).reader_bytes() == seq![8u8, 128, 128, 128, 128, 128, 128, 128, 128, 128, 128, 0],
{
    read_saved_time_field(reader);
}

impl<'a, 'b, R> FieldIterator<'a, 'b, R> {
    pub closed spec fn reader_bytes(&self) -> Seq<u8> {
        self.0.reader_bytes()
    }

    pub closed spec fn resource_range(&self) -> Range<u32> {
        self.0.resource_range()
    }

    pub closed spec fn decode_state(&self) -> &'b DecodeState<'b, R> {
        self.0.decode_state()
    }
}

pub fn empty_reader_iteration<'a, 'b, R>(
    reader: MessageReader<'a, 'b, R>,
) -> (result: Option<Result<(u32, FieldReader<'a, 'b, R>)>>)
    requires reader.reader_bytes().len() == 0,
    ensures result is None,
{
    let mut iterator = reader.into_iter();
    empty_iterator_step(&mut iterator)
}

pub fn empty_iterator_step<'a, 'b, R>(
    iterator: &mut FieldIterator<'a, 'b, R>,
) -> (result: Option<Result<(u32, FieldReader<'a, 'b, R>)>>)
    requires old(iterator).reader_bytes().len() == 0,
    ensures
        result is None,
        *final(iterator) == *old(iterator),
{
    native_empty_slice_next(iterator.next())
}

#[cfg(verus_reader_negative_nonempty)]
fn negative_nonempty_reader<R>(reader: MessageReader<'_, '_, R>)
    requires reader.reader_bytes().len() > 0,
{
    let mut iterator = reader.into_iter();
    native_empty_slice_next(iterator.next());
}

#[cfg(verus_reader_negative_missing_premise)]
fn negative_unjustified_empty_reader<R>(reader: MessageReader<'_, '_, R>) {
    empty_reader_iteration(reader);
}

} // verus!
