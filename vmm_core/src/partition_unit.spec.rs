// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

// The partition unit's spec (its request channel's protocol and its state
// unit), for human review.

use super::PartitionRequest;
use super::PartitionUnitRunner;
use vmcore::save_restore::spec::SavedStateBlobView;
use vmcore::save_restore::spec::UnitSnapshot;
use vmcore::save_restore::spec::ProtobufSaveRestoreSpec;
use vmm_core_defs::HaltReason;
use mesh_rpc_verus::MeshProtocol;
use state_unit::StateUnitSpec;
use vstd::prelude::*;
use vstd::resource::Loc;

verus! {

/// Declares `PartitionRequest` to Verus, as an opaque type.
#[verifier::external_type_specification]
#[verifier::external_body]
pub(super) struct ExPartitionRequest(PartitionRequest);

/// The partition unit's request channel (`PartitionUnit` to its runner
/// task). No ghosts travel on it yet.
impl MeshProtocol for PartitionRequest {
    type Const = ();
    type GhostIn = ();
    type GhostOut = ();
    type SenderGhost = ();
    type ReceiverGhost = ();

    /// TODO (for the layer that verifies quiesce and restore): the
    /// protocol, that is, what each request's reply promises (e.g. for
    /// `Snapshot(CheckTimers)`, that `Ok` means no one-shot LAPIC timer is
    /// armed). Uninterpreted until then: verified code can neither send a
    /// `PartitionRequest` nor learn anything from receiving one.
    uninterp spec fn request_ok(k: (), msg: PartitionRequest, gin: ()) -> bool;
}

/// The view of the partition unit's saved state (`save_restore::state::
/// Partition`): the partition's blob, each VP's index and blob, and the
/// halt reason, encoded (`mesh::payload::encode`).
pub struct PartitionSavedView {
    /// The partition's saved state (the hypervisor's VM-wide state).
    pub partition: SavedStateBlobView,
    /// Each VP's index and saved state.
    pub vps: Seq<(u32, SavedStateBlobView)>,
    /// The halt reason, if the VM halted, encoded.
    pub halt_reason: Option<Seq<u8>>,
}

impl PartitionSavedView {
    /// The encoding of a halt reason (TRUSTED: uninterpreted; what
    /// `mesh::payload::encode` gives, protobuf).
    pub uninterp spec fn encode_halt(reason: HaltReason) -> Seq<u8>;
}

impl PartitionUnitRunner {
    /// The view of the saved state the runner's `save` returns now: the
    /// partition's blob (`ProtobufSaveRestore`, `VmPartition`'s
    /// supertrait), the VPs' (as `VpSet`'s halves record them, see "The VP
    /// protocol"), and the halt reason.
    pub closed spec fn saved_view(&self) -> PartitionSavedView {
        PartitionSavedView {
            partition: self.partition.saved_blob(),
            vps: self.vp_set@.saved_vps(),
            halt_reason: match self.halt_reason {
                Some(r) => Some(PartitionSavedView::encode_halt(r)),
                None => None,
            },
        }
    }
}

/// The partition unit keeps its share of its FracGhost in its `share`
/// field. Its snapshot is its saved state's blob; it runs while started.
impl StateUnitSpec for PartitionUnitRunner {
    closed spec fn wf(&self) -> bool {
        &&& self.share@.follows_rule(self.id(), UnitSnapshot::Saved(self.saved_view().blob()))
        &&& self.share@.running == self.unit_started
    }

    closed spec fn id(&self) -> Loc {
        self.share@.frac.id()
    }

    closed spec fn running(&self) -> bool {
        self.unit_started
    }
}

} // verus!
