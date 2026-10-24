use vm_resource::ResourceKind;
use vstd::prelude::*;

verus! {

// Opaque Verus declarations for types that appear in snapshot
// contracts or in the fields of contract-visible types. Verus knows only that
// these types exist; their contents are abstracted by the representation
// functions in the `.proof.rs` files.

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExAnyhowError(anyhow::Error);

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExProtobufMessage(mesh::payload::message::ProtobufMessage);

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExSavedStateUnit(state_unit::SavedStateUnit);

#[verifier::external_trait_specification]
pub trait ExResourceKind: 'static + Send + Sync {
    type ExternalTraitSpecificationFor: ResourceKind;
}

#[verifier::external_type_specification]
#[verifier::external_body]
#[verifier::reject_recursive_types(K)]
pub struct ExResource<K: ResourceKind>(vm_resource::Resource<K>);

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExHypervisorKind(hypervisor_resources::HypervisorKind);

#[verifier::external_type_specification]
#[verifier::external_body]
#[verifier::reject_recursive_types(T)]
pub struct ExMeshSender<T>(mesh::Sender<T>);

#[verifier::external_type_specification]
#[verifier::external_body]
#[verifier::reject_recursive_types(T)]
pub struct ExMeshReceiver<T>(mesh::Receiver<T>);

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExHaltReason(vmm_core_defs::HaltReason);

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExMicrovmSnapshotBoundaryRequest(
    chipset_resources::microvm::MicrovmSnapshotBoundaryRequest,
);

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExMicrovmSnapshotScratchPolicy(chipset_resources::microvm::MicrovmSnapshotScratchPolicy);

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExFile(std::fs::File);

#[verifier::external_type_specification]
#[verifier::external_body]
#[verifier::reject_recursive_types(T)]
pub struct ExJoinHandle<T>(std::thread::JoinHandle<T>);

#[cfg(unix)]
#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExOwnedFd(std::os::fd::OwnedFd);

// Local types that are opaque to Verus. Declared here rather than in place so
// that their fields, variants, and derived impls are never translated.

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExConfig(crate::config::Config);

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExVmRpc(crate::rpc::VmRpc);

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExSnapshotQuiesceError(crate::rpc::SnapshotQuiesceError);

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExTimeCapture(crate::time_abi::TimeCapture);

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExRestoreTimeInput(crate::time_abi::RestoreTimeInput);

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExSnapshotRestoreGuards(crate::worker::SnapshotRestoreGuards);

} // verus!
