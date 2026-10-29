// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

//! Partition unit support for snapshots: validating the instantiated VP prefix,
//! stopping VPs at a deferred I/O boundary for capture, and the NVX time ABI's
//! LAPIC timer checks and restore advance.

use super::Error;
use super::PartitionRequest;
use super::PartitionUnit;
use super::PartitionUnitParams;
use super::PartitionUnitRunner;
use super::StopGuard;
// The snapshot requests go through the mesh RPC wrapper, with placeholder
// ghosts (`PartitionRequest`'s protocol is not yet specified).
use mesh::error::RemoteError;
use mesh::rpc::FailableRpc;
use mesh_rpc_verus::call;
use mesh_rpc_verus::complete;
use mesh_rpc_verus::split;
use vstd::prelude::Ghost;
use vstd::prelude::Tracked;

/// Snapshot requests handled by the partition unit runner.
#[cfg_attr(verus_keep_ghost, vstd::prelude::verus_verify)]
pub(super) enum SnapshotRequest {
    StopVpsAtIoBoundary(FailableRpc<(mesh::OneshotSender<()>, mesh::OneshotReceiver<()>), ()>),
    #[cfg(guest_arch = "x86_64")]
    CheckTimers(FailableRpc<(), ()>),
    #[cfg(guest_arch = "x86_64")]
    AdvanceLapic(FailableRpc<(u64, u64), ()>),
    #[cfg(guest_arch = "x86_64")]
    OmitSavedTsc(mesh::rpc::Rpc<(), ()>),
}

/// Returns `data`, a VP's saved state, without its saved TSC.
#[cfg(guest_arch = "x86_64")]
pub(super) fn without_saved_tsc(
    data: vmcore::save_restore::SavedStateBlob,
) -> Result<vmcore::save_restore::SavedStateBlob, vmcore::save_restore::RestoreError> {
    let mut state: virt::vp::VpSavedState = data.parse()?;
    state.clear_tsc();
    Ok(vmcore::save_restore::SavedStateBlob::new(state))
}

/// Returns the number of VPs to instantiate, validated against the topology's
/// VP count.
pub(super) fn active_vp_count(params: &PartitionUnitParams<'_>) -> Result<u32, Error> {
    let vp_capacity = params.processor_topology.vp_count();
    let active_vp_count = params.active_vp_count.unwrap_or(vp_capacity);
    if !(1..=vp_capacity).contains(&active_vp_count) {
        return Err(Error::InvalidActiveVpCount {
            active: active_vp_count,
            capacity: vp_capacity,
        });
    }
    Ok(active_vp_count)
}

impl PartitionUnit {
    /// Stops VPs after queuing stop events but before completing deferred I/O.
    ///
    /// Failure is terminal: the caller must tear down the partition instead of
    /// attempting to resume it. An I/O-completion error retains the stop reference.
    pub async fn temporarily_stop_vps_at_io_boundary(
        &mut self,
        release_io: mesh::OneshotSender<()>,
        io_completed: mesh::OneshotReceiver<()>,
    ) -> anyhow::Result<StopGuard> {
        call(
            &self.req_send,
            |rpc| PartitionRequest::Snapshot(SnapshotRequest::StopVpsAtIoBoundary(rpc)),
            (release_io, io_completed),
            // PROOF TODO: replace these placeholders (`assume_new()`) with the real
            // ghosts: none sent (`None`), and the protocol's reply expectation; keep
            // the returned stop token in the `StopGuard`.
            Tracked::assume_new(),
            Ghost::assume_new(),
        )
        .recv_failable()
        .await?;
        Ok(StopGuard {
            0: self.req_send.clone(),
            // PROOF TODO: replace this placeholder (`assume_new()`) with the
            // real ghost: the stop token from the reply.
            #[cfg(verus_keep_ghost)]
            1: Tracked::assume_new(),
        })
    }

    /// Checks the LAPIC timer of every stopped vCPU for the NVX time ABI
    /// (`E_LAPIC_PERIODIC`, `E_LAPIC_TSC_DEADLINE`).
    #[cfg(guest_arch = "x86_64")]
    pub async fn check_one_shot_timers(&mut self) -> anyhow::Result<()> {
        call(
            &self.req_send,
            |rpc| PartitionRequest::Snapshot(SnapshotRequest::CheckTimers(rpc)),
            (),
            // OUT OF SCOPE: keep these `assume_new()`s (a time-ABI request: guest time
            // is out of scope; the protocol forbids verified code to send it).
            Tracked::assume_new(),
            Ghost::assume_new(),
        )
        .recv_failable()
        .await?;
        Ok(())
    }

