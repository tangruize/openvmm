// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

//! RPC types for communicating with the VM worker.

use crate::config::DeviceVtl;
use guid::Guid;
use mesh::CancelContext;
use mesh::MeshPayload;
use mesh::error::RemoteError;
use mesh::payload::message::ProtobufMessage;
use mesh::rpc::FailableRpc;
use mesh::rpc::Rpc;
use std::fmt;
use std::fs::File;
use std::time::Duration;
use vm_resource::Resource;
use vm_resource::kind::PciDeviceHandleKind;
use vm_resource::kind::VmbusDeviceHandleKind;

/// Verus specification of the VM worker's RPC channel.
#[cfg(verus_keep_ghost)]
pub mod spec {
    include!("rpc.spec.rs");
}

/// Trusted encode contract of protobuf messages.
#[cfg(verus_keep_ghost)]
// UNSAFETY: Verus only: `assume_specification` (trusted specifications of
// external functions) expands to `unsafe` declarations; no unsafe code runs.
#[allow(unsafe_code)]
mod trusted {
    include!("rpc_t.rs");
}

#[cfg(verus_keep_ghost)]
use mesh_rpc_verus::ReceiverAdditionalSpecFns;
#[cfg(verus_keep_ghost)]
use mesh_rpc_verus::SenderAdditionalSpecFns;
#[cfg(not(verus_keep_ghost))]
use vstd::prelude::Ghost;
#[cfg(not(verus_keep_ghost))]
use vstd::prelude::Tracked;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
use vstd::resource::Loc;
#[cfg(verus_keep_ghost)]
use vstd::resource::ghost_var::GhostVarAuth;

vstd::prelude::verus! {

/// Creates the VM worker's RPC channel (`VmController` to the worker). In
/// Verus builds it also creates the ledger of `VmRpc`'s protocol, recording
/// no lent halves, and attaches both its pieces to the receiver (the worker
/// takes them); the channel's constant is the ledger's id.
pub fn vm_rpc_channel() -> (result: (mesh::Sender<VmRpc>, mesh::Receiver<VmRpc>))
    ensures
        result.0.constant() == result.1.constant(),
        result.1.attached() matches Some((auth, var)) && auth.id() == result.1.constant()
            && var.id() == auth.id() && auth@ == Map::<u64, (Seq<char>, Loc, bool)>::empty(),
{
    let tracked (auth, var) = GhostVarAuth::<Map<u64, (Seq<char>, Loc, bool)>>::new(Map::empty());
    let ghost id = auth.id();
    mesh_rpc_verus::channel(Ghost(id), Tracked(()), Tracked((auth, var)))
}

} // verus!

#[derive(MeshPayload)]
pub enum VmRpc {
    Save(FailableRpc<(), ProtobufMessage>),
    /// Boundedly quiesce the VM and return saved state while leaving it stopped.
    QuiesceForSnapshot(Rpc<Duration, Result<SnapshotSaveResponse, SnapshotQuiesceError>>),
    /// Resume a VM after a rollback-safe snapshot failure before commit.
    ResumeAfterFailedSnapshot(FailableRpc<Duration, ()>),
    /// Release a post-OUT boundary without starting a snapshot transaction.
    ReleaseSnapshotBoundary(FailableRpc<(), ()>),
    Resume(FailableRpc<(), bool>),
    Pause(Rpc<(), bool>),
    ClearHalt(Rpc<(), bool>),
    Reset(FailableRpc<(), ()>),
    Nmi(Rpc<u32, ()>),
    AddVmbusDevice(FailableRpc<(DeviceVtl, Resource<VmbusDeviceHandleKind>), ()>),
    ConnectHvsock(FailableRpc<(CancelContext, Guid, DeviceVtl), unix_socket::UnixStream>),
    PulseSaveRestore(Rpc<(), Result<(), PulseSaveRestoreError>>),
    StartReloadIgvm(FailableRpc<File, ()>),
    CompleteReloadIgvm(FailableRpc<bool, ()>),
    ReadMemory(FailableRpc<(u64, usize), Vec<u8>>),
    WriteMemory(FailableRpc<(u64, Vec<u8>), ()>),
    /// Updates the command line parameters that will be passed to the boot shim
    /// on the *next* VM load. This will replace the existing command line parameters.
    UpdateCliParams(FailableRpc<String, ()>),
    /// Hot-add a PCIe device to a named port at runtime.
    /// Tuple is (port_name, device_resource).
    AddPcieDevice(FailableRpc<(String, Resource<PciDeviceHandleKind>), ()>),
    /// Hot-remove a PCIe device from a named port at runtime.
    RemovePcieDevice(FailableRpc<String, ()>),
    /// Hot-add a VPCI device to VTL0 at runtime with the supplied instance ID.
    AddVpciDevice(FailableRpc<(Guid, Resource<PciDeviceHandleKind>), ()>),
    /// Hot-remove a dynamically added VPCI device by instance ID.
    RemoveVpciDevice(FailableRpc<Guid, ()>),
    /// Dump VM state (VP registers + memory) to a `.vmrs` file.
    ///
    /// The worker pauses the VM internally, collects state, and restores
    /// the prior running state afterward. The caller provides an open file
    /// handle to write to (typically a temporary file that gets renamed
    /// into place on success).
    DumpState(FailableRpc<File, ()>),
}

