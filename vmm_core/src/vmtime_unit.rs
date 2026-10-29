// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

//! [`StateUnit`] support for [`VmTimeKeeper`].

use inspect::InspectMut;
use mesh::Receiver;
#[cfg(verus_keep_ghost)]
use mesh_rpc_verus::ReceiverAdditionalSpecFns;
use state_unit::StateRequest;
use state_unit::StateUnit;
#[cfg(verus_keep_ghost)]
use state_unit::StateUnitSpec;
#[cfg(verus_keep_ghost)]
use vmcore::device_state::spec::DeviceGhostState;
use vmcore::save_restore::RestoreError;
use vmcore::save_restore::SaveError;
use vmcore::save_restore::SavedStateBlob;
#[cfg(verus_keep_ghost)]
use vmcore::save_restore::spec::UnitSnapshot;
use vmcore::vmtime::VmTimeKeeper;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
use vstd::resource::frac::FracGhost;

/// The `StateUnit` spec of the VM time keeper's state unit.
#[cfg(verus_keep_ghost)]
mod spec {
    include!("vmtime_unit.spec.rs");
}

#[cfg(verus_keep_ghost)]
mod proof {
    include!("vmtime_unit.proof.rs");
}

#[derive(InspectMut)]
#[inspect(transparent)]
struct KeeperUnit<'a>(
    #[inspect(mut)] &'a mut VmTimeKeeper,
    /// The unit's share of its FracGhost (Verus only).
    #[cfg(verus_keep_ghost)]
    #[inspect(skip)]
    Tracked<DeviceGhostState>,
);

#[cfg_attr(verus_keep_ghost, verus_verify)]
impl<'a> KeeperUnit<'a> {
    /// The unit for `keeper`, which is stopped, taking its half from `recv`,
    /// its channel's receiver.
    #[cfg_attr(verus_keep_ghost, verus_verify(external_body))]
    #[cfg_attr(verus_keep_ghost, verus_spec(unit =>
        requires
            !keeper@.running,
            old(recv).attached() matches Some(h) && h.valid(old(recv).constant(), 0.5real),
        ensures
            unit.wf(),
            unit.id() == old(recv).constant(),
            !unit.running(),
            final(recv).constant() == old(recv).constant(),
    ))]
    fn new(keeper: &'a mut VmTimeKeeper, recv: &mut Receiver<StateRequest>) -> Self {
        let _ = recv;
        // PROOF TODO: replace this placeholder (`assume_new()`) with the real
        // ghost: the half taken from `recv` (`take_receiver_ghost`).
        #[cfg(verus_keep_ghost)]
        return Self(keeper, Tracked::assume_new());
        #[cfg(not(verus_keep_ghost))]
        return Self(keeper);
    }
}

#[cfg_attr(verus_keep_ghost, verus_verify)]
impl StateUnit for KeeperUnit<'_> {
    #[cfg_attr(verus_keep_ghost, verus_verify(external_body))]
    #[cfg_attr(verus_keep_ghost, verus_spec(with Tracked(g): Tracked<&mut Option<FracGhost<UnitSnapshot>>>))]
    async fn start(&mut self) -> anyhow::Result<()> {
        self.0.start().await;
        Ok(())
    }

    #[cfg_attr(verus_keep_ghost, verus_verify(external_body))]
    #[cfg_attr(verus_keep_ghost, verus_spec(with Tracked(g): Tracked<&mut Option<FracGhost<UnitSnapshot>>>))]
    async fn stop(&mut self) {
        self.0.stop().await;
    }

    #[cfg_attr(verus_keep_ghost, verus_verify(external_body))]
    #[cfg_attr(verus_keep_ghost, verus_spec(with Tracked(g): Tracked<&mut Option<FracGhost<UnitSnapshot>>>))]
    async fn reset(&mut self) -> anyhow::Result<()> {
        self.0.reset().await;
        Ok(())
    }

    #[cfg_attr(verus_keep_ghost, verus_verify(external_body))]
    #[cfg_attr(verus_keep_ghost, verus_spec(with Tracked(g): Tracked<&mut Option<FracGhost<UnitSnapshot>>>))]
    async fn save(&mut self) -> Result<Option<SavedStateBlob>, SaveError> {
        Ok(Some(SavedStateBlob::new(self.0.save())))
    }

    #[cfg_attr(verus_keep_ghost, verus_verify(external_body))]
    #[cfg_attr(verus_keep_ghost, verus_spec(with Tracked(g): Tracked<&mut Option<FracGhost<UnitSnapshot>>>))]
    async fn restore(&mut self, state: SavedStateBlob) -> Result<(), RestoreError> {
        self.0.restore(state.parse()?).await;
        Ok(())
    }

    #[cfg_attr(verus_keep_ghost, verus_verify(external_body))]
    #[cfg_attr(verus_keep_ghost, verus_spec(with Tracked(g): Tracked<&mut Option<FracGhost<UnitSnapshot>>>))]
    async fn advance_time(&mut self, duration: std::time::Duration) -> anyhow::Result<()> {
        self.0.advance(duration).await;
        Ok(())
    }
}

/// Runs the VM time keeper, responding to state changes from `recv`, until
/// `recv` is closed.
pub async fn run_vmtime(keeper: &mut VmTimeKeeper, mut recv: Receiver<StateRequest>) {
    let unit = KeeperUnit::new(keeper, &mut recv);
    state_unit::run_unit(unit, recv).await;
}
