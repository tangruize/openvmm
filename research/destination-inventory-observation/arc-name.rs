// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

use std::sync::Arc;
use vstd::prelude::*;

#[verus_verify]
#[verus_spec(result => ensures result@ == (**name)@)]
pub fn name_string(name: &Arc<str>) -> String {
    name.to_string()
}

#[verus_verify]
#[verus_spec(result => ensures result@ == name@)]
pub fn str_string(name: &str) -> String {
    name.to_string()
}
