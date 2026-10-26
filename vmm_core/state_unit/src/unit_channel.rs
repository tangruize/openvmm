// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

//! The channel of a new state unit, with the unit's FracGhost.

use crate::StateRequest;
use crate::mesh_rpc_t::*;
use mesh::Receiver;
use mesh::Sender;
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
