// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

use pal_async::DefaultPool;
use state_unit::{SavedStateUnit, StateUnits};
use std::time::Duration;
use test_with_tracing::test;
use vmcore::save_restore::SavedStateBlob;
use vmcore::vmtime::{SavedState, VmTime, VmTimeKeeper};

fn exercise(initial_ticks: u64, supplied_ticks: Option<u64>) {
    DefaultPool::run_with(async |driver| {
        let mut keeper = VmTimeKeeper::new(&driver, VmTime::from_100ns(initial_ticks));
        let source = keeper.builder().build(&driver).await.unwrap();
        let access = source.access("saved-state-binding");
        let mut units = StateUnits::new();
        let unit = units
            .add("vmtime")
            .spawn(driver.clone(), async move |recv| {
                vmm_core::vmtime_unit::run_vmtime(&mut keeper, recv).await;
                keeper
            })
            .unwrap();

        assert_eq!(access.now().as_100ns(), initial_ticks);
        let inventory = vec!["vmtime".to_owned()];
        units.validate_inventory(&inventory).unwrap();
        let saved_units = supplied_ticks
            .map(|ticks| SavedStateUnit {
                name: "vmtime".to_owned(),
                state: SavedStateBlob::new(SavedState::from_vmtime(VmTime::from_100ns(ticks))),
            })
            .into_iter()
            .collect();

        units.restore(saved_units).await.unwrap();
        let restored_ticks = supplied_ticks.unwrap_or(initial_ticks);
        assert_eq!(access.now().as_100ns(), restored_ticks);
        let saved = units.save().await.unwrap();
        assert_eq!(saved.len(), 1);
        assert_eq!(saved[0].name, "vmtime");
        assert_eq!(
            mesh::payload::encode(saved[0].state.parse::<SavedState>().unwrap()),
            mesh::payload::encode(SavedState::from_vmtime(VmTime::from_100ns(restored_ticks))),
        );

        units.advance_time(Duration::from_nanos(1234)).await.unwrap();
        let advanced_ticks = restored_ticks + 12;
        assert_eq!(access.now().as_100ns(), advanced_ticks);
        let keeper = unit.remove().await;
        assert_eq!(
            mesh::payload::encode(keeper.save()),
            mesh::payload::encode(SavedState::from_vmtime(VmTime::from_100ns(advanced_ticks))),
        );
        println!(
            "initial={initial_ticks}, supplied={supplied_ticks:?}, inventory=accepted, \
             restore=Ok, local_and_source={restored_ticks}, after_1234ns={advanced_ticks}"
        );
    });
}

#[test]
fn omitted_vmtime_preserves_the_stopped_keeper() {
    exercise(0, None);
    exercise(37, None);
}

#[test]
fn supplied_vmtime_replaces_the_stopped_keeper() {
    exercise(0, Some(91));
    exercise(37, Some(91));
}
