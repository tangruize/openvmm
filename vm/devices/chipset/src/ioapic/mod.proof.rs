// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

// Declarations supporting the IOAPIC's spec; no human review needed.

use super::super::super::IoApicStats;
use super::super::super::spec::IndexRegister;
use chipset_resources::ioapic::IoApicRouting;
use vstd::prelude::*;

verus! {

// The IOAPIC's types, visible to Verus. (`IoApicDevice`, `IrqEntry`,
// `RedirectionEntry`, and the saved `SavedState` are `verus_verify`
// themselves: their fields are private to their modules.)

#[verifier::external_type_specification]
pub struct ExIndexRegister(IndexRegister);

// Runtime glue, as opaque types.

#[verifier::external_type_specification]
#[verifier::external_body]
pub(crate) struct ExIoApicStats(IoApicStats);

#[verifier::external_trait_specification]
pub trait ExIoApicRouting: Send + Sync {
    type ExternalTraitSpecificationFor: IoApicRouting;
}

} // verus!
