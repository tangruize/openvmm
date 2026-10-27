// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

//! The channel of a new state unit, with the unit's FracGhost.

use crate::StateRequest;
use mesh::Receiver;
use mesh::Sender;
use mesh_rpc_verus::*;
use vstd::prelude::*;

verus! {

/// Creates the channel for a new state unit. In Verus builds it also creates
/// the unit's FracGhost, value `Unknown`, and attaches one half to each end:
/// the sender's half goes to `StateUnits` (`UnitBuilder::build`), the
/// receiver's half to the unit. The channel's constant is the ghost's id.
pub fn unit_channel() -> (result: (Sender<StateRequest>, Receiver<StateRequest>))
    ensures
        result.0.attached() matches Some(h) && h.valid(result.0.constant(), 0.5real)
            && h@ == crate::spec::UnitSnapshot::Unknown,
        result.1.attached() matches Some(h) && h.valid(result.1.constant(), 0.5real)
            && h@ == crate::spec::UnitSnapshot::Unknown,
        result.0.constant() == result.1.constant(),
{
    let tracked mut send_half = vstd::resource::frac::FracGhost::new(
        crate::spec::UnitSnapshot::Unknown,
    );
    let tracked recv_half = send_half.split();
    let ghost id = send_half.id();
    channel(Ghost(id), Tracked(send_half), Tracked(recv_half))
}

} // verus!

/// Creates the channel for a new state unit whose state was created before
/// the channel, with the unit's FracGhost: the state's creator made the
/// FracGhost, keeps one half as the state's share, and passes the other,
/// `half`, here. It is attached to the sender, for `StateUnits`
/// (`UnitBuilder::build`); the receiver carries none. The channel's constant
/// is the ghost's id.
#[cfg_attr(verus_keep_ghost, verus_verify(external_body))]
#[cfg_attr(verus_keep_ghost, verus_spec(result =>
    with Tracked(half): Tracked<vstd::resource::frac::FracGhost<crate::spec::UnitSnapshot>>
    requires
        half.frac() == 0.5real,
    ensures
        result.0.attached() == Some(half),
        result.1.attached() is None,
        result.0.constant() == half.id(),
        result.1.constant() == half.id(),
))]
pub fn unit_channel_for() -> (Sender<StateRequest>, Receiver<StateRequest>) {
    mesh::channel()
}
