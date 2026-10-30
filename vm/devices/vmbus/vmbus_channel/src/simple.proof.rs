// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

// OUT OF SCOPE (vmbus): the Verus spec in this file is parked. vmbus is not
// on the microVM's snapshot path (`SNAPSHOT-SPEC-DESIGN.md`, "Out of scope
// for now"). Do not attempt to prove anything here. (Kept because the
// `StateUnit` trait requires the vmbus units to follow its protocol, and so
// that the work is not lost if vmbus comes back into scope.)

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

/// Declares `GpadlRingMem` to Verus, by name only (fields hidden).
#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExGpadlRingMem(GpadlRingMem);

#[verifier::external_trait_specification]
pub trait ExSimpleVmbusDevice<M: RingMem = GpadlRingMem>: 'static + Send {
    type ExternalTraitSpecificationFor: SimpleVmbusDevice<M>;

    type SavedState: SavedStateRoot + Send;

    type Runner: 'static + Send;
}

/// Declares `SimpleDeviceWrapper` to Verus, by name only (fields hidden).
#[verifier::external_type_specification]
#[verifier::external_body]
#[verifier::reject_recursive_types(T)]
pub struct ExSimpleDeviceWrapper<T: SimpleVmbusDevice>(SimpleDeviceWrapper<T>);

/// Declares `VmTaskDriverSource` to Verus, by name only (fields hidden).
#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExVmTaskDriverSource(vmcore::vm_task::VmTaskDriverSource);

/// Declares `InitialDeviceState` to Verus, by name only (fields hidden).
#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExInitialDeviceState(crate::simple::InitialDeviceState);

} // verus!
