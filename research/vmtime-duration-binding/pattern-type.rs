// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

#![feature(pattern_type_macro, pattern_types)]

use vstd::prelude::*;

#[verus_verify]
pub fn accept_pattern_type(_value: core::pattern_type!(u32 is 0..=999_999_999)) {}
