// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

#![feature(core_intrinsics)]
#![allow(internal_features)]

use vstd::prelude::*;

#[verus_verify]
pub fn intrinsic_read(slot: &u32) -> u32 {
    unsafe { core::intrinsics::read_via_copy(slot) }
}
