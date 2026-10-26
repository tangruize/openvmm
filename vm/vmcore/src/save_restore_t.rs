// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

// Trusted: the encode/decode contract of saved-state blobs.
//
// Everything in this file is TRUSTED, and listed in
// `SNAPSHOT-SPEC-TRUSTED.md`. A `SavedStateBlob` is a `ProtobufAny`: the
// saved-state type's URL and the protobuf encoding of the value. Protobuf
// encoding and decoding are not verified. What we assume:
//
// - `new(value)` gives a blob whose view is `SavedStateBlobView::of(value)`.
// - A successful `parse::<T>()` returns a `value` whose `of(value)` is the
//   blob's view. (`parse::<T>` fails unless the blob's type URL is `T`'s.)
// - `of::<T>` is injective: each saved-state type's protobuf encoding loses
//   nothing, so decoding returns exactly the value that was encoded.
// - A clone encodes the same value.
//
// Nothing is assumed across types: `of::<T>(a)` and `of::<U>(b)` are
// unrelated.

use super::SavedStateBlob;
use super::spec::SavedStateBlobView;
use vstd::prelude::*;

verus! {

// The contract of `new` and `parse` is not yet attached to them: their
// `T: SavedStateRoot` bound needs `SavedStateRoot`, `DescribedProtobuf`,
// `Protobuf`, and `Protobuf`'s codec traits declared to Verus, and
// `Protobuf`'s associated type is bounded by four generic codec traits (two
// with higher-ranked lifetimes). Only the unit side (`ProtobufSaveRestore`)
// calls them, so the plan is to attach the contract with that work, through
// a local trait with `encode`/`decode` methods carrying these contracts,
// blanket-implemented for every `SavedStateRoot` type. Stated contract:
//
//   SavedStateBlob::new::<T>(data)
//       ensures result@ == SavedStateBlobView::of(data)
//   SavedStateBlob::parse::<T>(&blob)
//       ensures result matches Ok(value) ==> SavedStateBlobView::of(value) == blob@

/// Trusted: a clone encodes the same value.
pub assume_specification[ <SavedStateBlob as Clone>::clone ](blob: &SavedStateBlob) -> (result:
    SavedStateBlob)
    ensures
        result@ == blob@,
;

impl SavedStateBlobView {
    /// AXIOM: encoding a saved-state value loses nothing.
    #[verifier::external_body]
    pub broadcast proof fn axiom_of_injective<T>(a: T, b: T)
        ensures
            #[trigger] SavedStateBlobView::of(a) == #[trigger] SavedStateBlobView::of(b) ==> a == b,
    {
    }
}

} // verus!
