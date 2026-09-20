// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

verus! {

impl View for VpIndex {
    type V = u32;

    closed spec fn view(&self) -> u32 {
        self.0
    }
}

} // verus!
