// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

// Declarations supporting `state_unit.spec.rs`; no human review needed.

use vstd::prelude::*;

verus! {

// (`ArcMutexChipsetDeviceUnit` itself is `verus_verify`.)

/// `ChipsetDevice`, declared to Verus (no methods).
#[verifier::external_trait_specification]
pub trait ExChipsetDevice: 'static + Send {
    type ExternalTraitSpecificationFor: chipset_device::ChipsetDevice;
}

/// `ChangeDeviceState`, declared to Verus (no methods).
#[verifier::external_trait_specification]
pub trait ExChangeDeviceState {
    type ExternalTraitSpecificationFor: vmcore::device_state::ChangeDeviceState;
}

/// `VmmChipsetDevice`, declared to Verus (no methods).
#[verifier::external_trait_specification]
pub trait ExVmmChipsetDevice: chipset_device::ChipsetDevice + inspect::InspectMut
    + vmcore::save_restore::ProtobufSaveRestore + vmcore::device_state::ChangeDeviceState {
    type ExternalTraitSpecificationFor: crate::VmmChipsetDevice;
}

/// `DynDevice`, declared to Verus (no methods).
#[verifier::external_trait_specification]
pub(super) trait ExDynDevice: inspect::InspectMut + Send + vmcore::device_state::DeviceSnapshot {
    type ExternalTraitSpecificationFor: super::DynDevice;
}

/// Declares `PollEvent` to Verus, as an opaque type.
#[verifier::external_type_specification]
#[verifier::external_body]
pub(super) struct ExPollEvent(super::PollEvent);

} // verus!
