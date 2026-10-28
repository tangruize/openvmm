// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

// The `StateUnit` spec of the vmbus server's and channels' state units, for human review.

use super::ChannelUnit;
use super::SimpleChannelUnit;
use super::VmbusServerUnit;
use state_unit::StateUnitSpec;
use vmbus_channel::channel::VmbusDevice;
use vmbus_channel::simple::SimpleVmbusDevice;
use vstd::prelude::*;
use vstd::resource::Loc;

verus! {

/// TODO (per-unit spec): where the unit keeps its share of its FracGhost,
/// and its invariant (`wf`). Uninterpreted until then; the impl's method
/// bodies are `external_body` meanwhile (`SNAPSHOT-SPEC-TRUSTED.md`).
impl StateUnitSpec for VmbusServerUnit {
    uninterp spec fn wf(&self) -> bool;

    uninterp spec fn id(&self) -> Loc;

    uninterp spec fn running(&self) -> bool;
}

/// A channel's unit is a proxy: its share of its FracGhost is kept by the
/// channel's task with the device (`vmbus_channel`'s `Device::keeps_share`),
/// and the unit passes the halves through its handle, over the channel's
/// internal channel. Its id is that channel's (the handle's `id()`); its
/// running flag is its own; it has no state of its own to relate (`wf`).
impl<T: 'static + VmbusDevice + ?Sized> StateUnitSpec for ChannelUnit<T> {
    closed spec fn wf(&self) -> bool {
        true
    }

    closed spec fn id(&self) -> Loc {
        self.0.id()
    }

    closed spec fn running(&self) -> bool {
        self.1@
    }
}

/// A channel's unit is a proxy: its share of its FracGhost is kept by the
/// channel's task with the device (`vmbus_channel`'s `Device::keeps_share`),
/// and the unit passes the halves through its handle, over the channel's
/// internal channel. Its id is that channel's (the handle's `id()`); its
/// running flag is its own; it has no state of its own to relate (`wf`).
impl<T: SimpleVmbusDevice> StateUnitSpec for SimpleChannelUnit<T> {
    closed spec fn wf(&self) -> bool {
        true
    }

    closed spec fn id(&self) -> Loc {
        self.0.id()
    }

    closed spec fn running(&self) -> bool {
        self.1@
    }
}

} // verus!
