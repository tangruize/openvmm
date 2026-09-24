// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

verus! {

/// A terminating protobuf varint prefix, including nonminimal encodings.
pub open spec fn varint_prefix(bytes: Seq<u8>, len: int) -> bool {
    &&& 1 <= len <= 10
    &&& len <= bytes.len()
    &&& bytes[len - 1] < 128
    &&& forall|i: int| 0 <= i < len - 1 ==> #[trigger] bytes[i] >= 128
}

/// The input starts with a terminating varint of at most ten bytes.
pub open spec fn has_varint(bytes: Seq<u8>) -> bool {
    exists|len: int| #[trigger] varint_prefix(bytes, len)
}

/// The unique consumed prefix length when `has_varint` holds.
pub open spec fn varint_len(bytes: Seq<u8>) -> int {
    choose|len: int| #[trigger] varint_prefix(bytes, len)
}

/// Low 64 bits of the little-endian seven-bit payload groups.
/// In particular, only bit zero of the tenth payload contributes.
pub open spec fn varint_value(bytes: Seq<u8>, len: int) -> u64
    recommends 0 <= len <= 10, len <= bytes.len(),
    decreases len,
{
    if len <= 0 {
        0
    } else {
        varint_value(bytes, len - 1)
            | ((bytes[len - 1] as u64 & 0x7f) << ((len - 1) * 7) as u32)
    }
}

/// Field one, varint wire type, followed by a terminating payload and any suffix.
pub open spec fn saved_time_field(bytes: Seq<u8>) -> bool {
    bytes.len() > 1 && bytes[0] == 0x08 && has_varint(bytes.skip(1))
}

/// The finite sequence of saved-time values, or None outside this wire domain.
pub open spec fn saved_time_values(bytes: Seq<u8>) -> Option<Seq<u64>>
    decreases bytes.len(),
{
    if bytes.len() == 0 {
        Some(Seq::empty())
    } else if saved_time_field(bytes) {
        let len = varint_len(bytes.skip(1));
        match saved_time_values(bytes.skip(1 + len)) {
            Some(rest) => Some(seq![varint_value(bytes.skip(1), len)] + rest),
            None => None,
        }
    } else {
        None
    }
}

/// A newly decoded saved-time message defaults to zero or takes its last field.
pub open spec fn saved_time_last(bytes: Seq<u8>) -> u64 {
    let values = saved_time_values(bytes)->Some_0;
    if values.len() == 0 { 0 } else { values.last() }
}

/// Payloads admitted by the saved-time wire domain, without canonicality.
pub open spec fn saved_time_payloads(payloads: Seq<Seq<u8>>) -> bool {
    forall|i: int| 0 <= i < payloads.len()
        ==> varint_prefix(#[trigger] payloads[i], payloads[i].len() as int)
}

/// Concatenation of field-one tags and their saved varint payloads.
pub open spec fn saved_time_bytes(payloads: Seq<Seq<u8>>) -> Seq<u8>
    decreases payloads.len(),
{
    if payloads.len() == 0 {
        Seq::empty()
    } else {
        seq![8u8] + payloads[0] + saved_time_bytes(payloads.skip(1))
    }
}

/// Values in wire order, including duplicates and nonminimal encodings.
pub open spec fn saved_time_decoded(payloads: Seq<Seq<u8>>) -> Seq<u64> {
    payloads.map(|i: int, bytes: Seq<u8>| varint_value(bytes, bytes.len() as int))
}

impl<'a, 'b, R> MessageReader<'a, 'b, R> {
    /// Unconsumed message bytes.
    pub open spec fn reader_bytes(&self) -> Seq<u8> {
        self.data@
    }

    /// The reader's remaining sideband resource range.
    pub open spec fn resource_range(&self) -> Range<u32> {
        self.resources
    }

    /// The decode state associated with this reader.
    pub open spec fn decode_state(&self) -> &'b DecodeState<'b, R> {
        self.state
    }
}

} // verus!
