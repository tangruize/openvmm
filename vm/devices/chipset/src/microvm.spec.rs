// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

// High-level spec of the microVM chipset devices, for human review.

use super::BUFFER_MAX;
use super::MicrovmPortb;
use super::MicrovmPortbSavedState;
use super::MicrovmShutdown;
use super::MicrovmSnapshotRequest;
use vmcore::save_restore::NoSavedState;
use vmcore::save_restore::spec::SaveRestoreSpecImpl;
use vstd::prelude::*;

verus! {

/// The view of portb: its buffered bytes. It is also the view of its saved
/// state. (Not in the view: the host endpoint, the generation ID and the
/// restore packet with their read positions, the time ABI and its time
/// window, the input gate, and the wakers and drain requests; none is
/// saved. `restore` resets the read positions and clears the time
/// window.)
pub struct MicrovmPortbView {
    /// Host bytes accepted but not yet read by the guest.
    pub rx: Seq<u8>,
    /// Guest bytes accepted but not yet written to the host endpoint.
    pub tx: Seq<u8>,
}

impl MicrovmPortbView {
    /// Each buffer holds at most `BUFFER_MAX` bytes (portb accepts no more;
    /// `restore` rejects a saved state with more).
    pub(super) open spec fn inv(self) -> bool {
        &&& self.rx.len() <= BUFFER_MAX
        &&& self.tx.len() <= BUFFER_MAX
    }
}

impl View for MicrovmPortb {
    type V = MicrovmPortbView;

    closed spec fn view(&self) -> MicrovmPortbView {
        MicrovmPortbView { rx: self.rx_buffer@, tx: self.tx_buffer@ }
    }
}

impl MicrovmPortb {
    /// Well formed: the view's invariant holds.
    pub closed spec fn wf(&self) -> bool {
        self@.inv()
    }
}

/// The view of portb's saved state: its two buffers' bytes.
impl View for MicrovmPortbSavedState {
    type V = MicrovmPortbView;

    open spec fn view(&self) -> MicrovmPortbView {
        MicrovmPortbView { rx: self.rx_buffer@, tx: self.tx_buffer@ }
    }
}

/// Portb saves its view (its saved state's view is portb's view).
impl SaveRestoreSpecImpl for MicrovmPortb {
    closed spec fn saved_view(view: MicrovmPortbView) -> MicrovmPortbView {
        view
    }

    /// Portb is well formed (`wf()`).
    closed spec fn device_wf(&self) -> bool {
        self.wf()
    }
}

/// The shutdown port saves nothing; every state is valid.
/// Its view is empty.
impl View for MicrovmShutdown {
    type V = ();

    open spec fn view(&self) -> () {
        ()
    }
}

impl SaveRestoreSpecImpl for MicrovmShutdown {
    open spec fn saved_view(view: ()) -> NoSavedState {
        NoSavedState
    }

    open spec fn device_wf(&self) -> bool {
        true
    }
}

/// The snapshot-request port saves nothing (its pending request, if any,
/// is not saved); every state is valid.
/// Its view is empty.
impl View for MicrovmSnapshotRequest {
    type V = ();

    open spec fn view(&self) -> () {
        ()
    }
}

impl SaveRestoreSpecImpl for MicrovmSnapshotRequest {
    open spec fn saved_view(view: ()) -> NoSavedState {
        NoSavedState
    }

    open spec fn device_wf(&self) -> bool {
        true
    }
}

} // verus!
