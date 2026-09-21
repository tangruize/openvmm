// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

use std::time::Duration;
use vstd::prelude::*;

#[path = "restore_history.spec.rs"]
mod history;
#[path = "../../openvmm/openvmm_core/src/worker/dispatch.spec.rs"]
mod frozen;
#[path = "../../vm/vmcore/src/vmtime_duration/mod.rs"]
mod duration_observation;

use history::*;

verus! {

// These are ghost operations only. Attaching them to the real body is owed.
pub proof fn clear_history(tracked history: &mut Ghost<RestoreHistory>)
    ensures final(history)@ == RestoreHistory::Unbased,
{
    *history = Ghost(RestoreHistory::Unbased);
}

pub proof fn record_completed_request(
    tracked history: &mut Ghost<RestoreHistory>,
    request: RestoreTime,
)
    ensures
        final(history)@ == (RestoreHistory::Completed {
            adjustment: requested_adjustment(request),
        }),
{
    *history = Ghost(RestoreHistory::Completed {
        adjustment: requested_adjustment(request),
    });
}

proof fn no_adjustment_is_retained(tracked history: &mut Ghost<RestoreHistory>)
    ensures
        final(history)@ == (RestoreHistory::Completed { adjustment: None }),
{
    clear_history(history);
    record_completed_request(history, None);
}

proof fn exact_duration_value_is_retained(
    tracked history: &mut Ghost<RestoreHistory>,
    duration: Duration,
    frequency: u64,
    apic_frequency: Option<u64>,
)
    ensures
        final(history)@ == (RestoreHistory::Completed { adjustment: Some(duration) }),
{
    clear_history(history);
    record_completed_request(history, Some((duration, frequency, apic_frequency)));
}

proof fn new_attempt_or_mutation_invalidates_previous_receipt(
    tracked history: &mut Ghost<RestoreHistory>,
    request: RestoreTime,
)
    ensures final(history)@ == RestoreHistory::Unbased,
{
    record_completed_request(history, request);
    clear_history(history);
}

proof fn latest_completion_replaces_previous_request(
    tracked history: &mut Ghost<RestoreHistory>,
    first: RestoreTime,
    second: RestoreTime,
)
    ensures
        final(history)@ == (RestoreHistory::Completed {
            adjustment: requested_adjustment(second),
        }),
{
    record_completed_request(history, first);
    clear_history(history);
    record_completed_request(history, second);
}

proof fn opaque_duration_values_are_not_collapsed(first: Duration, second: Duration)
    requires first != second,
    ensures
        requested_adjustment(Some((first, 1, None)))
            != requested_adjustment(Some((second, 1, None))),
{
}

proof fn keeper_ticks_do_not_identify_elapsed_nanoseconds()
    ensures
        frozen::vm_time_after_downtime(7, 0) == 7,
        frozen::vm_time_after_downtime(7, 1) == 7,
        frozen::vm_time_after_downtime(7, 99) == 7,
        frozen::vm_time_after_downtime(7, 1844674407370955161600) == 7,
        frozen::vm_time_after_downtime(0xffff_ffff_ffff_ffff, 100) == 0,
{
}

proof fn completed_request_has_exact_elapsed(
    tracked history: &mut Ghost<RestoreHistory>,
    request: RestoreTime,
)
    ensures
        completed_elapsed_nanoseconds(final(history)@)
            == Some(requested_nanoseconds(request)),
{
    clear_history(history);
    record_completed_request(history, request);
}

// Decoding the real request and attaching history to LoadedVm are still owed.
proof fn completed_elapsed_matches_frozen_projection(
    history: RestoreHistory,
    request: RestoreTime,
    decoded: frozen::RestoreRequestView,
)
    requires
        history == (RestoreHistory::Completed {
            adjustment: requested_adjustment(request),
        }),
        decoded.has_time_adjustment == request.is_some(),
        request.is_some() ==> decoded.downtime_ns == requested_nanoseconds(request),
    ensures
        completed_elapsed_nanoseconds(history) == Some(
            frozen::restored_virtual_time(
                decoded.saved_state.virtual_time, decoded,
            ).elapsed_since_snapshot_ns,
        ),
{
}

// An interface probe, not a replacement for any production restore operation.
fn observe_retained_duration(
    duration: Duration,
    Ghost(history): Ghost<RestoreHistory>,
) -> (result: u128)
    requires
        history == (RestoreHistory::Completed { adjustment: Some(duration) }),
    ensures
        completed_elapsed_nanoseconds(history) == Some(result as nat),
{
    duration.as_nanos()
}

} // verus!
