// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

// The request protocol of the VM worker's RPC channel, for human review.

use super::VmRpc;
use mesh_rpc_verus::MeshProtocol;
use vstd::prelude::*;

verus! {

/// Declares `VmRpc` to Verus, as an opaque type.
#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExVmRpc(VmRpc);

/// The VM worker's RPC channel (controller to worker). No ghosts travel on
/// it yet.
impl MeshProtocol for VmRpc {
    type Const = ();
    type GhostIn = ();
    type GhostOut = ();
    type SenderGhost = ();
    type ReceiverGhost = ();

    /// TODO (for the layer that verifies the controller): the protocol,
    /// that is, what each request's reply promises (e.g. for
    /// `QuiesceForSnapshot`, that an `Ok` response holds the worker's
    /// snapshot). Uninterpreted until then: verified code can neither send
    /// a `VmRpc` nor learn anything from receiving one.
    uninterp spec fn request_ok(k: (), msg: VmRpc, gin: ()) -> bool;
}

} // verus!
