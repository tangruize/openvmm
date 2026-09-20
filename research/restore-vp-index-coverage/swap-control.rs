// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

use vstd::prelude::*;

#[verus_verify]
#[verus_spec(
    ensures *final(left) == *old(right), *final(right) == *old(left),
)]
pub fn swap_bits(left: &mut bool, right: &mut bool) {
    std::mem::swap(left, right);
}
