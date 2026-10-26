// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

// Declarations and the registry's lock invariant; no human review needed.

use super::Inner;
use super::InnerPred;
use super::StateRequest;
use super::mesh_rpc_t::MeshProtocol;
use super::mutex_t::MutexPredicate;
use super::spec::UnitSnapshot;
use vstd::prelude::*;
use vstd::resource::Loc;
use vstd::resource::frac::FracGhost;

verus! {

#[verifier::external_type_specification]
#[verifier::external_body]
pub(super) struct ExInner(Inner);

#[verifier::external_type_specification]
#[verifier::external_body]
pub(super) struct ExInnerPred(InnerPred);

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExStateRequest(StateRequest);

/// The ghosts that travel on a state unit's channel. The channel's constant is
/// the id of the unit's FracGhost.
impl MeshProtocol for StateRequest {
    type Const = Loc;
    /// A half lent or returned by the caller.
    type GhostIn = Option<FracGhost<UnitSnapshot>>;
    /// A half returned to the caller.
    type GhostOut = Option<FracGhost<UnitSnapshot>>;
    /// `StateUnits`' half, attached at creation (`unit_channel`).
    type SenderGhost = FracGhost<UnitSnapshot>;
    /// The unit's half, attached at creation.
    type ReceiverGhost = FracGhost<UnitSnapshot>;

    /// No rules yet: they come with the request protocol.
    open spec fn request_ok(k: Loc, msg: StateRequest, gin: Self::GhostIn) -> bool {
        true
    }
}

impl MutexPredicate<Inner> for InnerPred {
    /// No invariant yet; the FracGhost halves waiting in the registry will
    /// add one.
    open spec fn inv(v: Inner) -> bool {
        true
    }
}

} // verus!
