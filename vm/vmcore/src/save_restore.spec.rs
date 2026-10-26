// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

// High-level spec of saved-state blobs, for human review.

use super::SavedStateBlob;
use vstd::prelude::*;

verus! {

/// Declares `SavedStateBlob` to Verus, as an opaque type.
#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExSavedStateBlob(SavedStateBlob);

/// The abstract value of a saved-state blob: the typed saved state it
/// encodes, independent of its protobuf encoding.
///
/// Opaque for now. The trusted contract that relates it to
/// `SavedStateBlob::new` and `SavedStateBlob::parse` (decoding returns the
/// value that was encoded) is still to be written; see gap 4 in
/// `SNAPSHOT-SPEC-DESIGN.md`.
#[verifier::external_body]
pub struct SavedStateBlobView {
    _private: (),
}

impl View for SavedStateBlob {
    type V = SavedStateBlobView;

    /// Trusted: the value this blob encodes.
    uninterp spec fn view(&self) -> SavedStateBlobView;
}

/// Trusted: a clone encodes the same value.
pub assume_specification[ <SavedStateBlob as Clone>::clone ](
    blob: &SavedStateBlob,
) -> (result: SavedStateBlob)
    ensures
        result@ == blob@,
;

} // verus!
