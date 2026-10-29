// The partition's VM state: declarations, and the token's trusted
// creation (`SNAPSHOT-SPEC-DESIGN.md`, "The partition's state").

use super::PartitionAccessState;
use crate::vm::VmSavedState;
use hvdef::Vtl;
use vstd::prelude::*;
use vstd::resource::frac::FracGhost;

verus! {

/// Declares `Vtl` to Verus, with its variants (once, here; `vmm_core` and
/// `openvmm_core` use this declaration).
#[verifier::external_type_specification]
pub struct ExVtl(Vtl);

/// Declares `InitialVpStateSource` to Verus, as an opaque type.
#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExInitialVpStateSource(super::InitialVpStateSource);

/// Declares `InitialPageImport` to Verus, as an opaque type.
#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExInitialPageImport(super::InitialPageImport);

/// Declares `Hv1` and `Partition` to Verus, without their methods (the
/// blanket `BasicPartitionStateAccess` impl in `openvmm_core` names
/// `Partition` as a bound).
#[verifier::external_trait_specification]
pub trait ExHv1 {
    type ExternalTraitSpecificationFor: super::Hv1;
}

#[verifier::external_trait_specification]
pub trait ExPartition: 'static + super::Hv1 + inspect::Inspect + Send + Sync {
    type ExternalTraitSpecificationFor: super::Partition;
}

/// Creates the token of a partition's VM state (VTL 0): the whole
/// FracGhost, whose value is the state. TRUSTED: the value is the
/// partition's current state (unknown to the proof), and this is called
/// once per partition, when its `WrappedPartition` is built.
#[verifier::external_body]
#[allow(unused_variables)]
pub fn new_vm_state_token<P: PartitionAccessState>(partition: &P) -> (token: Tracked<
    Option<FracGhost<VmSavedState>>,
>)
    ensures
        token@ matches Some(t) && t.frac() == 1real,
{
    Tracked::assume_new()
}

} // verus!
