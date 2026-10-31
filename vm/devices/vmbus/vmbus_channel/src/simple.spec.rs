// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

// OUT OF SCOPE (vmbus): the Verus spec in this file is parked. vmbus is not
// on the microVM's snapshot path (`snapshot-spec-docs/DESIGN.md`, "Out of scope
// for now"). Do not attempt to prove anything here. (Kept because the
// `StateUnit` trait requires the vmbus units to follow its protocol, and so
// that the work is not lost if vmbus comes back into scope.)

// A simple device's handle, for human review.

use super::SimpleDeviceHandle;
use super::SimpleVmbusDevice;
use vstd::prelude::*;
use vstd::resource::Loc;

verus! {

impl<T: SimpleVmbusDevice> SimpleDeviceHandle<T> {
    /// The id of the device's unit's FracGhost: its channel's.
    pub closed spec fn id(&self) -> Loc {
        self.0.id()
    }
}

} // verus!
