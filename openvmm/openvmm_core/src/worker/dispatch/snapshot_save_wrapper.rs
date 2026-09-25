// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

use super::LoadedVm;
use anyhow::Context;
use mesh::error::RemoteError;
use mesh::payload::message::ProtobufMessage;
use openvmm_defs::config::LoadMode;
use openvmm_defs::config::MachineProfile;
use state_unit::StateUnits;
use std::time::Duration;
use vstd::prelude::*;

#[cfg_attr(verus_keep_ghost, verus_verify(external_body))]
pub(super) fn snapshot_already_stopped() -> openvmm_defs::rpc::SnapshotQuiesceError {
    openvmm_defs::rpc::SnapshotQuiesceError::Rejected(RemoteError::new(anyhow::anyhow!(
        "VM is already stopped"
    )))
}

#[cfg_attr(verus_keep_ghost, verus_verify(external_body))]
pub(super) async fn quiesce_snapshot_state_units(
    state_units: &mut StateUnits,
    timeout: Duration,
) -> Result<(), openvmm_defs::rpc::SnapshotQuiesceError> {
    let quiesce = openvmm_defs::profile::ProfileSpan::start();
    if let Err(error) = state_units.quiesce_for_save(timeout).await {
        return Err(if error.has_uncertain_state() {
            openvmm_defs::rpc::SnapshotQuiesceError::Uncertain(RemoteError::new(error))
        } else {
            openvmm_defs::rpc::SnapshotQuiesceError::RollbackSafe(RemoteError::new(error))
        });
    }
    quiesce.complete("capture", "quiesce", Default::default());
    Ok(())
}

#[cfg_attr(verus_keep_ghost, verus_verify(external_body))]
pub(super) async fn capture_saved_state(
    vm: &mut LoadedVm,
) -> Result<openvmm_defs::worker::SavedState, openvmm_defs::rpc::SnapshotQuiesceError> {
    let save_state = openvmm_defs::profile::ProfileSpan::start();
    let saved_state = vm.save().await.map_err(|error| {
        openvmm_defs::rpc::SnapshotQuiesceError::RollbackSafe(RemoteError::new(error))
    })?;
    save_state.complete("capture", "save_state", Default::default());
    Ok(saved_state)
}

impl LoadedVm {
    pub(super) async fn quiesce_for_snapshot(
        &mut self,
        timeout: Duration,
    ) -> Result<openvmm_defs::rpc::SnapshotSaveResponse, openvmm_defs::rpc::SnapshotQuiesceError>
    {
        if self.inner.machine_profile != MachineProfile::Microvm {
            return Err(openvmm_defs::rpc::SnapshotQuiesceError::Rejected(
                RemoteError::new(anyhow::anyhow!(
                    "guest-requested snapshot quiesce requires the microVM profile"
                )),
            ));
        }

        let saved_state = self.capture_snapshot_state(timeout).await?;

        let mapped_memory_flush = openvmm_defs::profile::ProfileSpan::start();
        self.inner
            .memory_manager
            .flush_shared_file_backing()
            .context("failed to flush mapped guest RAM")
            .map_err(|error| {
                openvmm_defs::rpc::SnapshotQuiesceError::RollbackSafe(RemoteError::new(error))
            })?;
        mapped_memory_flush.complete("capture", "mapped_memory_flush", Default::default());

        let effective_command_line = match &self.inner.load_mode {
            LoadMode::Pvh { cmdline, .. } => cmdline.clone(),
            _ => {
                return Err(openvmm_defs::rpc::SnapshotQuiesceError::RollbackSafe(
                    RemoteError::new(anyhow::anyhow!(
                        "microVM snapshot has no effective PVH command line"
                    )),
                ));
            }
        };
        let tsc_frequency_hz = self
            .inner
            .partition
            .tsc_frequency_hz()
            .map_err(|error| {
                openvmm_defs::rpc::SnapshotQuiesceError::RollbackSafe(RemoteError::new(error))
            })?
            .ok_or_else(|| {
                openvmm_defs::rpc::SnapshotQuiesceError::RollbackSafe(RemoteError::new(
                    anyhow::anyhow!("backend does not expose a guest TSC frequency"),
                ))
            })?;
        let apic_frequency_hz = self
            .inner
            .partition
            .apic_frequency_hz()
            .map_err(|error| {
                openvmm_defs::rpc::SnapshotQuiesceError::RollbackSafe(RemoteError::new(error))
            })?
            .ok_or_else(|| {
                openvmm_defs::rpc::SnapshotQuiesceError::RollbackSafe(RemoteError::new(
                    anyhow::anyhow!("backend does not expose a local APIC frequency"),
                ))
            })?;
        let capture_wall_clock = self.snapshot_capture_wall_clock.ok_or_else(|| {
            openvmm_defs::rpc::SnapshotQuiesceError::RollbackSafe(RemoteError::new(
                anyhow::anyhow!("snapshot boundary has no wall-clock timestamp"),
            ))
        })?;

        Ok(openvmm_defs::rpc::SnapshotSaveResponse {
            state_unit_names: saved_state.inventory.clone(),
            saved_state: ProtobufMessage::new(saved_state),
            effective_command_line,
            tsc_frequency_hz,
            apic_frequency_hz,
            capture_wall_clock,
            cpu_contract: mesh::payload::encode(self.inner.partition.cpu_compatibility_contract()),
        })
    }
}
