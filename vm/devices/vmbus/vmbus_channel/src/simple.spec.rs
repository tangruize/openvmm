// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

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
