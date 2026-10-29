// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

//! State unit definitions for vmbus components.

#![warn(missing_docs)]

use inspect::Inspect;
use inspect::InspectMut;
use pal_async::task::Spawn;
use state_unit::InspectConcurrently;
use state_unit::NameInUse;
use state_unit::SpawnedUnit;
use state_unit::StateUnit;
#[cfg(verus_keep_ghost)]
use state_unit::StateUnitSpec;
use state_unit::StateUnits;
use state_unit::UnitBuilder;
use state_unit::UnitHandle;
use state_unit::run_async_unit;
use std::sync::Arc;
use vm_resource::Resource;
use vm_resource::ResourceResolver;
use vm_resource::kind::VmbusDeviceHandleKind;
use vmbus_channel::channel::ChannelHandle;
use vmbus_channel::channel::VmbusDevice;
use vmbus_channel::channel::offer_channel;
use vmbus_channel::channel::offer_generic_channel;
use vmbus_channel::resources::ResolveVmbusDeviceHandleParams;
use vmbus_channel::simple::InitialDeviceState;
use vmbus_channel::simple::SimpleDeviceHandle;
use vmbus_channel::simple::SimpleVmbusDevice;
use vmbus_channel::simple::offer_simple_device;
use vmbus_server::VmbusServer;
use vmbus_server::VmbusServerControl;
use vmcore::save_restore::RestoreError;
use vmcore::save_restore::SaveError;
use vmcore::save_restore::SavedStateBlob;
#[cfg(verus_keep_ghost)]
use vmcore::save_restore::spec::UnitSnapshot;
use vmcore::vm_task::VmTaskDriverSource;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
use vstd::resource::frac::FracGhost;

/// The `StateUnit` spec of the vmbus server's and channels' state units.
#[cfg(verus_keep_ghost)]
mod spec {
    include!("vmbus_unit.spec.rs");
}

#[cfg(verus_keep_ghost)]
mod proof {
    include!("vmbus_unit.proof.rs");
}

/// A handle to a vmbus server that is registered as a state unit.
///
/// FUTURE: incorporate the state unit handling directly into `VmbusServer`.
pub struct VmbusServerHandle {
    unit: SpawnedUnit<VmbusServerUnit>,
    control: Arc<VmbusServerControl>,
}

impl VmbusServerHandle {
    /// Makes a new handle, registering the server via `builder`.
    pub fn new(
        spawner: &impl Spawn,
        builder: UnitBuilder<'_>,
        server: VmbusServer,
    ) -> Result<Self, NameInUse> {
        let control = server.control();
        let unit = builder.spawn(spawner, |recv| {
            run_async_unit(VmbusServerUnit(server), recv)
        })?;
        Ok(Self { unit, control })
    }

    /// Gets the vmbus control interface.
    pub fn control(&self) -> &Arc<VmbusServerControl> {
        &self.control
    }

    /// Gets the vmbus unit handle.
    pub fn unit_handle(&self) -> &UnitHandle {
        self.unit.handle()
    }

    /// Removes the server.
    pub async fn remove(self) -> VmbusServer {
        self.unit.remove().await.0
    }
}

/// A newtype over `VmbusServer` implementing [`StateUnit`].
#[derive(Inspect)]
#[inspect(transparent)]
struct VmbusServerUnit(VmbusServer);

impl InspectMut for VmbusServerUnit {
    fn inspect_mut(&mut self, req: inspect::Request<'_>) {
        Inspect::inspect(&*self, req)
    }
}

/// Inspects concurrently through the server task (as inspecting the unit does).
impl InspectConcurrently for VmbusServerUnit {
    type Inspector = vmbus_server::VmbusServerInspector;

    fn inspector(&self) -> Self::Inspector {
        self.0.inspector()
    }
}

#[cfg_attr(verus_keep_ghost, verus_verify)]
impl StateUnit for VmbusServerUnit {
    #[cfg_attr(verus_keep_ghost, verus_verify(external_body))]
    #[cfg_attr(verus_keep_ghost, verus_spec(with Tracked(g): Tracked<&mut Option<FracGhost<UnitSnapshot>>>))]
    async fn start(&mut self) -> anyhow::Result<()> {
        self.0.start();
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
        Ok(Some(SavedStateBlob::new(self.0.save().await)))
    }

    #[cfg_attr(verus_keep_ghost, verus_verify(external_body))]
    #[cfg_attr(verus_keep_ghost, verus_spec(with Tracked(g): Tracked<&mut Option<FracGhost<UnitSnapshot>>>))]
    async fn restore(&mut self, buffer: SavedStateBlob) -> Result<(), RestoreError> {
        self.0
            .restore(buffer.parse()?)
            .await
            .map_err(|err| RestoreError::Other(err.into()))
    }
}

