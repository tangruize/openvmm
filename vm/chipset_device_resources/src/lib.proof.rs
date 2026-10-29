// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

// Declarations supporting the type-erased chipset device's spec; no human
// review needed.

use super::DynChipsetDevice;
use vstd::prelude::*;

verus! {

/// `ChipsetDevice`, declared to Verus (no methods). Here, upstream of every
/// crate that names it (`chipset`, `vmotherboard`).
#[verifier::external_trait_specification]
pub trait ExChipsetDevice: 'static + Send {
    type ExternalTraitSpecificationFor: chipset_device::ChipsetDevice;
}

/// `ChangeDeviceState`, declared to Verus (no methods). Here, upstream of
/// `vmotherboard`.
#[verifier::external_trait_specification]
pub trait ExChangeDeviceState {
    type ExternalTraitSpecificationFor: vmcore::device_state::ChangeDeviceState;
}

/// `ResolvedChipsetDevice` (its field is public).
#[verifier::external_type_specification]
pub struct ExResolvedChipsetDevice(super::ResolvedChipsetDevice);

/// `DynChipsetDevice`, declared to Verus (its own methods are `async_trait`
/// methods, without contracts; `save` and `restore` are its supertrait
/// `ProtobufSaveRestore`'s).
#[verifier::external_trait_specification]
pub(crate) trait ExDynChipsetDevice: chipset_device::ChipsetDevice + vmcore::save_restore::ProtobufSaveRestore + inspect::InspectMut {
    type ExternalTraitSpecificationFor: DynChipsetDevice;
}

} // verus!
