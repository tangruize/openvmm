// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

#![feature(core_intrinsics)]
#![allow(internal_features)]

use vstd::prelude::*;

#[verus_verify]
pub fn intrinsic_write(slot: &mut u32, value: u32) {
    unsafe { core::intrinsics::write_via_move(slot, value) }
}
