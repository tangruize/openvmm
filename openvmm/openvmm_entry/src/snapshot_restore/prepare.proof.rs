use super::PreparedSnapshotRestore;
use mesh::payload::message::ProtobufMessage;
use openvmm_defs::snapshot_spec::*;
use openvmm_defs::worker::SharedMemoryFd;
use openvmm_helpers::snapshot::restore::OpenedSnapshot;
use vstd::prelude::*;

verus! {

pub uninterp spec fn opened_snapshot_representation(
    snapshot: &OpenedSnapshot,
) -> PublishedSnapshotView;

pub uninterp spec fn prepared_snapshot_restore_representation(
    prepared: &PreparedSnapshotRestore,
) -> PreparedSnapshotRestoreView;

pub uninterp spec fn prepared_saved_state_representation(
    saved_state: &ProtobufMessage,
) -> SavedStateEnvelopeView;

pub uninterp spec fn prepared_ram_representation(
    shared_memory: &SharedMemoryFd,
) -> RamView;

impl View for PreparedSnapshotRestore {
    type V = PreparedSnapshotRestoreView;

    open spec fn view(&self) -> PreparedSnapshotRestoreView {
        prepared_snapshot_restore_representation(self)
    }
}

}
