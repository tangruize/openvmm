// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

use vstd::prelude::*;

#[verus_verify]
pub fn missing_bit(bits: &[bool]) -> Option<usize> {
    bits.iter().position(|present| !present)
}
