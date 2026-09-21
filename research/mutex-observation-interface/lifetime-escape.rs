// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

include!("candidate.spec.rs");
use parking_lot::Mutex;

verus! {
pub fn escape(mutex: &Mutex<u64>) -> &u64 {
    let guard = mutex.lock();
    &*guard
}
}
