// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

#![feature(proc_macro_hygiene)]

use vstd::prelude::*;

#[verus_verify]
#[verus_spec(result => ensures result == pair.0)]
fn project(pair: &(u32, u64)) -> u32 {
    let read =
        #[verus_spec(result: u32 => ensures result == *index)]
        |(index, _): &(u32, u64)| *index;
    read(pair)
}
