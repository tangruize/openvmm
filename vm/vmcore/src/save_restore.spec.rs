// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

// High-level spec of saved-state blobs, for human review.

use super::RestoreError;
use super::SaveError;
use super::SaveRestore;
use super::SavedStateBlob;
use vstd::prelude::*;

verus! {

/// Declares `SavedStateBlob` to Verus, as an opaque type.
#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExSavedStateBlob(SavedStateBlob);

/// Declares `SaveError` to Verus, as an opaque type.
#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExSaveError(SaveError);

/// Declares `RestoreError` to Verus, as an opaque type.
#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExRestoreError(RestoreError);

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

/// What a state unit's FracGhost records: the unit's snapshotable state,
/// that is, what its save returns. (Here, not in `state_unit`, so that
/// devices, which hold their unit's FracGhost, can name it.)
pub enum UnitSnapshot {
    /// Not recorded: the unit has been neither stopped nor restored since it
    /// was created. (In `StateUnits`' view, also a unit that is running.)
    Unknown,
    /// The unit has no saved state (its save returns `None`).
    NoSavedState,
    /// The unit's saved state.
    Saved(SavedStateBlobView),
}

/// The contract of every `SaveRestore` implementation (the unit-side
/// contract at the device level; see `SNAPSHOT-SPEC-DESIGN.md`).
/// `saved_state()` is the object's snapshotable state: what `save` returns.
/// `save` does not change it; a successful `restore(state)` sets it to
/// `state`.
///
/// Given through `SaveRestoreSpec`, so that the trait itself and its
/// implementations are unchanged. An implementation defines `saved_state`
/// by implementing `SaveRestoreSpecImpl`; unverified implementations are
/// trusted to meet the contract.
#[verifier::external_trait_specification]
#[verifier::external_trait_extension(SaveRestoreSpec via SaveRestoreSpecImpl)]
pub trait ExSaveRestore {
    type ExternalTraitSpecificationFor: SaveRestore;

    type SavedState;

    /// The state `save` returns: the object's snapshotable state.
    spec fn saved_state(&self) -> Self::SavedState;

    fn save(&mut self) -> (result: Result<Self::SavedState, SaveError>)
        ensures
            final(self).saved_state() == old(self).saved_state(),
            result matches Ok(state) ==> state == old(self).saved_state(),
    ;

    fn restore(&mut self, state: Self::SavedState) -> (result: Result<(), RestoreError>)
        ensures
            result is Ok ==> final(self).saved_state() == state,
    ;
}

} // verus!
