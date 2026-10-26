// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

// Examples supporting `save_restore.spec.rs`; no human review needed.

use crate::save_restore::RestoreError;
use crate::save_restore::SaveError;
use crate::save_restore::SaveRestore;
use vstd::prelude::*;

verus! {

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

} // verus!
