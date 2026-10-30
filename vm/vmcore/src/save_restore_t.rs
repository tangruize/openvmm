// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

// Trusted: the encode/decode contract of saved-state blobs.
//
// TRUSTED, do not attempt to prove anything in this file: Verus uses only
// the contracts and axioms here, and they are claims about protobuf
// encoding, which is external (`SNAPSHOT-SPEC-TRUSTED.md`, Part 1).
//
// Everything in this file is TRUSTED, and listed in
// `SNAPSHOT-SPEC-TRUSTED.md`. A `SavedStateBlob` is a `ProtobufAny`: the
// saved-state type's URL and the protobuf encoding of the value. Protobuf
// encoding and decoding are not verified. What we assume:
//
// - `new(value)` gives a blob whose view is `SavedStateBlobView::of(value)`.
// - A successful `parse::<T>()` returns a `value` whose `of(value)` is the
//   blob's view. (`parse::<T>` fails unless the blob's type URL is `T`'s.)
// - `of(value)` is `of_view::<T>(value@)`: a blob is determined by the
//   saved state's view, which keeps every field but guest time (out of the
//   spec's scope; each saved-state type's view is reviewed for this).
// - `of_view::<T>` is injective: each saved-state type's protobuf encoding
//   loses nothing, so decoding returns a value with the view encoded.
// - A clone encodes the same value.
//
// Nothing is assumed across types: `of::<T>(a)` and `of::<U>(b)` are
// unrelated.

use super::SavedStateBlob;
use super::SavedStateRoot;
use super::spec::SavedStateBlobView;
use mesh::payload::DefaultEncoding;
use mesh::payload::DescribedProtobuf;
use mesh::payload::FieldDecode;
use mesh::payload::FieldEncode;
use mesh::payload::MessageDecode;
use mesh::payload::MessageEncode;
use mesh::payload::NoResources;
use mesh::payload::Protobuf;
use vstd::prelude::*;

verus! {

/// The encoding, uninterpreted (TRUSTED with the contracts below: protobuf
/// is external).
impl SavedStateBlobView {
    /// The view of a blob that encodes `value`, of saved-state type `T`.
    pub uninterp spec fn of<T>(value: T) -> SavedStateBlobView;

    /// The view of a blob that encodes a saved state of type `T` whose view
    /// is `view`.
    pub uninterp spec fn of_view<T: View>(view: T::V) -> SavedStateBlobView;
}

impl View for SavedStateBlob {
    type V = SavedStateBlobView;

    /// The value this blob encodes (uninterpreted: protobuf).
    uninterp spec fn view(&self) -> SavedStateBlobView;
}


// The trait chain behind `new` and `parse`'s bound `T: SavedStateRoot`,
// declared to Verus (no methods: the contracts below need only the traits
// and their bounds). `SavedStateRoot: DescribedProtobuf: Protobuf`, and
// `Protobuf`'s associated type is bounded by four codec traits, two with
// higher-ranked lifetimes; declaring `Protobuf` needs Verus patch 0003
// (`verus-patches/`).

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExNoResources(NoResources);

#[verifier::external_trait_specification]
pub trait ExDefaultEncoding {
    type ExternalTraitSpecificationFor: DefaultEncoding;

    type Encoding;
}

#[verifier::external_trait_specification]
pub trait ExMessageEncode<T, R>: Sized {
    type ExternalTraitSpecificationFor: MessageEncode<T, R>;
}

#[verifier::external_trait_specification]
pub trait ExMessageDecode<'a, T, R>: Sized {
    type ExternalTraitSpecificationFor: MessageDecode<'a, T, R>;
}

#[verifier::external_trait_specification]
pub trait ExFieldEncode<T, R>: Sized {
    type ExternalTraitSpecificationFor: FieldEncode<T, R>;
}

#[verifier::external_trait_specification]
pub trait ExFieldDecode<'a, T, R>: Sized {
    type ExternalTraitSpecificationFor: FieldDecode<'a, T, R>;
}

#[verifier::external_trait_specification]
pub trait ExProtobuf: DefaultEncoding<Encoding = <Self as ExProtobuf>::Encoding> + Sized {
    type ExternalTraitSpecificationFor: Protobuf;

    type Encoding: MessageEncode<Self, NoResources> + for<'a> MessageDecode<'a, Self, NoResources>
        + FieldEncode<Self, NoResources> + for<'a> FieldDecode<'a, Self, NoResources>;
}

