use super::GuestSnapshotAction;
use super::VmController;
use chipset_resources::microvm::MicrovmSnapshotScratchPolicy;
use openvmm_defs::snapshot_spec::*;
use vstd::prelude::*;

verus! {

pub uninterp spec fn snapshot_publication_representation(
    controller: &VmController,
    scratch_policy: &MicrovmSnapshotScratchPolicy,
    action: &GuestSnapshotAction,
) -> SnapshotPublicationView;

}
