// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

//! Deterministic lifecycle probes using the unchanged production clock bodies.

use futures::future::poll_fn;
use inspect::Inspect;
use pal_async::DefaultDriver;
use pal_async::DefaultPool;
use pal_async::driver::Driver;
use pal_async::task::{Runnable, Schedule, Spawn, TaskMetadata};
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};
use std::time::Duration;
use test_with_tracing::test;
use vmcore::vm_task::{BuildVmTaskDriver, TargetedDriver, VmTaskDriverSource};
use vmcore::vmtime::{SavedState, VmTime, VmTimeKeeper};

#[derive(Default)]
struct SecondarySchedule(Mutex<Vec<Runnable>>);

impl Schedule for SecondarySchedule {
    fn schedule(&self, runnable: Runnable) {
        self.0.lock().unwrap().push(runnable);
    }

    fn name(&self) -> Arc<str> {
        "controlled-secondary".into()
    }
}

#[derive(Clone)]
struct SecondaryDriver {
    io: DefaultDriver,
    schedule: Arc<SecondarySchedule>,
}

impl Inspect for SecondaryDriver {
    fn inspect(&self, req: inspect::Request<'_>) {
        req.ignore();
    }
}

impl Spawn for SecondaryDriver {
    fn scheduler(&self, _: &TaskMetadata) -> Arc<dyn Schedule> {
        self.schedule.clone()
    }
}

impl TargetedDriver for SecondaryDriver {
    fn spawner(&self) -> &dyn Spawn {
        self
    }

    fn driver(&self) -> &dyn Driver {
        &self.io
    }

    fn retarget_vp(&self, _: u32) {}
}

impl BuildVmTaskDriver for SecondaryDriver {
    type Driver = Self;
    type CurrentDriver = Self;

    fn build(&self, _: String, _: Option<u32>, _: bool) -> Self {
        self.clone()
    }

    fn build_current(&self) -> Self {
        self.clone()
    }
}

fn poll_once<F: Future>(future: Pin<&mut F>) -> Poll<F::Output> {
    future.poll(&mut Context::from_waker(Waker::noop()))
}

// IoPool::run_until polls its argument before draining its task queue.
// One pending turn, with a self-wake, gives the queue a deterministic turn.
fn drain(pool: &mut DefaultPool) {
    let mut first = true;
    pool.run_until(poll_fn(|cx| {
        if std::mem::take(&mut first) {
            cx.waker().wake_by_ref();
            Poll::Pending
        } else {
            Poll::Ready(())
        }
    }));
}

fn assert_saved(keeper: &VmTimeKeeper, time: VmTime) {
    assert_eq!(
        mesh::payload::encode(keeper.save()),
        mesh::payload::encode(SavedState::from_vmtime(time)),
    );
}

#[test]
fn healthy_reset_and_registration_after_reset() {
    let mut pool = DefaultPool::new();
    let driver = pool.driver();
    let old = VmTime::from_100ns(7);
    let target = VmTime::from_100ns(91);
    let mut keeper = VmTimeKeeper::new(&driver, old);
    let builder = keeper.builder().clone();
    let source = pool.run_until(builder.build(&driver)).unwrap();
    let access = source.access("before-reset");
    assert_eq!(access.now(), old);

    pool.run_until(keeper.restore(SavedState::from_vmtime(target)));
    assert_saved(&keeper, target);
    assert_eq!(access.now(), target);
    assert_eq!(access.host_time(target), None);
    let fresh = pool.run_until(builder.build(&driver)).unwrap();
    assert_eq!(fresh.access("after-reset").now(), target);
    println!("healthy: local save, existing access, and later registration = {target:?}");
}

#[test]
fn registration_before_reset_must_ack_before_reset_completes() {
    let mut primary = DefaultPool::new();
    let mut secondary = DefaultPool::new();
    let driver = primary.driver();
    let secondary_driver = secondary.driver();
    let old = VmTime::from_100ns(7);
    let target = VmTime::from_100ns(91);
    let mut keeper = VmTimeKeeper::new(&driver, old);
    let builder = keeper.builder().clone();
    let mut build = Box::pin(builder.build(&secondary_driver));
    assert!(poll_once(build.as_mut()).is_pending());
    drain(&mut primary);

    // New has returned the old state, but build has not consumed its response.
    let mut reset = Box::pin(keeper.restore(SavedState::from_vmtime(target)));
    assert!(poll_once(reset.as_mut()).is_pending());
    drain(&mut primary);
    assert!(poll_once(reset.as_mut()).is_pending());
    let Poll::Ready(Ok(source)) = poll_once(build.as_mut()) else {
        panic!("registration response should be ready");
    };
    drop(build);
    let access = source.access("overlapping-registration");
    assert_eq!(access.now(), old);
    assert!(poll_once(reset.as_mut()).is_pending());

    drain(&mut secondary);
    primary.run_until(reset);
    assert_saved(&keeper, target);
    assert_eq!(access.now(), target);
    assert_eq!(access.host_time(target), None);
    println!("registration-before: old observation only while reset pending; then {target:?}");
}

