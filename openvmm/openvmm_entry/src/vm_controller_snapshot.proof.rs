// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

// Engineer-owned representation bridges for guest-requested snapshot capture.

use super::GuestSnapshotAction;
use super::VmController;
use super::vm_controller_snapshot_spec::VmControllerSnapshotView;
use chipset_resources::microvm::MicrovmSnapshotScratchPolicy;
use vstd::prelude::*;

verus! {

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExVmController(VmController);

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExSnapshotScratchPolicy(MicrovmSnapshotScratchPolicy);

pub uninterp spec fn vm_controller_snapshot_view(
    controller: &VmController,
) -> VmControllerSnapshotView;

impl View for VmController {
    type V = VmControllerSnapshotView;

    closed spec fn view(&self) -> VmControllerSnapshotView {
        vm_controller_snapshot_view(self)
    }
}

pub closed spec fn snapshot_request_pre(
    controller: &VmController,
) -> bool {
    super::vm_controller_snapshot_spec::snapshot_atomic_invariant(
        controller@.protocol,
    )
    && controller@.protocol.phase
        == super::vm_controller_snapshot_spec::SnapshotPhase::BoundaryEstablished
    && controller@.boundary_owned
}

pub open spec fn snapshot_action_is_success(
    action: &GuestSnapshotAction,
) -> bool {
    match action {
        GuestSnapshotAction::Continue => false,
        GuestSnapshotAction::Terminate { exit_code } => *exit_code == 0,
    }
}

pub uninterp spec fn snapshot_scratch_policy_view(
    policy: &MicrovmSnapshotScratchPolicy,
) -> int;

} // verus!
