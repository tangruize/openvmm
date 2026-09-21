// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

use std::time::Duration;
use vstd::prelude::*;
use crate::duration_observation::elapsed_nanoseconds;

verus! {

pub type RestoreTime = Option<(Duration, u64, Option<u64>)>;

// A completion receipt, not a clock or an assertion that an RPC succeeded.
pub enum RestoreHistory {
    Unbased,
    Completed { adjustment: Option<Duration> },
}

pub open spec fn requested_adjustment(request: RestoreTime) -> Option<Duration> {
    match request {
        None => None,
        Some((duration, _, _)) => Some(duration),
    }
}

pub open spec fn requested_nanoseconds(request: RestoreTime) -> nat {
    match requested_adjustment(request) {
        None => 0,
        Some(duration) => elapsed_nanoseconds(&duration),
    }
}

pub open spec fn completed_elapsed_nanoseconds(history: RestoreHistory) -> Option<nat> {
    match history {
        RestoreHistory::Unbased => None,
        RestoreHistory::Completed { adjustment: None } => Some(0),
        RestoreHistory::Completed { adjustment: Some(duration) } =>
            Some(elapsed_nanoseconds(&duration)),
    }
}

} // verus!
