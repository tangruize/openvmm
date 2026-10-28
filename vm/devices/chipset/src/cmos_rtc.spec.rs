// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

// High-level spec of the CMOS RTC, for human review.

use super::super::Rtc;
use super::state::SavedState;
use vmcore::save_restore::spec::SaveRestoreSpecImpl;
use vstd::prelude::*;

verus! {

/// The view of the RTC: its snapshotable state without time. (Guest time
/// is out of the spec's scope: not in the view are the clock, i.e. the UTC
/// time source, and whether the latched calendar sample is valid,
/// `time_valid`. Also not in it: the static configuration, the interrupt
/// line, the vmtime timers, and the update-bit book-keeping.) It is also
/// the view of the RTC's saved state.
pub struct RtcView {
    /// The selected CMOS register (port 0x70).
    pub(super) addr: u8,
    /// The CMOS bytes.
    pub(super) cmos: [u8; 256],
    /// The retired coherent-transaction state: always `None`.
    pub(super) transaction_read_mask: Option<u16>,
}

impl RtcView {
    /// No coherent transaction (the state is retired: nothing sets it, and
    /// `restore` rejects a saved state that has one).
    pub(super) open spec fn inv(self) -> bool {
        self.transaction_read_mask is None
    }
}

impl View for Rtc {
    type V = RtcView;

    closed spec fn view(&self) -> RtcView {
        RtcView {
            addr: self.state.addr,
            cmos: self.state.cmos.0,
            transaction_read_mask: self.state.transaction_read_mask,
        }
    }
}

impl Rtc {
    /// Well formed: the view's invariant holds.
    pub closed spec fn wf(&self) -> bool {
        self@.inv()
    }
}

/// The view of the RTC's saved state: its fields without time
/// (`clock_time_millis`, the guest-visible UTC time, and `time_valid`,
/// which `restore` reads as `true` when absent).
impl View for SavedState {
    type V = RtcView;

    closed spec fn view(&self) -> RtcView {
        RtcView {
            addr: self.addr,
            cmos: self.cmos,
            transaction_read_mask: self.transaction_read_mask,
        }
    }
}

/// The RTC saves its view (its saved state's view is the RTC's view).
impl SaveRestoreSpecImpl for Rtc {
    closed spec fn saved_view(view: RtcView) -> RtcView {
        view
    }

    /// The RTC is well formed (`wf()`).
    closed spec fn device_wf(&self) -> bool {
        self.wf()
    }
}

} // verus!
