// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

// Examples supporting `save_restore.spec.rs`; no human review needed.

use super::SaveRestoreSpec;
use super::SavedStateBlobView;
use crate::save_restore::NoSavedState;
use crate::save_restore::RestoreError;
use crate::save_restore::SaveError;
use crate::save_restore::SaveRestore;
use crate::save_restore::SavedStateBlob;
use crate::save_restore::SavedStateRoot;
use mesh::payload::Protobuf;
use vstd::prelude::*;

/// An example saved-state root that is an enum (all of OpenVMM's are
/// structs), to check that enum roots meet `SavedStateBlob::new`'s bound.
/// Not production code.
#[derive(Protobuf, crate::save_restore::SavedStateRoot)]
#[mesh(package = "save_restore.example")]
pub(crate) enum ExampleEnumState {
    #[mesh(1)]
    Off,
    #[mesh(2)]
    On(#[mesh(1)] u32),
}

verus! {

/// The saved state of a device that saves nothing.
#[verifier::external_type_specification]
pub struct ExNoSavedState(NoSavedState);

/// `NoSavedState`'s view is itself.
impl View for NoSavedState {
    type V = NoSavedState;

    open spec fn view(&self) -> NoSavedState {
        *self
    }
}

// `inspect`'s traits, declared to Verus here, in a crate every user of them
// depends on (`StateUnit` and `VmbusDevice` have `InspectMut` as a
// supertrait; units reached through `&X` get it from `Inspect`).

#[verifier::external_trait_specification]
pub trait ExInspectMut {
    type ExternalTraitSpecificationFor: inspect::InspectMut;
}

#[verifier::external_trait_specification]
pub trait ExInspect {
    type ExternalTraitSpecificationFor: inspect::Inspect;
}

/// Declares `std::task::Waker` to Verus, as an opaque type (devices keep
/// wakers; here, in a crate every device crate depends on).
#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExWaker(std::task::Waker);

/// Declares `anyhow::Error` to Verus, as an opaque type.
#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExAnyhowError(anyhow::Error);

/// Declares `inspect::Deferred` (a deferred inspect request) to Verus, as an
/// opaque type.
#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExDeferred(inspect::Deferred);


#[verifier::external_type_specification]
#[verifier::external_body]
pub(crate) struct ExExampleEnumState(ExampleEnumState);

/// Encodes the example enum state through the real `SavedStateBlob::new`.
pub(crate) fn example_encode_enum(state: ExampleEnumState) -> (blob: SavedStateBlob)
    ensures
        blob@ == SavedStateBlobView::of(state),
{
    SavedStateBlob::new(state)
}

/// An example device meeting the `SaveRestore` contract: its saved state is
/// its register. Not production code.
pub(crate) struct ExampleDevice {
    pub(crate) register: u64,
}

impl SaveRestore for ExampleDevice {
    type SavedState = u64;

    fn save(&mut self) -> (result: Result<u64, SaveError>) {
        Ok(self.register)
    }

    fn restore(&mut self, state: u64) -> (result: Result<(), RestoreError>) {
        self.register = state;
        Ok(())
    }
}

/// The example's view is its register.
impl View for ExampleDevice {
    type V = u64;

    closed spec fn view(&self) -> u64 {
        self.register
    }
}

impl super::SaveRestoreSpecImpl for ExampleDevice {
    /// It saves its register.
    closed spec fn saved_view(view: u64) -> u64 {
        view
    }

    /// Every value of the register is valid.
    closed spec fn device_wf(&self) -> bool {
        true
    }
}

/// The blob of a `SaveRestore` object is that of its saved view.
impl<T: SaveRestore> super::ProtobufSaveRestoreSpecImpl for T where
    T::SavedState: 'static + Send + SavedStateRoot,
 {
    open spec fn saved_blob(&self) -> SavedStateBlobView {
        SavedStateBlobView::of_view::<T::SavedState>(T::saved_view(self@))
    }

    open spec fn device_wf(&self) -> bool {
        SaveRestoreSpec::device_wf(self)
    }
}

} // verus!
