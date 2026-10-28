// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

// Declarations supporting `simple.spec.rs`; no human review needed.

use crate::gpadl_ring::GpadlRingMem;
use crate::simple::SimpleDeviceWrapper;
use crate::simple::SimpleVmbusDevice;
use vmbus_ring::RingMem;
use vmcore::save_restore::SavedStateRoot;
use vstd::prelude::*;

verus! {

#[verifier::external_trait_specification]
pub trait ExRingMem: Send {
    type ExternalTraitSpecificationFor: RingMem;
}

/// Declares `GpadlRingMem` to Verus, as an opaque type.
#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExGpadlRingMem(GpadlRingMem);

#[verifier::external_trait_specification]
pub trait ExSimpleVmbusDevice<M: RingMem = GpadlRingMem>: 'static + Send {
    type ExternalTraitSpecificationFor: SimpleVmbusDevice<M>;

    type SavedState: SavedStateRoot + Send;

    type Runner: 'static + Send;
}

/// Declares `SimpleDeviceWrapper` to Verus, as an opaque type.
#[verifier::external_type_specification]
#[verifier::external_body]
#[verifier::reject_recursive_types(T)]
pub struct ExSimpleDeviceWrapper<T: SimpleVmbusDevice>(SimpleDeviceWrapper<T>);

/// Declares `VmTaskDriverSource` to Verus, as an opaque type.
#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExVmTaskDriverSource(vmcore::vm_task::VmTaskDriverSource);

/// Declares `InitialDeviceState` to Verus, as an opaque type.
#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExInitialDeviceState(crate::simple::InitialDeviceState);

} // verus!
