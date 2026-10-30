// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

use crate::VmmChipsetDevice;
use async_trait::async_trait;
#[cfg(verus_keep_ghost)]
use closeable_mutex::LockInvariant;
use futures::FutureExt;
use futures::task::ArcWake;
use futures::task::waker_ref;
use inspect::InspectMut;
#[cfg(verus_keep_ghost)]
use mesh_rpc_verus::MeshProtocol;
#[cfg(verus_keep_ghost)]
use mesh_rpc_verus::ReceiverAdditionalSpecFns;
use state_unit::RequestGhost;
use state_unit::StateRequest;
use state_unit::StateUnit;
#[cfg(verus_keep_ghost)]
use state_unit::StateUnitSpec;
use std::sync::Arc;
use std::task::Context;
use vmcore::save_restore::RestoreError;
use vmcore::save_restore::SaveError;
use vmcore::save_restore::SavedStateBlob;
#[cfg(verus_keep_ghost)]
use vmcore::save_restore::spec::UnitSnapshot;
use vmcore::slim_event::SlimEvent;
use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
use vstd::resource::frac::FracGhost;

/// The `StateUnit` spec of a chipset device's state unit.
#[cfg(verus_keep_ghost)]
mod spec {
    include!("state_unit.spec.rs");
}

#[cfg(verus_keep_ghost)]
mod proof {
    include!("state_unit.proof.rs");
}

/// The state unit of a chipset device. Its share of its FracGhost is kept
/// with the device, in the device's lock (`vmcore::device_state::DeviceSlot`).
#[cfg_attr(verus_keep_ghost, verus_verify)]
pub struct ArcMutexChipsetDeviceUnit {
    device: Arc<vmcore::device_state::DeviceMutex<dyn DynDevice>>,
    poll_event: Arc<PollEvent>,
    running: bool,
    omit_saved_state: bool,
}

#[cfg_attr(verus_keep_ghost, verus_verify)]
impl InspectMut for ArcMutexChipsetDeviceUnit {
    #[cfg_attr(verus_keep_ghost, verus_verify(external_body))]
    fn inspect_mut(&mut self, req: inspect::Request<'_>) {
        self.device.lock().inspect_mut(req);
    }
}

/// The device's save and restore, for [`DynDevice`]. (`DeviceSnapshot`: in
/// Verus builds, the device's saved blob, which its lock's invariant names.)
///
/// Verus: a trait of its own, without `async_trait`, so that its contracts
/// can be stated and its blanket impl checked (`async_trait`'s
/// `dyn Future + Send` is not supported by Verus).
#[cfg_attr(verus_keep_ghost, verus_verify)]
trait DynDeviceState: vmcore::device_state::DeviceSnapshot {
    /// Saves the device: returns its blob and changes nothing.
    #[cfg_attr(verus_keep_ghost, verus_spec(result =>
        requires
            old(self).device_wf(),
        ensures
            final(self).device_wf(),
            final(self).saved_blob() == old(self).saved_blob(),
            result matches Ok(blob) ==> blob@ == old(self).saved_blob(),
    ))]
    fn save(&mut self) -> Result<SavedStateBlob, SaveError>;

    /// Restores the device: on success, its blob is `state`'s.
    #[cfg_attr(verus_keep_ghost, verus_spec(result =>
        requires
            old(self).device_wf(),
        ensures
            final(self).device_wf(),
            result is Ok ==> final(self).saved_blob() == state@,
    ))]
    fn restore(&mut self, state: SavedStateBlob) -> Result<(), RestoreError>;
}

// Checked against `DynDeviceState`'s contracts: the device's
// `ProtobufSaveRestore` meets them.
#[cfg_attr(verus_keep_ghost, verus_verify)]
impl<T: VmmChipsetDevice> DynDeviceState for T {
    fn save(&mut self) -> Result<SavedStateBlob, SaveError> {
        vmcore::save_restore::ProtobufSaveRestore::save(self)
    }

    fn restore(&mut self, state: SavedStateBlob) -> Result<(), RestoreError> {
        vmcore::save_restore::ProtobufSaveRestore::restore(self, state)
    }
}

