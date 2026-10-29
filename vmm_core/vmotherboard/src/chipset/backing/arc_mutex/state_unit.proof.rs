// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

// Declarations supporting `state_unit.spec.rs`; no human review needed.

use vmcore::save_restore::RestoreError;
use vmcore::save_restore::SaveError;
use vmcore::save_restore::SavedStateBlob;
use vstd::prelude::*;

verus! {

// (`ArcMutexChipsetDeviceUnit` itself is `verus_verify`.)

// (`ChipsetDevice` is declared in `chipset_device_resources`.)

// (`ChangeDeviceState` is declared in `chipset_device_resources`.)

/// `VmmChipsetDevice`, declared to Verus (no methods).
#[verifier::external_trait_specification]
pub trait ExVmmChipsetDevice: chipset_device::ChipsetDevice + inspect::InspectMut
    + vmcore::save_restore::ProtobufSaveRestore + vmcore::device_state::ChangeDeviceState {
    type ExternalTraitSpecificationFor: crate::VmmChipsetDevice;
}

/// `DynDevice`, declared to Verus (its methods are `async_trait` methods,
/// which cannot carry contracts; `save` and `restore` are in
/// `DynDeviceState`, its supertrait, with contracts).
#[verifier::external_trait_specification]
pub(super) trait ExDynDevice: inspect::InspectMut + Send + super::DynDeviceState {
    type ExternalTraitSpecificationFor: super::DynDevice;
}

/// Declares `PollEvent` to Verus, as an opaque type.
#[verifier::external_type_specification]
#[verifier::external_body]
pub(super) struct ExPollEvent(super::PollEvent);

} // verus!
