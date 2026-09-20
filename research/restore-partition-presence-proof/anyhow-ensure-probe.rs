// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

// Frontend diagnostic only; this is not a replacement for the production guard.
use vstd::prelude::*;

#[verus_verify]
#[verus_spec(result => ensures result.is_ok() == condition)]
fn ensure_condition(condition: bool) -> anyhow::Result<()> {
    anyhow::ensure!(condition, "condition must hold");
    Ok(())
}

fn main() {}
