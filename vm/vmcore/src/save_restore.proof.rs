// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

// Examples supporting `save_restore.spec.rs`; no human review needed.

use crate::save_restore::RestoreError;
use crate::save_restore::SaveError;
use crate::save_restore::SaveRestore;
use crate::save_restore::SavedStateRoot;
use super::SaveRestoreSpec;
use super::SavedStateBlobView;
use crate::save_restore::SavedStateBlob;
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

impl super::SaveRestoreSpecImpl for ExampleDevice {
    closed spec fn saved_state(&self) -> u64 {
        self.register
    }
}

/// The blob of a `SaveRestore` object is its saved state, encoded.
impl<T: SaveRestore> super::ProtobufSaveRestoreSpecImpl for T where
    T::SavedState: 'static + Send + SavedStateRoot,
 {
    open spec fn saved_blob(&self) -> SavedStateBlobView {
        SavedStateBlobView::of(self.saved_state())
    }
}

} // verus!