/// State returned after a successful bounded snapshot quiesce.
#[derive(Debug, MeshPayload)]
pub struct SnapshotSaveResponse {
    /// Encoded VM saved state.
    pub saved_state: ProtobufMessage,
    /// Complete state-unit inventory in stable registration order.
    pub state_unit_names: Vec<String>,
    /// Effective Linux direct command line loaded into the guest.
    pub effective_command_line: String,
    /// The NVX time ABI records of the capture, which make the manifest
    /// version 6.
    pub time: crate::time_abi::TimeCapture,
}

/// Failure classification for a bounded snapshot quiesce/save operation.
#[derive(Debug, MeshPayload, thiserror::Error)]
pub enum SnapshotQuiesceError {
    /// The request was rejected before any state transition began.
    #[error("snapshot quiesce request was rejected")]
    Rejected(#[source] RemoteError),
    /// No uncertain transition occurred; the controller may request rollback.
    #[error("snapshot quiesce failed without uncertain state")]
    RollbackSafe(#[source] RemoteError),
    /// A unit may have partially transitioned; the VM must be terminated.
    #[error("snapshot quiesce left uncertain state")]
    Uncertain(#[source] RemoteError),
}

#[derive(Debug, MeshPayload, thiserror::Error)]
pub enum PulseSaveRestoreError {
    #[error("reset not supported")]
    ResetNotSupported,
    #[error("pulse save+restore failed")]
    Other(#[source] RemoteError),
    #[error("save and restore are unavailable for this machine profile")]
    UnsupportedMachineProfile,
}

impl From<anyhow::Error> for PulseSaveRestoreError {
    fn from(err: anyhow::Error) -> Self {
        Self::Other(RemoteError::new(err))
    }
}

impl fmt::Debug for VmRpc {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            VmRpc::Reset(_) => "Reset",
            VmRpc::Save(_) => "Save",
            VmRpc::QuiesceForSnapshot(_) => "QuiesceForSnapshot",
            VmRpc::ResumeAfterFailedSnapshot(_) => "ResumeAfterFailedSnapshot",
            VmRpc::ReleaseSnapshotBoundary(_) => "ReleaseSnapshotBoundary",
            VmRpc::Resume(_) => "Resume",
            VmRpc::Pause(_) => "Pause",
            VmRpc::ClearHalt(_) => "ClearHalt",
            VmRpc::Nmi(_) => "Nmi",
            VmRpc::AddVmbusDevice(_) => "AddVmbusDevice",
            VmRpc::ConnectHvsock(_) => "ConnectHvsock",
            VmRpc::PulseSaveRestore(_) => "PulseSaveRestore",
            VmRpc::StartReloadIgvm(_) => "StartReloadIgvm",
            VmRpc::CompleteReloadIgvm(_) => "CompleteReloadIgvm",
            VmRpc::ReadMemory(_) => "ReadMemory",
            VmRpc::WriteMemory(_) => "WriteMemory",
            VmRpc::UpdateCliParams(_) => "UpdateCliParams",
            VmRpc::AddPcieDevice(_) => "AddPcieDevice",
            VmRpc::RemovePcieDevice(_) => "RemovePcieDevice",
            VmRpc::AddVpciDevice(_) => "AddVpciDevice",
            VmRpc::RemoveVpciDevice(_) => "RemoveVpciDevice",
            VmRpc::DumpState(_) => "DumpState",
        };
        f.pad(s)
    }
}
