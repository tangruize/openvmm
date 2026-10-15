use super::InitializedVm;
use super::LoadedVm;
use super::VmWorker;
use super::snapshot_spec::*;
use mesh::payload::message::ProtobufMessage;
use openvmm_defs::rpc::SnapshotSaveResponse;
use openvmm_defs::worker::SavedState;
use openvmm_defs::worker::VmWorkerParameters;
use vstd::prelude::*;

verus! {

pub uninterp spec fn initialized_vm_representation(
    vm: &InitializedVm,
) -> InitializedVmView;

pub uninterp spec fn loaded_vm_representation(
    vm: &LoadedVm,
) -> RuntimeVmView;

pub uninterp spec fn saved_state_representation(
    state: &SavedState,
) -> SavedStateEnvelopeView;

pub uninterp spec fn encoded_saved_state_representation(
    message: &ProtobufMessage,
) -> SavedStateEnvelopeView;

pub uninterp spec fn save_response_representation(
    response: &SnapshotSaveResponse,
) -> SnapshotSaveResponseView;

pub uninterp spec fn load_restore_request_view(
    vm: &InitializedVm,
    state: &Option<SavedState>,
    restore: &super::restore::SnapshotRestore,
) -> RestoreRequestView;

pub uninterp spec fn loaded_restore_request_view(
    vm: &LoadedVm,
    state: &SavedState,
) -> RestoreRequestView;

pub uninterp spec fn worker_restore_request_view(
    parameters: &VmWorkerParameters,
) -> RestoreRequestView;

impl View for InitializedVm {
    type V = InitializedVmView;

    open spec fn view(&self) -> InitializedVmView {
        initialized_vm_representation(self)
    }
}

impl View for LoadedVm {
    type V = RuntimeVmView;

    open spec fn view(&self) -> RuntimeVmView {
        loaded_vm_representation(self)
    }
}

impl View for VmWorker {
    type V = RuntimeVmView;

    open spec fn view(&self) -> RuntimeVmView {
        loaded_vm_representation(&self.vm)
    }
}

pub proof fn restore_core_implies_pre_execution(
    before: RuntimeVmView,
    request: RestoreRequestView,
    saved: SavedStateEnvelopeView,
    after: RuntimeVmView,
)
    requires
        request.restore_succeeds(before, saved, after),
    ensures
        !after.guest_can_execute,
        !after.externally_ready,
{
}

}