#[test]
fn discarded_secondary_error_leaves_a_surviving_stale_access() {
    let mut primary = DefaultPool::new();
    let driver = primary.driver();
    let schedule = Arc::new(SecondarySchedule::default());
    let secondary_driver = VmTaskDriverSource::new(SecondaryDriver {
        io: driver.clone(),
        schedule: schedule.clone(),
    })
    .simple();
    let old = VmTime::from_100ns(7);
    let target = VmTime::from_100ns(91);
    let mut keeper = VmTimeKeeper::new(&driver, old);
    let builder = keeper.builder().clone();
    let source = primary.run_until(builder.build(&secondary_driver)).unwrap();
    let access = source.access("survives-secondary");
    drop(source);

    let mut reset = Box::pin(keeper.restore(SavedState::from_vmtime(target)));
    assert!(poll_once(reset.as_mut()).is_pending());
    drain(&mut primary);
    // Primary has drained its queue and is waiting for the secondary RPC.
    assert!(poll_once(reset.as_mut()).is_pending());
    assert_eq!(access.now(), old);
    // Explicitly discard the unpolled production secondary task, not just its
    // executor handle: an executor's channel may retain queued runnables.
    let runnables = std::mem::take(&mut *schedule.0.lock().unwrap());
    assert_eq!(runnables.len(), 1);
    drop(runnables);
    primary.run_until(reset);

    assert_saved(&keeper, target);
    assert_eq!(access.now(), old);
    assert_eq!(access.host_time(old), None);
    let fresh = primary.run_until(builder.build(&driver)).unwrap();
    assert_eq!(fresh.access("primary-after-error").now(), target);
    println!(
        "discarded-error: reset returned normally; local/primary = {target:?}, surviving access = {old:?}"
    );
}

#[test]
fn cancelled_registration_does_not_supply_an_old_source() {
    let mut primary = DefaultPool::new();
    let secondary = DefaultPool::new();
    let driver = primary.driver();
    let secondary_driver = secondary.driver();
    let target = VmTime::from_100ns(91);
    let mut keeper = VmTimeKeeper::new(&driver, VmTime::from_100ns(7));
    let builder = keeper.builder().clone();
    let mut build = Box::pin(builder.build(&secondary_driver));
    assert!(poll_once(build.as_mut()).is_pending());
    drain(&mut primary);
    drop(build);
    primary.run_until(keeper.restore(SavedState::from_vmtime(target)));
    let fresh = primary.run_until(builder.build(&driver)).unwrap();
    assert_eq!(fresh.access("after-cancelled-build").now(), target);
    assert_saved(&keeper, target);
    println!("cancelled-registration: no source published; subsequent source = {target:?}");
}

#[test]
fn cancelled_reset_is_not_a_completed_installation() {
    let mut pool = DefaultPool::new();
    let driver = pool.driver();
    let old = VmTime::from_100ns(7);
    let target = VmTime::from_100ns(91);
    let mut keeper = VmTimeKeeper::new(&driver, old);
    let source = pool.run_until(keeper.builder().build(&driver)).unwrap();
    let access = source.access("cancelled-reset");
    let mut reset = Box::pin(keeper.restore(SavedState::from_vmtime(target)));
    assert!(poll_once(reset.as_mut()).is_pending());
    drop(reset);
    assert_saved(&keeper, target);
    assert_eq!(access.now(), old);
    drain(&mut pool);
    assert_eq!(access.now(), target);
    println!("cancelled-reset: local updated before installation; queued operation still executes");
}

#[test]
fn real_state_unit_advance_routes_the_duration_while_stopped() {
    let mut pool = DefaultPool::new();
    let driver = pool.driver();
    let mut keeper = VmTimeKeeper::new(&driver, VmTime::from_100ns(7));
    let source = pool.run_until(keeper.builder().build(&driver)).unwrap();
    let access = source.access("state-unit-route");
    let mut units = state_unit::StateUnits::new();
    let unit = units
        .add("vmtime")
        .spawn(driver, |recv| async move {
            vmm_core::vmtime_unit::run_vmtime(&mut keeper, recv).await;
            keeper
        })
        .unwrap();
    pool.run_until(units.advance_time(Duration::from_nanos(1234)))
        .unwrap();
    let expected = VmTime::from_100ns(19);
    assert_eq!(access.now(), expected);
    assert_eq!(access.host_time(expected), None);
    let keeper = pool.run_until(unit.remove());
    assert_saved(&keeper, expected);
    println!("production StateUnits -> run_vmtime -> advance: 7 ticks + 1234 ns = 19 ticks");
}
