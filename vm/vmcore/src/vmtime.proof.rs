// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

verus! {

impl View for VmTime {
    type V = u64;

    closed spec fn view(&self) -> u64 {
        self.0
    }
}

} // verus!
