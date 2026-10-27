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
/// The view forgets guest time, which is out of the spec's scope
/// (`SNAPSHOT-SPEC-DESIGN.md`, "Out of scope for now"): for the saved-state
/// types of the units that keep guest time (the partition's VPs: TSC and
/// LAPIC timers; vmtime; the RTC's clock), values that differ only in their
/// time fields have the same view. For other types it forgets nothing.
///
/// The trusted contract (`save_restore_t.rs`): `SavedStateBlob::new(value)`
/// has view `of(value)`; a successful `parse::<T>()` returns a `value` with
/// `of(value)` equal to the blob's view; and `of(a) == of(b)` exactly when
/// `a` and `b` are equal except for time (`equal_except_time`).
#[verifier::external_body]
pub struct SavedStateBlobView {
    _private: (),
}

impl SavedStateBlobView {
    /// The view of a blob that encodes `value`, of saved-state type `T`.
    pub uninterp spec fn of<T>(value: T) -> SavedStateBlobView;

    /// `a` and `b` differ at most in their time fields. Equality for a type
    /// without time fields; for the time-keeping units' types, equality
    /// except the TSC and LAPIC timer state (the partition's VPs), the VM
    /// time and its stop time (vmtime), and the clock (the RTC). Trusted per
    /// type.
    pub uninterp spec fn equal_except_time<T>(a: T, b: T) -> bool;
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

    /// Well formed: what a valid object of this type satisfies (its view's
    /// `inv()` and the properties of its other fields), defined by each
    /// implementation that is specified. `save` and `restore` keep it.
    spec fn device_wf(&self) -> bool;

    fn save(&mut self) -> (result: Result<Self::SavedState, SaveError>)
        requires
            old(self).device_wf(),
        ensures
            final(self).device_wf(),
            final(self).saved_state() == old(self).saved_state(),
            result matches Ok(state) ==> state == old(self).saved_state(),
    ;

    fn restore(&mut self, state: Self::SavedState) -> (result: Result<(), RestoreError>)
        requires
            old(self).device_wf(),
        ensures
            final(self).device_wf(),
            result is Ok ==> final(self).saved_state() == state,
    ;
}

/// The contract of every `ProtobufSaveRestore` implementation (layer 2 of
/// the unit-side contract): `saved_blob()` is the object's snapshotable
/// state, encoded: what `save` returns. `save` does not change it; a
/// successful `restore(state)` sets it to `state@`.
///
/// Given through `ProtobufSaveRestoreSpec`, as for `SaveRestore`. The
/// blanket implementation for `SaveRestore` types is verified against it,
/// with `saved_blob()` the encoding of `saved_state()`
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
