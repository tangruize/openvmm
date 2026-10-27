// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

// The request protocol of the partition unit's request channel, for human
// review.

use super::PartitionRequest;
use mesh_rpc_verus::MeshProtocol;
use vstd::prelude::*;

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

} // verus!
