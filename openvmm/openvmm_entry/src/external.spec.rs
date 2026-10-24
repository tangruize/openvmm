use vstd::prelude::*;

verus! {

// Opaque Verus declarations for types that appear in this crate's snapshot
// contracts. Verus knows only that these types exist; their contents are
// abstracted by the representation functions in the `.proof.rs` files.

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExOpenedSnapshot(openvmm_helpers::snapshot::restore::OpenedSnapshot);

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExSnapshotAttachment(openvmm_helpers::snapshot::microvm::SnapshotAttachment);

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExSnapshotMicrovmSandboxBlock(
    openvmm_helpers::snapshot::microvm::SnapshotMicrovmSandboxBlock,
);

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExMicrovmFilesystemSource<'a>(
    openvmm_helpers::snapshot::microvm::MicrovmFilesystemSource<'a>,
);

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExEgressPolicy(net_backend_resources::egress::EgressPolicy);

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExMicrovmNetworkConfig(openvmm_defs::microvm::MicrovmNetworkConfig);

// Local types that are opaque to Verus. Declared here rather than in place so
// that their fields, variants, and derived impls are never translated.

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExVmController(crate::vm_controller::VmController);

#[verifier::external_type_specification]
#[verifier::external_body]
pub(crate) struct ExTimeAbiRestore(crate::microvm::TimeAbiRestore);

#[verifier::external_type_specification]
#[verifier::external_body]
pub(crate) struct ExTimeAbiRestoreOptions<'a>(crate::microvm::TimeAbiRestoreOptions<'a>);

} // verus!
