// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

// Trusted: the encode contract of protobuf messages (`ProtobufMessage`).
//
// TRUSTED, do not attempt to prove anything in this file: Verus uses only
// the contracts and axioms here, and they are claims about protobuf
// encoding, which is external (`SNAPSHOT-SPEC-TRUSTED.md`, Part 1).
//
// Everything in this file is TRUSTED, and listed in
// `SNAPSHOT-SPEC-TRUSTED.md`. Protobuf encoding is not verified. What we
// assume, as for saved-state blobs (`vm/vmcore/src/save_restore_t.rs`):
//
// - `ProtobufMessage::new(value)` gives a message that encodes `value`:
//   `encoded() == ProtobufMessageView::of(value)`.
// - `of::<T>` is injective: encoding loses nothing.
//
// Nothing is assumed across types.

use super::spec::ProtobufMessageAdditionalSpecFns;
use super::spec::ProtobufMessageView;
use mesh::payload::Protobuf;
use mesh::payload::message::ProtobufMessage;
use vstd::prelude::*;

verus! {

/// The encoding, uninterpreted (TRUSTED with the contracts below: protobuf
/// is external).
impl ProtobufMessageView {
    /// The view of a message that encodes `value`, of type `T`.
    pub uninterp spec fn of<T>(value: T) -> ProtobufMessageView;
}

impl ProtobufMessageAdditionalSpecFns for ProtobufMessage {
    /// The value this message encodes (uninterpreted: protobuf).
    uninterp spec fn encoded(&self) -> ProtobufMessageView;
}


/// Trusted: encoding gives a message whose view is `of(data)`.
pub assume_specification<T: Protobuf>[ ProtobufMessage::new ](data: T) -> (result:
    ProtobufMessage)
    ensures
        result.encoded() == ProtobufMessageView::of(data),
;

impl ProtobufMessageView {
    /// AXIOM: encoding a value loses nothing.
    #[verifier::external_body]
    pub broadcast proof fn axiom_of_injective<T>(a: T, b: T)
        ensures
            #[trigger] ProtobufMessageView::of(a) == #[trigger] ProtobufMessageView::of(b) ==> a == b,
    {
    }
}

} // verus!
