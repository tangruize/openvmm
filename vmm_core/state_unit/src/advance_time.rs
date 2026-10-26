// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

//! Advancing guest-visible time by the snapshot downtime after restore.

use super::State;
use super::StateRequest;
use super::StateTransitionError;
use super::StateUnits;
use super::check;
use std::time::Duration;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

impl StateUnits {
    /// Advances guest-visible time on all stopped units in dependency order.
    /// Only units added with [`UnitBuilder::advances_time`](crate::UnitBuilder::advances_time)
    /// receive the request; the others pass through the operation without a
    /// round trip.
    #[cfg_attr(verus_keep_ghost, verus_verify(external_body))]
    #[cfg_attr(verus_keep_ghost, verus_spec(
        requires
            old(self).wf(),
            !old(self)@.running,
        ensures
            final(self).wf(),
            final(self)@.running == old(self)@.running,
            final(self)@.inventory == old(self)@.inventory,
            final(self)@.advances_time == old(self)@.advances_time,
            final(self)@.units.dom() == old(self)@.units.dom(),
            // Only the units that advance time may change.
            forall|name: Seq<char>| #[trigger] old(self)@.units.contains_key(name)
                && !old(self)@.advances_time.contains(name)
                ==> final(self)@.units[name] == old(self)@.units[name],
    ))]
    pub async fn advance_time(&mut self, duration: Duration) -> Result<(), StateTransitionError> {
        assert!(!self.running);
        let results = self
            .run_op(
                "advance_time",
                None,
                State::Stopped,
                State::AdvancingTime,
                State::Stopped,
                StateRequest::AdvanceTime,
                |_, unit| unit.advances_time.then_some(duration),
                |unit| &unit.dependencies,
            )
            .await;
        check(
            "advance_time",
            results
                .into_iter()
                .map(|(name, result)| (name, result.map_err(anyhow::Error::from))),
        )?;
        Ok(())
    }
}
