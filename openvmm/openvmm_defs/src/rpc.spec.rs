// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

// The request protocol of the VM worker's RPC channel, for human review.

use super::SnapshotQuiesceError;
use super::SnapshotSaveResponse;
use super::VmRpc;
use mesh::payload::message::ProtobufMessage;
use crate::worker::SavedState;
use mesh::error::RemoteError;
use mesh_rpc_verus::MeshProtocol;
use mesh_rpc_verus::RpcAdditionalSpecFns;
use state_unit::spec::LentHalves;
use vm_resource::ResourceKind;
use vstd::prelude::*;
use vstd::resource::Loc;
use vstd::resource::ghost_var::GhostVar;
use vstd::resource::ghost_var::GhostVarAuth;

verus! {

/// The abstract value of an encoded protobuf message: the typed value it
/// encodes. A message that encodes `value: T` has view
/// `ProtobufMessageView::of(value)`. (As `SavedStateBlobView` for blobs,
/// without a type URL.) A message's is `encoded()`
/// (`ProtobufMessageAdditionalSpecFns`). The trusted contract is in
/// `rpc_t.rs`: `ProtobufMessage::new(value).encoded() == of(value)`, and
/// `of::<T>` is injective.
#[verifier::external_body]
pub struct ProtobufMessageView {
    _private: (),
}

// `ProtobufMessageView::of` (the encoding, uninterpreted) is in `rpc_t.rs`.

/// Declares `ProtobufMessage` to Verus by name only (`external_body`).
#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExProtobufMessage(ProtobufMessage);

/// Spec functions of a `ProtobufMessage`. (A trait of this crate: `View`
/// cannot be implemented here for mesh's type.)
pub trait ProtobufMessageAdditionalSpecFns {
    /// The value this message encodes.
    spec fn encoded(&self) -> ProtobufMessageView;
}

// Its implementation for `ProtobufMessage` (uninterpreted) is in `rpc_t.rs`.

/// Declares `SnapshotSaveResponse` (the reply to `QuiesceForSnapshot`) to
/// Verus, with its fields.
#[verifier::external_type_specification]
pub struct ExSnapshotSaveResponse(SnapshotSaveResponse);

/// Declares `VmRpc` to Verus, with its variants (the protocol matches on
/// them).
#[verifier::external_type_specification]
pub struct ExVmRpc(VmRpc);

// The types inside `VmRpc`'s variants that Verus did not know. Verus must
// know every type inside an enum's variants before code can `match` on the
// enum (as `request_ok` does), even for variants the protocol maps to
// `false`. Each is declared by name only (`external_body`: Verus cannot see
// its fields), which says nothing about how it behaves.

/// Declares `DeviceVtl` to Verus by name only (`external_body`).
#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExDeviceVtl(crate::config::DeviceVtl);

/// Declares `PulseSaveRestoreError` to Verus by name only (`external_body`).
#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExPulseSaveRestoreError(super::PulseSaveRestoreError);

/// Declares `Guid` to Verus by name only (`external_body`).
#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExGuid(guid::Guid);

/// Declares `mesh::CancelContext` to Verus by name only (`external_body`).
#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExCancelContext(mesh::CancelContext);

/// Declares `std::fs::File` to Verus by name only (`external_body`).
#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExFile(std::fs::File);

/// Declares `UnixStream` to Verus by name only (`external_body`).
#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExUnixStream(std::os::unix::net::UnixStream);

/// Declares `vm_resource::ResourceKind` to Verus (no claims).
#[verifier::external_trait_specification]
pub trait ExResourceKind: 'static + Send + Sync {
    type ExternalTraitSpecificationFor: ResourceKind;
}

/// Declares `vm_resource::Resource` to Verus by name only (`external_body`).
#[verifier::external_type_specification]
#[verifier::external_body]
#[verifier::reject_recursive_types(K)]
pub struct ExResource<K: ResourceKind>(vm_resource::Resource<K>);

/// Declares `VmbusDeviceHandleKind` to Verus by name only (`external_body`).
#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExVmbusDeviceHandleKind(vm_resource::kind::VmbusDeviceHandleKind);

/// Declares `PciDeviceHandleKind` to Verus by name only (`external_body`).
#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExPciDeviceHandleKind(vm_resource::kind::PciDeviceHandleKind);

/// The lent state units' halves, as they travel on `VmRpc` (worker to
/// controller with a snapshot, and back for a rollback), with the
/// protocol's ledger piece recording whose halves they are.
pub tracked struct SnapshotHalves {
    /// The ledger's `GhostVar` (the worker keeps its `GhostVarAuth`): it
    /// records the lent units, so the worker knows that halves coming back
    /// are the ones it lent.
    pub tracked ledger: GhostVar<Map<u64, (Seq<char>, Loc, bool)>>,
    /// The halves.
    pub tracked lent: LentHalves,
}

impl SnapshotHalves {
    /// Well formed on the channel with constant `k`: the ledger piece is
    /// the channel's and records the lent units; the halves are well formed.
    pub open spec fn wf(&self, k: Loc) -> bool {
        &&& self.ledger.id() == k
        &&& self.ledger@ == self.lent.units
        &&& self.lent.wf()
    }
}

