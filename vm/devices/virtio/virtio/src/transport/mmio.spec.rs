// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

// High-level spec of the virtio-MMIO transport, for human review.

use super::super::super::core::QueueData;
use super::super::super::core::VirtioTransportCore;
use super::super::MmioTransport;
use super::super::VirtioMmioDevice;
use super::state::SavedQueueState;
use super::state::SavedState;
use crate::queue::QueueState;
use vmcore::save_restore::spec::SaveRestoreSpecImpl;
use vmcore::save_restore::spec::SavedStateBlobView;
use vstd::prelude::*;

verus! {

/// The view of one virtqueue: its guest-programmed configuration and its
/// progress, as the transport saves them.
pub struct VirtioQueueView {
    /// The queue size.
    pub size: u16,
    /// Whether the guest enabled the queue.
    pub enable: bool,
    /// The descriptor table's guest address.
    pub desc_addr: u64,
    /// The available ring's guest address.
    pub avail_addr: u64,
    /// The used ring's guest address.
    pub used_addr: u64,
    /// The queue's progress (available and used indexes), if saved or
    /// captured at the last stop.
    pub progress: Option<QueueState>,
}

/// The view of a virtio-MMIO device: the transport state it saves, the
/// inner device's captured private state, and the interrupt status. It is
/// also the view of its saved state. (Not in the view: the device task and
/// its command sender, the transport's busy state, deferred and stalled
/// I/O, doorbells, guest memory, the device features, each queue's initial
/// size and event, the staged restore state, the MMIO region and IDs, and
/// the interrupt delivery's internals.)
pub struct VirtioMmioView {
    /// Whether the inner device supports save/restore (static
    /// configuration; a device that does not cannot be saved).
    pub supports_save_restore: bool,
    /// The device status register.
    pub device_status: u8,
    /// The driver's feature banks (two).
    pub driver_feature_banks: Seq<u32>,
    /// The device feature select register.
    pub device_feature_select: u32,
    /// The driver feature select register.
    pub driver_feature_select: u32,
    /// The queue select register.
    pub queue_select: u32,
    /// The configuration generation.
    pub config_generation: u32,
    /// The queues.
    pub queues: Seq<VirtioQueueView>,
    /// The inner device's private state, as captured at the last stop or
    /// set by the last restore: `Some(state)` if there is one to save,
    /// `None` if not (nothing captured, or the capture failed; then `save`
    /// fails).
    pub device_state: Option<Option<SavedStateBlobView>>,
    /// The interrupt status register (TRUSTED accessor, see
    /// `MmioTransport::interrupt_status`).
    pub interrupt_status: u32,
}

impl VirtioMmioView {
    /// Two feature banks.
    pub open spec fn inv(self) -> bool {
        self.driver_feature_banks.len() == 2
    }

    /// The saved state of a device in this state, as a view: the same,
    /// with the private state to save (none if none was captured).
    pub open spec fn saved(self) -> VirtioMmioView {
        VirtioMmioView {
            device_state: match self.device_state {
                Some(s) => Some(s),
                None => Some(None),
            },
            ..self
        }
    }
}

impl View for QueueData {
    type V = VirtioQueueView;

    closed spec fn view(&self) -> VirtioQueueView {
        VirtioQueueView {
            size: self.params.size,
            enable: self.params.enable,
            desc_addr: self.params.desc_addr,
            avail_addr: self.params.avail_addr,
            used_addr: self.params.used_addr,
            progress: self.saved_state,
        }
    }
}

impl MmioTransport {
    /// The interrupt status the guest sees (TRUSTED boundary:
    /// uninterpreted). In the shared-status mode (the microVM's) it is a
    /// 32-bit word in guest memory at `status_gpa`, which the device sets
    /// and the guest reads and clears directly; in the legacy modes, the
    /// `status` field behind `interrupt_state`'s lock. The trusted
    /// contracts of `read_interrupt_status` and `restore_interrupt_status`
    /// (`transport/mmio/interrupt.rs`) say how `save` reads it and
    /// `restore` sets it; otherwise only the guest (while
    /// its VPs run) and the device task's interrupts (while the device runs)
    /// change it. See the design doc, "virtio-MMIO's interrupt status: the
    /// guest-memory boundary".
    pub uninterp spec fn interrupt_status(&self) -> u32;
}

impl View for VirtioMmioDevice {
    type V = VirtioMmioView;

    closed spec fn view(&self) -> VirtioMmioView {
        let core: VirtioTransportCore = self.core;
        VirtioMmioView {
            supports_save_restore: core.supports_save_restore,
            device_status: core.device_status.bits(),
            driver_feature_banks: seq![
                core.driver_feature.bank_bits(0),
                core.driver_feature.bank_bits(1),
            ],
            device_feature_select: core.device_feature_select,
            driver_feature_select: core.driver_feature_select,
            queue_select: core.queue_select,
            config_generation: core.config_generation,
            queues: core.queues@.map_values(|q: QueueData| q@),
            device_state: core.restore.captured(),
            interrupt_status: self.mmio.interrupt_status(),
        }
    }
}

impl VirtioMmioDevice {
    /// Well formed: the view's invariant holds.
    pub closed spec fn wf(&self) -> bool {
        self@.inv()
    }
}

/// The view of a saved queue: its configuration and progress.
impl View for SavedQueueState {
    type V = VirtioQueueView;

    closed spec fn view(&self) -> VirtioQueueView {
        VirtioQueueView {
            size: self.common.size,
            enable: self.common.enable,
            desc_addr: self.common.desc_addr,
            avail_addr: self.common.avail_addr,
            used_addr: self.common.used_addr,
            progress: self.common.queue_state,
        }
    }
}

/// The view of the saved state: the same fields, with the private state
/// to restore. (`supports_save_restore` is true: only such a device saves.)
impl View for SavedState {
    type V = VirtioMmioView;

    closed spec fn view(&self) -> VirtioMmioView {
        VirtioMmioView {
            supports_save_restore: true,
            device_status: self.common.device_status,
            driver_feature_banks: self.common.driver_feature_banks@,
            device_feature_select: self.common.device_feature_select,
            driver_feature_select: self.common.driver_feature_select,
            queue_select: self.common.queue_select,
            config_generation: self.common.config_generation,
            queues: self.queues@.map_values(|q: SavedQueueState| q@),
            device_state: Some(match self.device_state {
                Some(b) => Some(b@),
                None => None,
            }),
            interrupt_status: self.interrupt_status,
        }
    }
}

/// A virtio-MMIO device saves its view, with the private state to save.
impl SaveRestoreSpecImpl for VirtioMmioDevice {
    closed spec fn saved_view(view: VirtioMmioView) -> VirtioMmioView {
        view.saved()
    }

    /// The device is well formed (`wf()`).
    closed spec fn device_wf(&self) -> bool {
        self.wf()
    }
}

} // verus!