/// A type wrapping a [`ChannelHandle`] and implementing [`StateUnit`].
#[must_use]
#[derive(Inspect)]
#[cfg_attr(not(verus_keep_ghost), derive(Debug))]
#[inspect(transparent)]
#[cfg_attr(verus_keep_ghost, verus_verify)]
#[cfg_attr(verus_keep_ghost, verifier::reject_recursive_types(T))]
pub struct ChannelUnit<T: ?Sized>(
    ChannelHandle<T>,
    /// Whether the unit runs (Verus only). Its share of its FracGhost is
    /// kept by the channel's task (`vmbus_channel`).
    #[cfg(verus_keep_ghost)]
    #[inspect(skip)]
    Ghost<bool>,
);

// The derived `Debug`, without the ghost field (which has no `Debug`).
#[cfg(verus_keep_ghost)]
impl<T: ?Sized + std::fmt::Debug> std::fmt::Debug for ChannelUnit<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("ChannelUnit").field(&self.0).finish()
    }
}

#[cfg_attr(verus_keep_ghost, verus_verify)]
impl<T: 'static + VmbusDevice + ?Sized> ChannelUnit<T> {
    /// The unit of the channel `handle` (stopped).
    #[cfg_attr(verus_keep_ghost, verus_verify(external_body))]
    #[cfg_attr(verus_keep_ghost, verus_spec(unit =>
        ensures
            unit.wf(),
            unit.id() == handle.id(),
            !unit.running(),
    ))]
    fn new(handle: ChannelHandle<T>) -> Self {
        Self(
            handle,
            // OUT OF SCOPE: keep this `assume_new()` (vmbus is out of scope;
            // design doc).
            #[cfg(verus_keep_ghost)]
            Ghost::assume_new(),
        )
    }
}

/// Offers a channel, creates a unit for it, and adds it to `state_units`.
pub async fn offer_channel_unit<T: 'static + VmbusDevice>(
    driver: &impl Spawn,
    state_units: &StateUnits,
    vmbus: &VmbusServerHandle,
    channel: T,
) -> anyhow::Result<SpawnedUnit<ChannelUnit<T>>> {
    let offer = channel.offer();
    let name = format!("{}:{}", offer.interface_name, offer.instance_id);
    let handle = offer_channel(driver, vmbus.control.as_ref(), channel).await?;
    let unit = state_units
        .add(name)
        .depends_on(vmbus.unit.handle())
        // The channel's task keeps the unit's share; the other half goes to
        // the unit's sender (`unit_channel_for`).
        .spawn_with_channel(driver, state_unit::unit_channel_for(), |recv| {
            run_async_unit(ChannelUnit::new(handle), recv)
        })?;
    Ok(unit)
}

impl<T: 'static + VmbusDevice> ChannelUnit<T> {
    /// Revokes a channel.
    pub async fn revoke(self) -> T {
        self.0.revoke().await.unwrap()
    }
}

impl<T: ?Sized> InspectMut for ChannelUnit<T> {
    fn inspect_mut(&mut self, req: inspect::Request<'_>) {
        Inspect::inspect(&*self, req)
    }
}

/// Inspects concurrently through the channel's task (as inspecting the unit
/// does).
impl<T: ?Sized> InspectConcurrently for ChannelUnit<T> {
    type Inspector = vmbus_channel::channel::ChannelInspector;

    fn inspector(&self) -> Self::Inspector {
        self.0.inspector()
    }
}

#[cfg_attr(verus_keep_ghost, verus_verify)]
impl<T: 'static + VmbusDevice + ?Sized> StateUnit for ChannelUnit<T> {
    #[cfg_attr(verus_keep_ghost, verus_verify(external_body))]
    #[cfg_attr(verus_keep_ghost, verus_spec(with Tracked(g): Tracked<&mut Option<FracGhost<UnitSnapshot>>>))]
    async fn start(&mut self) -> anyhow::Result<()> {
        self.0.start();
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
        let state = self.0.save().await.map_err(SaveError::Other)?;
        Ok(state)
    }

    #[cfg_attr(verus_keep_ghost, verus_verify(external_body))]
    #[cfg_attr(verus_keep_ghost, verus_spec(with Tracked(g): Tracked<&mut Option<FracGhost<UnitSnapshot>>>))]
    async fn restore(&mut self, state: SavedStateBlob) -> Result<(), RestoreError> {
        self.0.restore(state).await.map_err(RestoreError::Other)
    }
}

/// A type wrapping a [`ChannelHandle`] and implementing [`StateUnit`].
#[must_use]
#[cfg_attr(not(verus_keep_ghost), derive(Debug))]
#[cfg_attr(verus_keep_ghost, verus_verify)]
#[cfg_attr(verus_keep_ghost, verifier::reject_recursive_types(T))]
pub struct SimpleChannelUnit<T: SimpleVmbusDevice>(
    SimpleDeviceHandle<T>,
    /// Whether the unit runs (Verus only). Its share of its FracGhost is
    /// kept by the channel's task (`vmbus_channel`).
    #[cfg(verus_keep_ghost)]
    Ghost<bool>,
);

// The derived `Debug`, without the ghost field (which has no `Debug`).
#[cfg(verus_keep_ghost)]
impl<T: SimpleVmbusDevice + std::fmt::Debug> std::fmt::Debug for SimpleChannelUnit<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("SimpleChannelUnit").field(&self.0).finish()
    }
}

