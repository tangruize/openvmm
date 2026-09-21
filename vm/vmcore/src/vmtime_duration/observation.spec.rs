// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

use std::time::Duration;
use vstd::prelude::*;

verus! {

/// Trusted observation of this Duration's actual core::time::Duration::secs.
pub uninterp spec fn whole_seconds(duration: &Duration) -> u64;

/// Trusted observation of this Duration's actual nanos.as_inner(), not a free
/// elapsed-time parameter. The as_nanos contract supplies its representation bound.
pub uninterp spec fn subsecond_nanoseconds(duration: &Duration) -> u32;

/// Total elapsed nanoseconds, defined from this Duration's concrete components.
pub open spec fn elapsed_nanoseconds(duration: &Duration) -> nat {
    whole_seconds(duration) as nat * 1_000_000_000
        + subsecond_nanoseconds(duration) as nat
}

/// The selected library's bounded nanosecond observation.
#[verifier::external_fn_specification]
pub fn duration_as_nanos(duration: &Duration) -> (result: u128)
    ensures
        subsecond_nanoseconds(duration) < 1_000_000_000,
        result == elapsed_nanoseconds(duration),
{
    duration.as_nanos()
}

} // verus!
