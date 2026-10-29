// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

// Declarations supporting `partition_unit.spec.rs`; no human review needed.

use super::InternalHaltReason;
use super::VmPartition;
use virt::InitialRegs;
use super::Topology;
use vmm_core_defs::HaltReason;
use vstd::prelude::*;

verus! {

// `PartitionUnitRunner` is `verus_verify` itself (its fields are
// private); the types of its fields, declared to Verus.

#[verifier::external_trait_specification]
pub trait ExVmPartition: 'static + Send + Sync + inspect::InspectMut + vmcore::save_restore::ProtobufSaveRestore {
    type ExternalTraitSpecificationFor: VmPartition;
}

#[verifier::external_type_specification]
#[verifier::external_body]
pub(super) struct ExInternalHaltReason(InternalHaltReason);

/// Declares `HaltReason` to Verus, as an opaque type (here, for the
/// partition unit and its users, `openvmm_core`).
#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExHaltReason(HaltReason);

#[verifier::external_type_specification]
#[verifier::external_body]
pub(super) struct ExTopology(Topology);

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExInitialRegs(InitialRegs);

// (`StopGuard` is `verus_verify` itself, `partition_unit.rs`.)

/// The partition requests' error types, declared to Verus as opaque types
/// (`PartitionRequest`'s variants).
#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExInitialRegError(super::InitialRegError);

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExAcceptInitialPagesError(super::AcceptInitialPagesError);

} // verus!
