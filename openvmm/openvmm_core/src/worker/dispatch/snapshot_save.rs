// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

#[allow(dead_code, unused_imports)]
pub(super) mod snapshot_save_spec {
    include!("snapshot_save.spec.rs");
}

#[cfg(verus_keep_ghost)]
#[allow(dead_code, unused_imports)]
pub(super) mod snapshot_save_proof {
    include!("snapshot_save.proof.rs");
}

use super::LoadedVm;
use super::snapshot_save_wrapper::capture_saved_state;
use super::snapshot_save_wrapper::quiesce_snapshot_state_units;
use super::snapshot_save_wrapper::snapshot_already_stopped;
use std::time::Duration;
use vstd::prelude::*;

impl LoadedVm {
    /// Quiesces state units, serializes their state, and leaves the VM stopped.
    ///
    /// Backend metadata and snapshot publication are outside this boundary.
    #[cfg_attr(verus_keep_ghost, verus_verify(external_body))]
    #[cfg_attr(verus_keep_ghost, verus_spec(result =>
        requires
            snapshot_save_proof::snapshot_capture_request_is_valid(old(self)),
        ensures
            match result {
                Ok(saved_state) => (
                    snapshot_save_spec::snapshot_state_capture_success(
                        old(self)@,
                        snapshot_save_proof::saved_snapshot_state_view(&saved_state),
                        final(self)@,
                    )
                    && snapshot_save_proof::snapshot_stopped_representation(final(self))
                ),
                Err(error) => snapshot_save_proof::snapshot_capture_error_post(
                    old(self),
                    &error,
                    final(self),
                ),
            },
    ))]
    pub(super) async fn capture_snapshot_state(
        &mut self,
        timeout: Duration,
    ) -> Result<openvmm_defs::worker::SavedState, openvmm_defs::rpc::SnapshotQuiesceError> {
        if !self.running {
            return Err(snapshot_already_stopped());
        }

        quiesce_snapshot_state_units(&mut self.state_units, timeout).await?;
        self.running = false;

        capture_saved_state(self).await
    }
}
