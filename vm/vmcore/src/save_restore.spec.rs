// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

// High-level spec of saved-state blobs, for human review.

use super::ProtobufSaveRestore;
use super::RestoreError;
use super::SaveError;
use super::SaveRestore;
use super::SavedStateBlob;
use vstd::prelude::*;

verus! {

/// Declares `SavedStateBlob` to Verus, by name only (fields hidden).
#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExSavedStateBlob(SavedStateBlob);

/// Declares `SaveError` to Verus, by name only (fields hidden).
#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExSaveError(SaveError);

/// Declares `RestoreError` to Verus, by name only (fields hidden).
#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExRestoreError(RestoreError);

/// The abstract value of a saved-state blob: the view of the typed saved
/// state it encodes, independent of its protobuf encoding. A blob that
/// encodes `value: T` has view `SavedStateBlobView::of(value)`, which is
/// `of_view::<T>(value@)`: a blob is determined by the saved state's view.
///
/// A saved-state type's view keeps every field except guest time, which is
/// out of the spec's scope (`SNAPSHOT-SPEC-DESIGN.md`, "Out of scope for
/// now"): the time-keeping units' time fields (the partition's VPs: TSC
/// and LAPIC timers; vmtime; the RTC's clock) are not in their saved
/// states' views. Each saved-state type's view is reviewed for this (in its
/// `.spec.rs`); a type not yet specified has its identity view.
///
/// The trusted contract (`save_restore_t.rs`): `SavedStateBlob::new(value)`
/// has view `of(value)`; a successful `parse::<T>()` returns a `value` with
/// `of(value)` equal to the blob's view; `of(value)` is
/// `of_view::<T>(value@)`; and `of_view::<T>` is injective.
#[verifier::external_body]
pub struct SavedStateBlobView {
    _private: (),
}

// `SavedStateBlobView::of`, `of_view`, and `View for SavedStateBlob` (the
// encoding, uninterpreted) are in `save_restore_t.rs`, with the encoding's
// trusted contracts.


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

impl UnitSnapshot {
    /// The snapshot of a unit whose save returns `blob`: `Saved(b)` for a
    /// saved state `b`, `NoSavedState` for none.
    pub open spec fn from_saved(blob: Option<SavedStateBlobView>) -> UnitSnapshot {
        match blob {
            Some(b) => UnitSnapshot::Saved(b),
            None => UnitSnapshot::NoSavedState,
        }
    }
}

/// What `SaveRestore`'s contract needs of an object and of its saved
/// state: a view. Every `SaveRestore` type and every saved-state type has
/// one (the trait's supertrait and `SavedState`'s bound, in Verus builds).
/// A specified device's view is its abstract state; a device not yet
/// specified has the placeholder view `()`. A saved-state type's view is
/// its fields without time; one not yet specified has its identity view.
pub trait SaveRestoreView: View {}

impl<T: View + ?Sized> SaveRestoreView for T {}

/// The contract of every `SaveRestore` implementation (the unit-side
/// contract at the device level; see `SNAPSHOT-SPEC-DESIGN.md`), on views:
/// `save` does not change the object's view, and returns a saved state
/// whose view is `saved_view(view)`; after a successful `restore(state)`,
/// `saved_view(view)` is `state`'s view. No blobs: those are generic
/// (`ProtobufSaveRestore`).
///
/// `saved_view` is defined by each specified implementation (in
/// `SaveRestoreSpecImpl`); unverified implementations are trusted to meet
/// the contract.
#[verifier::external_trait_specification]
#[verifier::external_trait_extension(SaveRestoreSpec via SaveRestoreSpecImpl)]
pub trait ExSaveRestore: SaveRestoreView {
    type ExternalTraitSpecificationFor: SaveRestore;

    type SavedState: SaveRestoreView;

    /// The view of the saved state of an object with view `view`: what its
    /// `save` returns, as a view. (Projects out what the object does not
    /// save, e.g. static configuration.)
    spec fn saved_view(view: <Self as View>::V) -> <Self::SavedState as View>::V;

    /// Well formed: what a valid object of this type satisfies (its view's
    /// `inv()` and the properties of its other fields), defined by each
    /// implementation that is specified. `save` and `restore` keep it.
    spec fn device_wf(&self) -> bool;

    fn save(&mut self) -> (result: Result<Self::SavedState, SaveError>)
        requires
            old(self).device_wf(),
        ensures
            final(self).device_wf(),
            final(self)@ == old(self)@,
            result matches Ok(state) ==> state@ == Self::saved_view(old(self)@),
    ;

    fn restore(&mut self, state: Self::SavedState) -> (result: Result<(), RestoreError>)
        requires
            old(self).device_wf(),
        ensures
            final(self).device_wf(),
            result is Ok ==> Self::saved_view(final(self)@) == state@,
    ;
}

/// The contract of every `ProtobufSaveRestore` implementation (layer 2 of
/// the unit-side contract): `saved_blob()` is the object's snapshotable
/// state, encoded: what `save` returns. `save` does not change it; a
/// successful `restore(state)` sets it to `state@`.
///
/// Given through `ProtobufSaveRestoreSpec`, as for `SaveRestore`. The
/// blanket implementation for `SaveRestore` types is verified against it,
/// with `saved_blob()` the blob of the object's saved view
/// (`of_view(saved_view(self@))`)
/// (`save_restore.proof.rs`); the other implementations
/// (`ChipsetDeviceProxy`, `BoundVp`, `ErasedChipsetDevice`) are trusted to
/// meet it.
#[verifier::external_trait_specification]
#[verifier::external_trait_extension(ProtobufSaveRestoreSpec via ProtobufSaveRestoreSpecImpl)]
pub trait ExProtobufSaveRestore {
    type ExternalTraitSpecificationFor: ProtobufSaveRestore;

    /// The blob `save` returns: the object's snapshotable state, encoded.
    spec fn saved_blob(&self) -> SavedStateBlobView;

    /// Well formed (for a `SaveRestore` type, its `device_wf()`). `save`
    /// and `restore` keep it.
    spec fn device_wf(&self) -> bool;

    fn save(&mut self) -> (result: Result<SavedStateBlob, SaveError>)
        requires
            old(self).device_wf(),
        ensures
            final(self).device_wf(),
            final(self).saved_blob() == old(self).saved_blob(),
            result matches Ok(blob) ==> blob@ == old(self).saved_blob(),
    ;

    fn restore(&mut self, state: SavedStateBlob) -> (result: Result<(), RestoreError>)
        requires
            old(self).device_wf(),
        ensures
            final(self).device_wf(),
            result is Ok ==> final(self).saved_blob() == state@,
    ;
}

} // verus!
