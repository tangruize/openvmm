// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

//! [`StateUnit`] support for [`VmTimeKeeper`].

use inspect::InspectMut;
use mesh::Receiver;
use state_unit::StateRequest;
use state_unit::StateUnit;
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
struct KeeperUnit<'a>(#[inspect(mut)] &'a mut VmTimeKeeper);

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
pub async fn run_vmtime(keeper: &mut VmTimeKeeper, recv: Receiver<StateRequest>) {
    state_unit::run_unit(KeeperUnit(keeper), recv).await;
}