// The derive's encoders. `#[derive(Protobuf)]` gives a struct
// `DefaultEncoding::Encoding = TableEncoder` and implements the table traits
// below for it, and an enum `OneofEncoder` with the oneof traits; each
// encoder implements the codec traits for any type with those. With these,
// Verus sees that a derived saved-state struct or enum is a
// `SavedStateRoot`. (The derive's `transparent` and `prost` modes are not
// declared; no saved-state root uses them.)

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExTableEncoder(mesh::payload::table::TableEncoder);

#[verifier::external_trait_specification]
pub unsafe trait ExStructMetadata {
    type ExternalTraitSpecificationFor: mesh::payload::table::StructMetadata;
}

#[verifier::external_trait_specification]
pub unsafe trait ExStructEncodeMetadata<R>: mesh::payload::table::StructMetadata {
    type ExternalTraitSpecificationFor: mesh::payload::table::encode::StructEncodeMetadata<R>;
}

#[verifier::external_trait_specification]
pub unsafe trait ExStructDecodeMetadata<'de, R>: mesh::payload::table::StructMetadata {
    type ExternalTraitSpecificationFor: mesh::payload::table::decode::StructDecodeMetadata<'de, R>;
}

#[verifier::external_trait_specification]
pub trait ExDescribeTable {
    type ExternalTraitSpecificationFor: mesh::payload::table::DescribeTable;
}

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExOneofEncoder(mesh::payload::oneof::OneofEncoder);

#[verifier::external_trait_specification]
pub trait ExOneofEncode<R> {
    type ExternalTraitSpecificationFor: mesh::payload::oneof::OneofEncode<R>;
}

#[verifier::external_trait_specification]
pub trait ExOneofDecode<'de, R>: Sized {
    type ExternalTraitSpecificationFor: mesh::payload::oneof::OneofDecode<'de, R>;
}

#[verifier::external_trait_specification]
pub trait ExDescribeOneof {
    type ExternalTraitSpecificationFor: mesh::payload::oneof::DescribeOneof;
}

#[verifier::external_trait_specification]
pub trait ExDescribeMessage<T> {
    type ExternalTraitSpecificationFor: mesh::payload::protofile::DescribeMessage<T>;
}

#[verifier::external_trait_specification]
pub trait ExDescribedProtobuf: Protobuf {
    type ExternalTraitSpecificationFor: DescribedProtobuf;
}

#[verifier::external_trait_specification]
pub trait ExSavedStateRoot: DescribedProtobuf {
    type ExternalTraitSpecificationFor: SavedStateRoot;
}

/// Declares `mesh::payload::Error` (a decoding error) to Verus, by name
/// only (fields hidden).
#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExPayloadError(mesh::payload::Error);

/// Trusted: encoding gives a blob whose view is `of(data)`.
pub assume_specification<T: SavedStateRoot>[ SavedStateBlob::new::<T> ](data: T) -> (result:
    SavedStateBlob)
    ensures
        result@ == SavedStateBlobView::of(data),
;

/// Trusted: a successful decoding returns a value whose `of` is the blob's
/// view.
pub assume_specification<T: SavedStateRoot>[ SavedStateBlob::parse::<T> ](blob: &SavedStateBlob) -> (result:
    Result<T, mesh::payload::Error>)
    ensures
        result matches Ok(value) ==> SavedStateBlobView::of(value) == blob@,
;

/// A decoding error as a `RestoreError` (`RestoreError::ProtobufDecode`).
/// No contract.
pub assume_specification[ <super::RestoreError as From<mesh::payload::Error>>::from ](err:
    mesh::payload::Error) -> super::RestoreError;

/// Trusted: a clone encodes the same value.
pub assume_specification[ <SavedStateBlob as Clone>::clone ](blob: &SavedStateBlob) -> (result:
    SavedStateBlob)
    ensures
        result@ == blob@,
;

impl SavedStateBlobView {
    /// AXIOM: a saved state's blob is determined by its view.
    #[verifier::external_body]
    pub broadcast proof fn axiom_of_is_of_view<T: SavedStateRoot + View>(value: T)
        ensures
            #[trigger] SavedStateBlobView::of(value) == SavedStateBlobView::of_view::<T>(value@),
    {
    }

    /// AXIOM: encoding loses nothing of the view.
    #[verifier::external_body]
    pub broadcast proof fn axiom_of_view_injective<T: View>(a: T::V, b: T::V)
        ensures
            #[trigger] SavedStateBlobView::of_view::<T>(a)
                == #[trigger] SavedStateBlobView::of_view::<T>(b) ==> a == b,
    {
    }
}

} // verus!
