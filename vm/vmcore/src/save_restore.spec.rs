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
/// encodes, independent of its protobuf encoding. A blob that encodes
/// `value: T` has view `SavedStateBlobView::of(value)`.
///
/// The trusted contract (`save_restore_t.rs`): `SavedStateBlob::new(value)`
/// has view `of(value)`; a successful `parse::<T>()` returns a `value` with
/// `of(value)` equal to the blob's view; and `of::<T>` is injective, so
/// decoding returns exactly the value that was encoded.
#[verifier::external_body]
pub struct SavedStateBlobView {
    _private: (),
}

impl SavedStateBlobView {
    /// The view of a blob that encodes `value`, of saved-state type `T`.
    pub uninterp spec fn of<T>(value: T) -> SavedStateBlobView;
}

impl View for SavedStateBlob {
    type V = SavedStateBlobView;

    /// Trusted: the value this blob encodes (see `save_restore_t.rs`).
    uninterp spec fn view(&self) -> SavedStateBlobView;
}

} // verus!
