// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

use vstd::prelude::*;

#[verus_verify]
fn project(pair: &(u32, u64)) -> u32 {
    let read = |(index, _): &(u32, u64)| *index;
    read(pair)
}
