// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

use parking_lot::Mutex;
use vstd::prelude::*;

#[verus_verify]
pub fn observe(lock: &Mutex<u64>) -> u64 {
    *lock.lock()
}
