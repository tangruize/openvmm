// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

// Isolates the macro's dependency call from Result, error allocation and strings.
use vstd::prelude::*;

#[verus_verify]
#[verus_spec(result => ensures result == !condition)]
fn dependency_not(condition: bool) -> bool {
    anyhow::__private::not(condition)
}

fn main() {}
