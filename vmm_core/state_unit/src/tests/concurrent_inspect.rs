// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

//! Tests for inspects served while a state change is in flight
//! (`run_async_unit`, through the unit's inspector).

use crate::InspectConcurrently;
use crate::StateRequest;
use crate::StateUnit;
use crate::run_async_unit;
use futures::FutureExt;
use inspect::Inspect;
use inspect::InspectMut;
use mesh::rpc::RpcSend;
use pal_async::DefaultDriver;
use pal_async::async_test;
use pal_async::task::Spawn;
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use test_with_tracing::test;
use vmcore::save_restore::RestoreError;
use vmcore::save_restore::SaveError;
use vmcore::save_restore::SavedStateBlob;

/// A unit whose `start` waits until it is released.
struct SlowStartUnit {
    release: Option<mesh::OneshotReceiver<()>>,
    inspected_by_inspector: Arc<AtomicUsize>,
    inspected_by_unit: Arc<AtomicUsize>,
}

impl StateUnit for SlowStartUnit {
    async fn start(&mut self) -> anyhow::Result<()> {
        if let Some(release) = self.release.take() {
            release.await.unwrap();
        }
        Ok(())
    }

    async fn stop(&mut self) {}

    async fn reset(&mut self) -> anyhow::Result<()> {
        Ok(())
    }

    async fn save(&mut self) -> Result<Option<SavedStateBlob>, SaveError> {
        Ok(None)
    }

    async fn restore(&mut self, _state: SavedStateBlob) -> Result<(), RestoreError> {
        Err(RestoreError::SavedStateNotSupported)
    }
}

impl InspectMut for SlowStartUnit {
    fn inspect_mut(&mut self, req: inspect::Request<'_>) {
        self.inspected_by_unit.fetch_add(1, Ordering::SeqCst);
        req.respond();
    }
}

struct CountingInspector(Arc<AtomicUsize>);

impl Inspect for CountingInspector {
    fn inspect(&self, req: inspect::Request<'_>) {
        self.0.fetch_add(1, Ordering::SeqCst);
        req.respond();
    }
}

impl InspectConcurrently for SlowStartUnit {
    type Inspector = CountingInspector;

    fn inspector(&self) -> CountingInspector {
        CountingInspector(self.inspected_by_inspector.clone())
    }
}

/// Sends inspect requests to a unit's channel.
struct UnitTarget(mesh::Sender<StateRequest>);

impl Inspect for UnitTarget {
    fn inspect(&self, req: inspect::Request<'_>) {
        self.0.send(StateRequest::Inspect(req.defer()));
    }
}

#[async_test]
async fn test_inspect_during_state_change(driver: DefaultDriver) {
    let (release_send, release_recv) = mesh::oneshot();
    let by_inspector = Arc::new(AtomicUsize::new(0));
    let by_unit = Arc::new(AtomicUsize::new(0));
    let unit = SlowStartUnit {
        release: Some(release_recv),
        inspected_by_inspector: by_inspector.clone(),
        inspected_by_unit: by_unit.clone(),
    };
    let (send, recv) = mesh::channel();
    let task = driver.spawn("unit", run_async_unit(unit, recv));

    // Start the unit; its `start` blocks until released.
    let mut start = send.call_failable(StateRequest::Start, ()).boxed();
    assert!(futures::poll!(&mut start).is_pending());

    // An inspect while the start is in flight is answered, by the inspector.
    inspect::inspect("", &UnitTarget(send.clone()))
        .resolve()
        .await;
    assert_eq!(by_inspector.load(Ordering::SeqCst), 1);
    assert_eq!(by_unit.load(Ordering::SeqCst), 0);
    assert!(futures::poll!(&mut start).is_pending());

    // Released, the start completes.
    release_send.send(());
    start.await.unwrap();

    // With no state change in flight, an inspect goes to the unit.
    inspect::inspect("", &UnitTarget(send.clone()))
        .resolve()
        .await;
    assert_eq!(by_unit.load(Ordering::SeqCst), 1);
    assert_eq!(by_inspector.load(Ordering::SeqCst), 1);

    drop(send);
    task.await;
}
