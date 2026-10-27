// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

// The request protocol of the VM worker's RPC channel, for human review.

use super::SnapshotSaveResponse;
use super::VmRpc;
use mesh::payload::message::ProtobufMessage;
use mesh_rpc_verus::MeshProtocol;
use vstd::prelude::*;

verus! {

/// The abstract value of an encoded protobuf message: the typed value it
/// encodes. A message that encodes `value: T` has view
/// `ProtobufMessageView::of(value)`. (As `SavedStateBlobView` for blobs,
/// without a type URL.) A message's is `encoded()`
/// (`ProtobufMessageAdditionalSpecFns`). The trusted contract is in
/// `rpc_t.rs`: `ProtobufMessage::new(value).encoded() == of(value)`, and
/// `of::<T>` is injective.
#[verifier::external_body]
pub struct ProtobufMessageView {
    _private: (),
}

impl ProtobufMessageView {
    /// The view of a message that encodes `value`, of type `T`.
    pub uninterp spec fn of<T>(value: T) -> ProtobufMessageView;
}

/// Declares `ProtobufMessage` to Verus, as an opaque type.
#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExProtobufMessage(ProtobufMessage);

/// Spec functions of a `ProtobufMessage`. (A trait of this crate: `View`
/// cannot be implemented here for mesh's type.)
pub trait ProtobufMessageAdditionalSpecFns {
    /// The value this message encodes.
    spec fn encoded(&self) -> ProtobufMessageView;
}

impl ProtobufMessageAdditionalSpecFns for ProtobufMessage {
    /// Trusted: the value this message encodes (see `rpc_t.rs`).
    uninterp spec fn encoded(&self) -> ProtobufMessageView;
}

/// Declares `SnapshotSaveResponse` (the reply to `QuiesceForSnapshot`) to
/// Verus, with its fields.
#[verifier::external_type_specification]
pub struct ExSnapshotSaveResponse(SnapshotSaveResponse);

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
