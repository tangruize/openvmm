// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

// High-level spec of the microVM chipset devices, for human review.

use super::MicrovmShutdown;
use super::MicrovmSnapshotRequest;
use vmcore::save_restore::NoSavedState;
use vmcore::save_restore::spec::SaveRestoreSpecImpl;
use vstd::prelude::*;

verus! {

/// The shutdown port saves nothing; every state is valid.
/// Its view is empty.
impl View for MicrovmShutdown {
    type V = ();

    open spec fn view(&self) -> () {
        ()
    }
}

impl SaveRestoreSpecImpl for MicrovmShutdown {
    open spec fn saved_view(view: ()) -> NoSavedState {
        NoSavedState
    }

    open spec fn device_wf(&self) -> bool {
        true
    }
}

/// The snapshot-request port saves nothing (its pending request, if any,
/// is not saved); every state is valid.
/// Its view is empty.
impl View for MicrovmSnapshotRequest {
    type V = ();

    open spec fn view(&self) -> () {
        ()
    }
}

impl SaveRestoreSpecImpl for MicrovmSnapshotRequest {
    open spec fn saved_view(view: ()) -> NoSavedState {
        NoSavedState
    }

    open spec fn device_wf(&self) -> bool {
        true
    }
}

} // verus!
