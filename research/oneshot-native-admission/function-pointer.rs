// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

// Frontend-only reduction of the type constructor rejected in the production
// SlotState and SlotHandler. This is not a channel implementation or proof.
use vstd::prelude::*;

type DecodeFn = unsafe fn(u8) -> u8;

#[verus_verify]
struct Handler {
    decode: DecodeFn,
}

fn main() {}
