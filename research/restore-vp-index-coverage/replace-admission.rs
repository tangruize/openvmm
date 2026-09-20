// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

use vstd::prelude::*;

#[verus_verify]
#[verus_spec(result =>
    ensures result == *old(slot), *final(slot) == true,
)]
pub fn replace_bit(slot: &mut bool) -> bool {
    std::mem::replace(slot, true)
}