/// Object-safe trait for the subset of [`VmmChipsetDevice`] that we use here.
#[async_trait]
trait DynDevice: InspectMut + Send + DynDeviceState {
    async fn start(&mut self) -> anyhow::Result<()>;
    async fn stop(&mut self);
    async fn quiesce_input(&mut self) -> anyhow::Result<()>;
    async fn resume_input(&mut self) -> anyhow::Result<()>;
    async fn reset(&mut self);
    async fn advance_time(&mut self, duration: std::time::Duration) -> anyhow::Result<()>;
    fn poll_device(&mut self, cx: &mut Context<'_>);
}

#[async_trait]
impl<T: VmmChipsetDevice> DynDevice for T {
    async fn start(&mut self) -> anyhow::Result<()> {
        self.start_fallible().await
    }

    async fn stop(&mut self) {
        self.stop().await
    }
    async fn quiesce_input(&mut self) -> anyhow::Result<()> {
        vmcore::device_state::ChangeDeviceState::quiesce_input(self).await
    }
    async fn resume_input(&mut self) -> anyhow::Result<()> {
        vmcore::device_state::ChangeDeviceState::resume_input(self).await
    }

    async fn reset(&mut self) {
        self.reset().await
    }

    async fn advance_time(&mut self, duration: std::time::Duration) -> anyhow::Result<()> {
        self.advance_time(duration).await
    }

    fn poll_device(&mut self, cx: &mut Context<'_>) {
        if let Some(poll) = self.supports_poll_device() {
            poll.poll_device(cx);
        }
    }
}

struct PollEvent(SlimEvent);

impl ArcWake for PollEvent {
    fn wake_by_ref(arc_self: &Arc<Self>) {
        arc_self.0.signal();
    }
}

#[cfg_attr(verus_keep_ghost, verus_verify)]
impl ArcMutexChipsetDeviceUnit {
    /// The unit of `device`, whose lock keeps the unit's share
    /// (`DeviceSlot::new`); `omit_saved_state` must match the lock's.
    #[cfg_attr(verus_keep_ghost, verus_verify(external_body))]
    #[cfg_attr(verus_keep_ghost, verus_spec(unit =>
        requires
            (*device).param().1 == omit_saved_state,
        ensures
            unit.wf(),
            unit.id() == (*device).param().0,
            !unit.running(),
    ))]
    pub fn new(
        device: Arc<vmcore::device_state::DeviceMutex<impl 'static + VmmChipsetDevice>>,
        omit_saved_state: bool,
    ) -> Self {
        Self {
            device,
            poll_event: Arc::new(PollEvent(SlimEvent::new())),
            running: false,
            omit_saved_state,
        }
    }
}

verus! {

/// What the unit's loop waits for.
enum UnitEvent {
    /// A state request and the FracGhost half it carries.
    StateChange(StateRequest, Tracked<RequestGhost>),
    /// The running device asked to be polled.
    Poll,
}

impl ArcMutexChipsetDeviceUnit {
    /// Answers state requests until every sender is gone, polling the
    /// device while it runs.
    ///
    /// Verus: `state_change` is the unit's own channel.
    #[verifier::exec_allows_no_decreases_clause]
    pub async fn run(self, state_change: mesh::Receiver<StateRequest>) -> (result: Self)
        requires
            self.wf(),
            state_change.constant() == self.id(),
        ensures
            result.wf(),
            result.id() == self.id(),
    {
        let mut unit = self;
        let mut recv = state_change;
        let ghost id = unit.id();
        loop
            invariant
                unit.wf(),
                unit.id() == id,
                recv.constant() == id,
        {
            match unit.next_event(&mut recv).await {
                None => break,
                Some(UnitEvent::StateChange(req, gin)) => {
                    state_unit::handle_request(&mut unit, req, gin).await;
                },
                Some(UnitEvent::Poll) => unit.poll_now(),
            }
        }
        unit
    }

    /// Waits for the next state request on `recv` (`None` when every
    /// sender is gone) or, while the device runs, for it to ask to be
    /// polled. Its contract: as `recv_msg`, a request comes with a half
    /// meeting the request protocol for `recv`'s constant.
    ///
    /// PROOF DEFERRED: do not attempt to prove this function until Verus can
    /// state postconditions on futures (part of gap G5, design doc, "The
    /// trust audit"). Verus does not support `select!`; then rewrite it with
    /// `futures::future::select`, give that library function an
    /// `assume_specification`, and prove this from `recv_msg`'s contract.
    #[verifier::external_body]
    async fn next_event(&self, recv: &mut mesh::Receiver<StateRequest>) -> (event: Option<UnitEvent>)
        ensures
            final(recv).constant() == old(recv).constant(),
            event matches Some(UnitEvent::StateChange(req, gin)) ==> StateRequest::request_ok(
                old(recv).constant(),
                req,
                gin@,
            ),
    {
        // Wait for poll requests.
        let poll_fut = async {
            if self.running {
                self.poll_event.0.wait().await;
                return;
            }
            // The device is not running. Never
            // complete this future.
            std::future::pending().await
        };

        futures::select! {
            req = mesh_rpc_verus::recv_msg(recv).fuse() => {
                req.map(|(req, gin)| UnitEvent::StateChange(req, gin))
            }
            () = poll_fut.fuse() => Some(UnitEvent::Poll),
        }
    }

    /// Polls the device. Verus: `external_body`, to prove: a user of the
    /// device's lock (`SNAPSHOT-SPEC-TRUSTED.md`, "The locks' unverified
    /// users"); once verified through `lock_verified` / `release` and
    /// `poll_device`'s contract, Verus checks the device's invariant at
    /// `release`.
    #[verifier::external_body]
    fn poll_now(&self) {
        let waker = waker_ref(&self.poll_event);
        let mut device = self.device.lock();
        device.poll_device(&mut Context::from_waker(&waker));
    }
}

} // verus!

