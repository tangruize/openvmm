// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

use vmcore::vmtime::VmTime;
use vstd::prelude::*;

#[verus_verify]
#[verus_spec(result => ensures result == n,)]
pub fn scalar_round_trip(n: u64) -> u64 {
    VmTime::from_100ns(n).as_100ns()
}

#[verus_verify]
#[verus_spec(result => ensures result == time@,)]
pub fn read_arbitrary_time(time: &VmTime) -> u64 {
    time.as_100ns()
}