    /// Advances the one-shot LAPIC timer of every stopped vCPU by
    /// `downtime_ns` at `apic_hz` and sets every vCPU's LAPIC state again,
    /// for the NVX time ABI.
    #[cfg(guest_arch = "x86_64")]
    pub async fn advance_lapic_timers(
        &mut self,
        downtime_ns: u64,
        apic_hz: u64,
    ) -> anyhow::Result<()> {
        call(
            &self.req_send,
            |rpc| PartitionRequest::Snapshot(SnapshotRequest::AdvanceLapic(rpc)),
            (downtime_ns, apic_hz),
            // OUT OF SCOPE: keep these `assume_new()`s (a time-ABI request: guest time
            // is out of scope; the protocol forbids verified code to send it).
            Tracked::assume_new(),
            Ghost::assume_new(),
        )
        .recv_failable()
        .await?;
        Ok(())
    }

    /// Makes every later restore drop the VPs' saved TSC values, which the
    /// NVX time ABI's synchronized TSC set supersedes, so no backend applies
    /// a per-VP TSC write before it.
    #[cfg(guest_arch = "x86_64")]
    pub async fn omit_saved_tsc(&mut self) {
        call(
            &self.req_send,
            |rpc| PartitionRequest::Snapshot(SnapshotRequest::OmitSavedTsc(rpc)),
            (),
            // OUT OF SCOPE: keep these `assume_new()`s (a time-ABI request: guest time
            // is out of scope; the protocol forbids verified code to send it).
            Tracked::assume_new(),
            Ghost::assume_new(),
        )
        .recv()
        .await
        .unwrap();
    }
}

impl PartitionUnitRunner {
    /// Handles a snapshot request from [`PartitionUnit`].
    pub(super) async fn handle_snapshot(&mut self, request: SnapshotRequest) {
        match request {
            SnapshotRequest::StopVpsAtIoBoundary(rpc) => {
                let ((release_io, io_completed), rpc) = split::<_, _, ()>(rpc);
                // Keep the stop reference on failure too: a failed
                // boundary must not be restarted before teardown.
                self.vp_stop_count += 1;
                let r = self
                    .vp_set
                    .stop_at_io_boundary(release_io, io_completed)
                    .await
                    .map_err(RemoteError::new);
                // PROOF TODO: replace this placeholder (`assume_new()`) with the real
                // ghost: a new stop token, from the runner's `stop_tokens.insert`.
                complete(rpc, r, Tracked::<()>::assume_new());
            }
            #[cfg(guest_arch = "x86_64")]
            SnapshotRequest::CheckTimers(rpc) => {
                let ((), rpc) = split::<_, _, ()>(rpc);
                let r = self
                    .vp_set
                    .check_one_shot_timers()
                    .await
                    .map_err(RemoteError::new);
                // OUT OF SCOPE: keep this `assume_new()` (a time-ABI request: guest time is
                // out of scope).
                complete(rpc, r, Tracked::<()>::assume_new());
            }
            #[cfg(guest_arch = "x86_64")]
            SnapshotRequest::AdvanceLapic(rpc) => {
                let ((downtime_ns, apic_hz), rpc) = split::<_, _, ()>(rpc);
                let r = self
                    .vp_set
                    .advance_lapic_timers(downtime_ns, apic_hz)
                    .await
                    .map_err(RemoteError::new);
                // OUT OF SCOPE: keep this `assume_new()` (a time-ABI request: guest time is
                // out of scope).
                complete(rpc, r, Tracked::<()>::assume_new());
            }
            #[cfg(guest_arch = "x86_64")]
            SnapshotRequest::OmitSavedTsc(rpc) => {
                let ((), rpc) = split::<_, _, ()>(rpc);
                self.omit_saved_tsc = true;
                // OUT OF SCOPE: keep this `assume_new()` (a time-ABI request: guest time is
                // out of scope).
                complete(rpc, (), Tracked::<()>::assume_new());
            }
        }
    }
}