#[cfg_attr(verus_keep_ghost, verus_verify)]
impl StateUnit for ArcMutexChipsetDeviceUnit {
    #[cfg_attr(verus_keep_ghost, verus_verify(external_body))]
    #[cfg_attr(verus_keep_ghost, verus_spec(with Tracked(g): Tracked<&mut Option<FracGhost<UnitSnapshot>>>))]
    async fn start(&mut self) -> anyhow::Result<()> {
        self.device.clone().close().start().await?;
        self.running = true;

        // Poll the device at least once after startup completes.
        let mut device = self.device.lock();
        device.poll_device(&mut Context::from_waker(&waker_ref(&self.poll_event)));
        Ok(())
    }

    #[cfg_attr(verus_keep_ghost, verus_verify(external_body))]
    #[cfg_attr(verus_keep_ghost, verus_spec(with Tracked(g): Tracked<&mut Option<FracGhost<UnitSnapshot>>>))]
    async fn stop(&mut self) {
        self.device.clone().close().stop().await;
        self.running = false;
        // FUTURE: consider closing the mutex while the device is stopped to
        // find bugs. This may be difficult or not worth it since it requires
        // that:
        //
        // 1. all cross-device dependencies are exactly correct.
        // 2. no device manipulation happens externally to normal VM operation
        //    (e.g., no calls are made to the device while the VM is stopped).
        //
        // These are currently not true.
    }

    #[cfg_attr(verus_keep_ghost, verus_verify(external_body))]
    async fn quiesce_input(&mut self) -> anyhow::Result<()> {
        self.device.clone().close().quiesce_input().await
    }

    #[cfg_attr(verus_keep_ghost, verus_verify(external_body))]
    async fn resume_input(&mut self) -> anyhow::Result<()> {
        self.device.clone().close().resume_input().await
    }

    #[cfg_attr(verus_keep_ghost, verus_verify(external_body))]
    #[cfg_attr(verus_keep_ghost, verus_spec(with Tracked(g): Tracked<&mut Option<FracGhost<UnitSnapshot>>>))]
    async fn reset(&mut self) -> anyhow::Result<()> {
        self.device.clone().close().reset().await;
        Ok(())
    }

    #[cfg_attr(verus_keep_ghost, verus_verify(external_body))]
    #[cfg_attr(verus_keep_ghost, verus_spec(with Tracked(g): Tracked<&mut Option<FracGhost<UnitSnapshot>>>))]
    async fn advance_time(&mut self, duration: std::time::Duration) -> anyhow::Result<()> {
        self.device.clone().close().advance_time(duration).await
    }

    #[cfg_attr(verus_keep_ghost, verus_verify(external_body))]
    #[cfg_attr(verus_keep_ghost, verus_spec(with Tracked(g): Tracked<&mut Option<FracGhost<UnitSnapshot>>>))]
    async fn save(&mut self) -> Result<Option<SavedStateBlob>, SaveError> {
        if self.omit_saved_state {
            return Ok(None);
        }

        // TODO: make async
        let state = self.device.clone().close().save()?;
        Ok(Some(state))
    }

    #[cfg_attr(verus_keep_ghost, verus_verify(external_body))]
    #[cfg_attr(verus_keep_ghost, verus_spec(with Tracked(g): Tracked<&mut Option<FracGhost<UnitSnapshot>>>))]
    async fn restore(&mut self, state: SavedStateBlob) -> Result<(), RestoreError> {
        // TODO: make async
        self.device.clone().close().restore(state)
    }
}
