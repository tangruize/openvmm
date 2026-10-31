// The partition's VM state: declarations, and the token's trusted
// creation (`snapshot-spec-docs/DESIGN.md`, "The partition's state").

use hvdef::Vtl;
use vstd::prelude::*;

verus! {

/// Declares `Vtl` to Verus, with its variants (once, here; `vmm_core` and
/// `openvmm_core` use this declaration).
#[verifier::external_type_specification]
pub struct ExVtl(Vtl);

/// Declares `InitialVpStateSource` to Verus, by name only (fields hidden).
#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExInitialVpStateSource(super::InitialVpStateSource);

/// Declares `InitialPageImport` to Verus, by name only (fields hidden).
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


} // verus!
