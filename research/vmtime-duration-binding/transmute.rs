// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

use vstd::prelude::*;

#[verus_verify]
pub fn call_transmute(value: u32) -> u32 {
    unsafe { core::mem::transmute::<u32, u32>(value) }
}