/// The VM worker's RPC channel (`VmController` to the worker's
/// `LoadedVm::run`): the protocol (design doc, "The VM worker's request
/// protocol"; decided by the user, 2026-10-30).
///
/// The snapshot requests move the state units' halves (`LentHalves`)
/// between the worker and the controller. While the controller holds them,
/// no state unit can change its state: the units are stopped, and the
/// worker's `StateUnits` holds none of the halves it would need to restart,
/// reset, or restore them. So the VM cannot run between the snapshot and
/// the controller's copy of the disk.
///
/// The **ledger** is a `GhostVarAuth` / `GhostVar` pair created with the
/// channel (`vm_rpc_channel`), both attached to the receiver; its id is the
/// channel's constant `k`. The worker keeps the `GhostVarAuth`; the
/// `GhostVar` travels with the halves (`SnapshotHalves`) and records whose
/// they are.
///
/// - `QuiesceForSnapshot`: nothing is sent. On `Ok`, the reply carries the
///   halves, and the response's saved state encodes a well-formed
///   `SavedState` whose units are what the halves record, and whose
///   inventory is the response's unit names. On `Err(RollbackSafe)`, the
///   reply carries the halves (the units are stopped; the controller may
///   roll back). On `Rejected` (nothing changed) and `Uncertain` (the VM
///   must be terminated), nothing.
/// - `ResumeAfterFailedSnapshot`: the halves come back; nothing in the
///   reply.
/// - `ReleaseSnapshotBoundary`: nothing either way (no halves are out:
///   sent after a rejected request or a failed preflight).
///
/// The other 19 variants are not specified yet: `request_ok` is `false`,
/// so verified code cannot send them (unverified code still can). The
/// microVM worker rejects some of them (`filter_vm_rpc`;
/// `snapshot_rpc::filter` during a snapshot boundary):
///
/// - Next to specify (the microVM accepts them outside a snapshot
///   boundary): `Pause`, `Resume`, `Reset`.
/// - Always rejected by the microVM worker: `Save`, `PulseSaveRestore`.
/// - Accepted by the microVM worker outside a snapshot boundary, not
///   planned yet: `ClearHalt`, `Nmi`, `ReadMemory` (also during a
///   boundary), `WriteMemory`, `AddVmbusDevice`, `ConnectHvsock`,
///   `StartReloadIgvm`, `CompleteReloadIgvm`, `UpdateCliParams`,
///   `AddPcieDevice`, `RemovePcieDevice`, `AddVpciDevice`,
///   `RemoveVpciDevice`, `DumpState`.
impl MeshProtocol for VmRpc {
    /// The ledger's id.
    type Const = Loc;
    /// The halves, given back (`ResumeAfterFailedSnapshot`).
    type GhostIn = Option<SnapshotHalves>;
    /// The halves, lent (`QuiesceForSnapshot`).
    type GhostOut = Option<SnapshotHalves>;
    type SenderGhost = ();
    /// The ledger, both pieces, no halves lent (the worker takes it).
    type ReceiverGhost = (
        GhostVarAuth<Map<u64, (Seq<char>, Loc, bool)>>,
        GhostVar<Map<u64, (Seq<char>, Loc, bool)>>,
    );

    open spec fn request_ok(k: Loc, msg: VmRpc, gin: Self::GhostIn) -> bool {
        match msg {
            VmRpc::QuiesceForSnapshot(rpc) => {
                &&& gin is None
                &&& forall|
                    r: Result<SnapshotSaveResponse, SnapshotQuiesceError>,
                    gout: Self::GhostOut,
                |
                    match r {
                        Ok(response) => gout matches Some(h) && h.wf(k)
                            && exists|saved: SavedState| {
                                &&& saved.wf()
                                &&& #[trigger] ProtobufMessageView::of(saved)
                                    == response.saved_state.encoded()
                                &&& saved@.units == h.lent.saved_units()
                                &&& saved@.inventory == response.state_unit_names@.map_values(
                                    |name: String| name@,
                                )
                            },
                        Err(SnapshotQuiesceError::RollbackSafe(_)) => gout matches Some(h)
                            && h.wf(k),
                        Err(_) => gout is None,
                    } ==> #[trigger] rpc.reply_ok(r, gout)
            },
            VmRpc::ResumeAfterFailedSnapshot(rpc) => {
                &&& gin matches Some(h) && h.wf(k)
                &&& forall|r: Result<(), RemoteError>, gout: Self::GhostOut|
                    gout is None ==> #[trigger] rpc.reply_ok(r, gout)
            },
            VmRpc::ReleaseSnapshotBoundary(rpc) => {
                &&& gin is None
                &&& forall|r: Result<(), RemoteError>, gout: Self::GhostOut|
                    gout is None ==> #[trigger] rpc.reply_ok(r, gout)
            },
            // Next to specify.
            VmRpc::Pause(_) => false,
            VmRpc::Resume(_) => false,
            VmRpc::Reset(_) => false,
            // Always rejected by the microVM worker.
            VmRpc::Save(_) => false,
            VmRpc::PulseSaveRestore(_) => false,
            // Not planned yet.
            VmRpc::ClearHalt(_) => false,
            VmRpc::Nmi(_) => false,
            VmRpc::ReadMemory(_) => false,
            VmRpc::WriteMemory(_) => false,
            VmRpc::AddVmbusDevice(_) => false,
            VmRpc::ConnectHvsock(_) => false,
            VmRpc::StartReloadIgvm(_) => false,
            VmRpc::CompleteReloadIgvm(_) => false,
            VmRpc::UpdateCliParams(_) => false,
            VmRpc::AddPcieDevice(_) => false,
            VmRpc::RemovePcieDevice(_) => false,
            VmRpc::AddVpciDevice(_) => false,
            VmRpc::RemoveVpciDevice(_) => false,
            VmRpc::DumpState(_) => false,
        }
    }
}

} // verus!
