// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

// Declarations supporting the virtio-MMIO transport's spec; no human review
// needed.

use crate::transport::StalledIo;
use crate::transport::core::QueueData;
use crate::transport::core::VirtioTransportCore;
use crate::transport::task::DeviceCommand;
use crate::transport::task::TransportState;
use crate::transport::mmio::InterruptState;
use crate::transport::mmio::interrupt::InterruptAck;
use super::super::state::SavedQueueState;
use super::super::state::SavedState;
use crate::VirtioDoorbells;
use crate::queue::QueueParams;
use crate::queue::QueueState;
use crate::transport::core::restore::Validator;
use crate::transport::task::restore::DeviceRestoreState;
use crate::transport::saved_state::state::CommonQueueState;
use crate::transport::saved_state::state::CommonSavedState;
use chipset_device::io::deferred::DeferredWrite;
use guestmem::GuestMemory;
use vstd::prelude::*;

verus! {

// The transport's types, visible to Verus (`VirtioMmioDevice`,
// `MmioTransport`, and `TransportRestore` are `verus_verify` themselves:
// their fields are private).

#[verifier::external_type_specification]
pub(crate) struct ExVirtioTransportCore(VirtioTransportCore);

#[verifier::external_type_specification]
pub(crate) struct ExQueueData(QueueData);

#[verifier::external_type_specification]
pub struct ExQueueParams(QueueParams);

#[verifier::external_type_specification]
pub struct ExQueueState(QueueState);

// The saved state.

#[verifier::external_type_specification]
pub struct ExSavedState(SavedState);

#[verifier::external_type_specification]
pub struct ExSavedQueueState(SavedQueueState);

#[verifier::external_type_specification]
pub struct ExCommonSavedState(CommonSavedState);

#[verifier::external_type_specification]
pub struct ExCommonQueueState(CommonQueueState);

// Runtime glue and opaque state, as opaque types.

#[verifier::external_type_specification]
#[verifier::external_body]
pub(crate) struct ExValidator(Validator);

#[verifier::external_type_specification]
pub struct ExDeviceRestoreState(DeviceRestoreState);

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExDeviceCommand(DeviceCommand);

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExTransportState(TransportState);

#[verifier::external_type_specification]
#[verifier::external_body]
pub(crate) struct ExVirtioDoorbells(VirtioDoorbells);

#[verifier::external_type_specification]
#[verifier::external_body]
pub(crate) struct ExStalledIo(StalledIo);

#[verifier::external_type_specification]
#[verifier::external_body]
pub(crate) struct ExInterruptState(InterruptState);

#[verifier::external_type_specification]
#[verifier::external_body]
pub(crate) struct ExInterruptAck(InterruptAck);

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExGuestMemory(GuestMemory);

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExDeferredWrite(DeferredWrite);

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExEvent(pal_event::Event);

#[verifier::external_type_specification]
#[verifier::external_body]
#[verifier::reject_recursive_types(T)]
#[verifier::reject_recursive_types(M)]
pub struct ExTask<T, M>(async_task::Task<T, M>);

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExTaskMetadata(pal_async::task::TaskMetadata);

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExRawMutex(parking_lot::RawMutex);

#[verifier::external_type_specification]
#[verifier::external_body]
#[verifier::reject_recursive_types(R)]
#[verifier::reject_recursive_types(T)]
pub struct ExMutex<R, T: ?Sized>(parking_lot::lock_api::Mutex<R, T>);

} // verus!
