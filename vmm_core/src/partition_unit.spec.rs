// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

// The partition unit's spec (its request channel's protocol and its state
// unit), for human review.

use super::PartitionRequest;
use super::PartitionUnit;
use super::PartitionUnitRunner;
use super::StopGuard;
use mesh_rpc_verus::ReceiverAdditionalSpecFns;
use mesh_rpc_verus::SenderAdditionalSpecFns;
use vmcore::save_restore::spec::SavedStateBlobView;
use vmcore::save_restore::spec::UnitSnapshot;
use vmcore::save_restore::spec::ProtobufSaveRestoreSpec;
use vmm_core_defs::HaltReason;
use mesh_rpc_verus::MeshProtocol;
use mesh_rpc_verus::RpcAdditionalSpecFns;
use state_unit::StateUnitSpec;
use vstd::prelude::*;
use vstd::resource::Loc;
use vstd::resource::set::GhostSingleton;
use super::snapshot::SnapshotRequest;
use mesh::error::RemoteError;

verus! {

/// The partition unit's request channel (`PartitionUnit` to its runner
/// task): the protocol (design doc, "The partition unit's request
/// protocol"; decided by the user, 2026-10-30).
///
/// A **stop token** is a `GhostSingleton<u64>` of the runner's stop-token
/// authority (`PartitionUnitRunner::stop_tokens`), whose id is the
/// channel's constant `k`. Holding one is the guarantee that the VPs are
/// stopped: the runner keeps one outstanding token per stop reference
/// (`vp_stop_count`), and runs the VPs only when there is none.
///
/// - `StopVps`: nothing is sent; the reply carries a new stop token: the
///   VPs are stopped, and stay stopped while it is held.
/// - `StartVps` (one-way): a stop token is given back.
/// - `Snapshot(StopVpsAtIoBoundary(..))`: nothing is sent; the reply
///   carries a new stop token, also on `Err` (a failed boundary keeps its
///   stop reference). On `Ok` the deferred I/O has also completed after
///   every VP had a queued stop (a fact about the guest, not stated here).
/// - The time-ABI requests (`CheckTimers`, `AdvanceLapic`,
///   `OmitSavedTsc`): out of scope (guest time); verified code cannot send
///   them.
/// - `ClearHalt`, `SetInitialRegs`, `AcceptInitialPages`,
///   `BuildDumpPartitionState`: out of scope (boot, halts, dumps: not on
///   the snapshot path); verified code cannot send them.
impl MeshProtocol for PartitionRequest {
    type Const = Loc;
    /// A stop token, given back (`StartVps`).
    type GhostIn = Option<GhostSingleton<u64>>;
    /// A new stop token (`StopVps`, `StopVpsAtIoBoundary`).
    type GhostOut = Option<GhostSingleton<u64>>;
    type SenderGhost = ();
    type ReceiverGhost = ();

    open(super) spec fn request_ok(k: Loc, msg: PartitionRequest, gin: Self::GhostIn) -> bool {
        match msg {
            PartitionRequest::StopVps(rpc) => {
                &&& gin is None
                &&& forall|r: (), gout: Self::GhostOut|
                    gout matches Some(t) && t.id() == k ==> #[trigger] rpc.reply_ok(r, gout)
            },
            PartitionRequest::StartVps => gin matches Some(t) && t.id() == k,
            PartitionRequest::Snapshot(req) => match req {
                SnapshotRequest::StopVpsAtIoBoundary(rpc) => {
                    &&& gin is None
                    &&& forall|r: Result<(), RemoteError>, gout: Self::GhostOut|
                        gout matches Some(t) && t.id() == k ==> #[trigger] rpc.reply_ok(r, gout)
                },
                #[cfg(guest_arch = "x86_64")]
                SnapshotRequest::CheckTimers(_) => false,
                #[cfg(guest_arch = "x86_64")]
                SnapshotRequest::AdvanceLapic(_) => false,
                #[cfg(guest_arch = "x86_64")]
                SnapshotRequest::OmitSavedTsc(_) => false,
            },
            PartitionRequest::ClearHalt(_) => false,
            PartitionRequest::SetInitialRegs(_) => false,
            PartitionRequest::AcceptInitialPages(_) => false,
            #[cfg(feature = "dump")]
            PartitionRequest::BuildDumpPartitionState(_) => false,
        }
    }
}

/// The view of the partition unit's saved state (`save_restore::state::
/// Partition`): the partition's blob, each VP's index and blob, and the
/// halt reason, encoded (`mesh::payload::encode`).
pub struct PartitionSavedView {
    /// The partition's saved state (the hypervisor's VM-wide state).
    pub partition: SavedStateBlobView,
    /// Each VP's index and saved state (a set: see `state::Partition`'s
    /// view).
    pub vps: Set<(u32, SavedStateBlobView)>,
    /// The halt reason, if the VM halted, encoded.
    pub halt_reason: Option<Seq<u8>>,
}

impl PartitionSavedView {
    /// This saved state with only the VPs below index `k` (what a partition
    /// unit with `k` instantiated VPs keeps of it on restore).
    pub open spec fn restrict_vps(self, k: nat) -> PartitionSavedView {
        PartitionSavedView {
            vps: self.vps.filter(|p: (u32, SavedStateBlobView)| (p.0 as nat) < k),
            ..self
        }
    }

    // `encode_halt` (the encoding of a halt reason, uninterpreted) is in
    // `partition_unit_t.rs`.
}

impl PartitionUnitRunner {
    /// The view of the saved state the runner's `save` returns now: the
    /// partition's blob (`ProtobufSaveRestore`, `VmPartition`'s
    /// supertrait), the VPs' (as `VpSet`'s halves record them, see "The VP
    /// protocol"), and the halt reason.
    pub closed spec fn saved_view(&self) -> PartitionSavedView {
        PartitionSavedView {
            partition: self.partition.saved_blob(),
            vps: self.vp_set@.saved_vps().to_set(),
            halt_reason: match self.halt_reason {
                Some(r) => Some(PartitionSavedView::encode_halt(r)),
                None => None,
            },
        }
    }
}

impl StopGuard {
    /// Well formed: its stop token belongs to its channel's partition unit
    /// (so while it is held, that unit's VPs are stopped).
    pub closed spec fn wf(&self) -> bool {
        self.1@.id() == self.0.constant()
    }

    /// The partition unit whose VPs the guard holds stopped: the id of its
    /// request channel (`PartitionUnit::id`).
    pub closed spec fn partition_id(&self) -> Loc {
        self.0.constant()
    }
}

impl PartitionUnit {
    /// The id of the partition unit's request channel, which is its
    /// runner's stop-token authority (`PartitionRequest`'s protocol).
    pub closed spec fn id(&self) -> Loc {
        self.req_send.constant()
    }
}

/// The partition unit keeps its share of its FracGhost in its `share`
/// field; it runs while started. Its snapshot is its saved state's blob,
/// except after a restore with fewer VPs instantiated than the topology's
/// (MSHV's restore-time VP prefix, set at creation: `active_vp_count`):
/// then the recorded blob is the whole restored one, of which the unit
/// holds the VPs it instantiated (and `save` fails: a prefix cannot give
/// a complete snapshot). See the design doc, "The VP protocol", C3.
impl StateUnitSpec for PartitionUnitRunner {
    closed spec fn wf(&self) -> bool {
        // The request protocol's stop tokens: the authority's id is the
        // request channel's constant; one outstanding token per stop
        // reference; and while any is outstanding, the VPs are stopped.
        &&& self.stop_tokens@.id() == self.req_recv.constant()
        &&& self.stop_tokens@@.len() == self.vp_stop_count
        &&& self.vp_stop_count > 0 ==> !self.vp_set@.started
        &&& self.vp_set@.vps.len() >= self.vp_set@.capacity ==> self.share@.follows_rule(
            self.id(),
            UnitSnapshot::Saved(self.saved_view().blob()),
        )
        &&& self.vp_set@.vps.len() < self.vp_set@.capacity ==> exists|v: PartitionSavedView|
            #[trigger] v.restrict_vps(self.vp_set@.vps.len()) == self.saved_view()
                && self.share@.follows_rule(self.id(), UnitSnapshot::Saved(v.blob()))
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
