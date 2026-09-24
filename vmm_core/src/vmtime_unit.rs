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
use vmcore::vmtime::VmTimeKeeper;
use vstd::prelude::*;

#[cfg(verus_keep_ghost)]
include!("vmtime_unit.proof.rs");

#[derive(InspectMut)]
#[inspect(transparent)]
#[verus_verify]
struct KeeperUnit<'a>(#[inspect(mut)] &'a mut VmTimeKeeper);

impl StateUnit for KeeperUnit<'_> {
    verus! {
        closed spec fn restore_requires(&self, state: &SavedStateBlob) -> bool {
            self.0.local_stopped_time() is Some
                && mesh::payload::protobuf::saved_time_values(state@.bytes) is Some
                && vmcore::vmtime::saved_type_url().accepts(state@.type_url)
        }
    }
    async fn start(&mut self) -> anyhow::Result<()> {
        self.0.start().await;
        Ok(())
    }

    async fn stop(&mut self) {
        self.0.stop().await;
    }

    async fn reset(&mut self) -> anyhow::Result<()> {
        self.0.reset().await;
        Ok(())
    }

    async fn save(&mut self) -> Result<Option<SavedStateBlob>, SaveError> {
        Ok(Some(SavedStateBlob::new(self.0.save())))
    }

    #[verus_verify]
    #[verus_spec(result =>
        ensures
            result is Ok,
            final(self).restore_completed(old(self), &state),
            cfg!(verus_keeper_negative_completion) ==> result is Err,
    )]
    async fn restore(&mut self, state: SavedStateBlob) -> Result<(), RestoreError> {
        self.0
            .restore({
                proof_with! { native_specialized_call(vmcore::vmtime::parse_saved_blob_body) }
                state.parse()
            }?)
            .await;
        Ok(())
    }

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