#[cfg_attr(verus_keep_ghost, verus_verify)]
impl<T: SimpleVmbusDevice> SimpleChannelUnit<T> {
    /// The unit of the channel `handle` (stopped).
    #[cfg_attr(verus_keep_ghost, verus_verify(external_body))]
    #[cfg_attr(verus_keep_ghost, verus_spec(unit =>
        ensures
            unit.wf(),
            unit.id() == handle.id(),
            !unit.running(),
    ))]
    fn new(handle: SimpleDeviceHandle<T>) -> Self {
        Self(
            handle,
            // OUT OF SCOPE: keep this `assume_new()` (vmbus is out of scope;
            // design doc).
            #[cfg(verus_keep_ghost)]
            Ghost::assume_new(),
        )
    }
}

/// Offers a simple vmbus device, creates a unit for it, and adds it to `state_units`.
pub async fn offer_simple_device_unit<T: SimpleVmbusDevice>(
    driver_source: &VmTaskDriverSource,
    state_units: &StateUnits,
    vmbus: &VmbusServerHandle,
    device: T,
) -> anyhow::Result<SpawnedUnit<SimpleChannelUnit<T>>> {
    anyhow::ensure!(
        !state_units.is_running(),
        "cannot offer a simple VMBus device while state units are running"
    );
    let offer = device.offer();
    let name = format!("{}:{}", offer.interface_name, offer.instance_id);
    let handle = offer_simple_device(
        driver_source,
        vmbus.control.as_ref(),
        device,
        InitialDeviceState::Stopped,
    )
    .await?;
    let unit = state_units
        .add(name)
        .depends_on(vmbus.unit.handle())
        // The channel's task keeps the unit's share; the other half goes to
        // the unit's sender (`unit_channel_for`).
        .spawn_with_channel(
            driver_source.simple(),
            state_unit::unit_channel_for(),
            |recv| run_async_unit(SimpleChannelUnit::new(handle), recv),
        )?;
    Ok(unit)
}

impl<T: SimpleVmbusDevice> SimpleChannelUnit<T> {
    /// Revokes the channel and returns it.
    pub async fn revoke(self) -> T {
        self.0.revoke().await.unwrap()
    }
}

impl<T: SimpleVmbusDevice> Inspect for SimpleChannelUnit<T> {
    fn inspect(&self, req: inspect::Request<'_>) {
        self.0.inspect(req);
    }
}

impl<T: SimpleVmbusDevice> InspectMut for SimpleChannelUnit<T> {
    fn inspect_mut(&mut self, req: inspect::Request<'_>) {
        Inspect::inspect(&*self, req)
    }
}

/// Inspects concurrently through the channel's task (as inspecting the unit
/// does).
impl<T: SimpleVmbusDevice> InspectConcurrently for SimpleChannelUnit<T> {
    type Inspector = vmbus_channel::channel::ChannelInspector;

    fn inspector(&self) -> Self::Inspector {
        self.0.inspector()
    }
}

#[cfg_attr(verus_keep_ghost, verus_verify)]
impl<T: SimpleVmbusDevice> StateUnit for SimpleChannelUnit<T> {
    #[cfg_attr(verus_keep_ghost, verus_verify(external_body))]
    #[cfg_attr(verus_keep_ghost, verus_spec(with Tracked(g): Tracked<&mut Option<FracGhost<UnitSnapshot>>>))]
    async fn start(&mut self) -> anyhow::Result<()> {
        self.0.start();
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
        let state = self.0.save().await.map_err(SaveError::Other)?;
        Ok(state)
    }

    #[cfg_attr(verus_keep_ghost, verus_verify(external_body))]
    #[cfg_attr(verus_keep_ghost, verus_spec(with Tracked(g): Tracked<&mut Option<FracGhost<UnitSnapshot>>>))]
    async fn restore(&mut self, state: SavedStateBlob) -> Result<(), RestoreError> {
        self.0.restore(state).await.map_err(RestoreError::Other)
    }
}

/// Offers a channel, creates a unit for it, and adds it to `state_units`.
pub async fn offer_vmbus_device_handle_unit(
    driver_source: &VmTaskDriverSource,
    state_units: &StateUnits,
    vmbus: &VmbusServerHandle,
    resolver: &ResourceResolver,
    resource: Resource<VmbusDeviceHandleKind>,
) -> anyhow::Result<SpawnedUnit<ChannelUnit<dyn VmbusDevice>>> {
    let channel = resolver
        .resolve(resource, ResolveVmbusDeviceHandleParams { driver_source })
        .await?;
    let offer = channel.0.offer();
    let name = format!("{}:{}", offer.interface_name, offer.instance_id);
    let handle =
        offer_generic_channel(&driver_source.simple(), vmbus.control.as_ref(), channel.0).await?;
    let unit = state_units
        .add(name)
        .depends_on(vmbus.unit.handle())
        // The channel's task keeps the unit's share; the other half goes to
        // the unit's sender (`unit_channel_for`).
        .spawn_with_channel(
            driver_source.simple(),
            state_unit::unit_channel_for(),
            |recv| run_async_unit(ChannelUnit::new(handle), recv),
        )?;
    Ok(unit)
}
