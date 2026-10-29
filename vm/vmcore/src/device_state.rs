// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

//! Trait definition for chipset device state transitions.

#![forbid(unsafe_code)]

use std::future::Future;
use std::time::Duration;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

/// Verus specification of a device's ghost state.
#[cfg(verus_keep_ghost)]
pub mod spec {
    include!("device_state.spec.rs");
}

/// What a chipset device's lock needs of the device (see
/// `spec::DeviceSnapshot`). Empty in normal builds.
#[cfg(verus_keep_ghost)]
pub use spec::DeviceSnapshot;

/// What a chipset device's lock needs of the device (see
/// `spec::DeviceSnapshot`). Empty in normal builds.
#[cfg(not(verus_keep_ghost))]
pub trait DeviceSnapshot {}

#[cfg(not(verus_keep_ghost))]
impl<T: ?Sized> DeviceSnapshot for T {}

/// A chipset device in its lock (a [`DeviceMutex`]): the device and, in
/// Verus builds, its state unit's share of its FracGhost, kept with the
/// device's state so that whoever changes the device under the lock can
/// update the share in the same step (see "Chipset devices: the share in
/// the device's lock" in `SNAPSHOT-SPEC-DESIGN.md`). Normal builds hold just
/// the device. Dereferences to the device.
#[cfg_attr(verus_keep_ghost, verus_verify)]
pub struct DeviceSlot<T: ?Sized> {
    /// The unit's share (Verus only).
    #[cfg(verus_keep_ghost)]
    share: Tracked<spec::DeviceGhostState>,
    /// Whether the unit omits the device's saved state (Verus only).
    #[cfg(verus_keep_ghost)]
    omit: Ghost<bool>,
    device: T,
}

/// A chipset device's lock.
pub type DeviceMutex<T> = closeable_mutex::CloseableMutex<DeviceSlot<T>>;

#[cfg_attr(verus_keep_ghost, verus_verify)]
impl<T: DeviceSnapshot> DeviceSlot<T> {
    /// `device` in a slot. Verus: it creates the unit's FracGhost (value
    /// `Unknown`), keeps one half as the slot's share, and gives the other
    /// back in `half`, for `StateUnits` (registered with
    /// `state_unit::unit_channel_for`). `omit` is whether the unit will omit
    /// the device's saved state.
    #[cfg_attr(verus_keep_ghost, verus_verify(external_body))]
    #[cfg_attr(verus_keep_ghost, verus_spec(slot =>
        with
            Ghost(omit): Ghost<bool>,
            Tracked(half): Tracked<&mut Option<vstd::resource::frac::FracGhost<crate::save_restore::spec::UnitSnapshot>>>,
        ensures
            closeable_mutex::LockInvariant::inv(&slot),
            closeable_mutex::LockInvariant::key(&slot).1 == omit,
            *final(half) matches Some(h) && h.valid(closeable_mutex::LockInvariant::key(&slot).0, 0.5real)
                && h@ is Unknown,
    ))]
    pub fn new(device: T) -> Self {
        Self {
            // PROOF TODO: replace these placeholders (`assume_new()`) with the
            // real ghost: the device's share and `omit`, as the contract
            // states.
            #[cfg(verus_keep_ghost)]
            share: Tracked::assume_new(),
            #[cfg(verus_keep_ghost)]
            omit: Ghost::assume_new(),
            device,
        }
    }
}

impl<T: ?Sized> std::ops::Deref for DeviceSlot<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.device
    }
}

impl<T: ?Sized> std::ops::DerefMut for DeviceSlot<T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.device
    }
}

/// Trait for transitioning device state.
pub trait ChangeDeviceState {
    /// Starts a device, allowing it to interact with the guest asynchronously.
    ///
    /// For example, a device might process work queues that reside in guest
    /// memory on a separate thread.
    ///
    /// Callers must ensure that the device is in a stopped state before calling
    /// this method.
    ///
    /// This is a synchronous method instead of an asynchronous one because it
    /// is a notification only--callers do not need to wait for the device to
    /// finish starting to consider the VM started. Devices should kick off any
    /// tasks that need to run any return, without waiting.
    ///
    // FUTURE: make this asynchronous if any device has a good reason to need
    // it. This may also require changing state units to wait for device start
    // to finish, which may require a bunch of other changes.
    fn start(&mut self);

    /// Starts a device and waits until startup has either completed or failed.
    ///
    /// The default preserves the notification-only behavior of [`Self::start`].
    /// Devices that restore process-local workers should override this so a
    /// restore failure is reported before dependent units start.
    fn start_fallible(&mut self) -> impl Send + Future<Output = anyhow::Result<()>> {
        self.start();
        async { Ok(()) }
    }

    /// Stops accepting new host input before a snapshot vCPU boundary.
    fn quiesce_input(&mut self) -> impl Send + Future<Output = anyhow::Result<()>> {
        async { Ok(()) }
    }

    /// Resumes host input after a failed snapshot transaction.
    fn resume_input(&mut self) -> impl Send + Future<Output = anyhow::Result<()>> {
        async { Ok(()) }
    }

    /// Stops a device's asynchronous work.
    ///
    /// After this returns, the device must not process any additional work. It
    /// should be in a stable state where it can be saved without losing data
    /// (if it implements the appropriate trait).
    ///
    /// Callers must ensure that the device is in a started state before calling
    /// this method.
    fn stop(&mut self) -> impl Send + Future<Output = ()>;

    /// Resets the device state to its initial state, for a fresh boot.
    ///
    /// Callers must ensure that the device is in a stopped state before calling
    /// this method.
    fn reset(&mut self) -> impl Send + Future<Output = ()>;

    /// Advances guest-visible device time after snapshot restore.
    fn advance_time(
        &mut self,
        _duration: Duration,
    ) -> impl Send + Future<Output = anyhow::Result<()>> {
        async { Ok(()) }
    }

    /// Returns whether [`Self::advance_time`] does any work for this device,
    /// because it keeps guest-visible time of its own instead of deriving its
    /// deadlines from VM time. Only such devices receive the advance after a
    /// restore, so a device that implements [`Self::advance_time`] must
    /// return `true`.
    fn advances_time(&self) -> bool {
        false
    }
}
