// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

// Declarations supporting the dual PIC's spec; no human review needed.

use super::super::super::DualPicStats;
use super::super::super::InitStage;
use super::super::super::Pic;
use super::super::state::SavedInitStage;
use super::super::state::SavedPic;
use chipset_device::pio::ControlPortIoIntercept;
use vstd::prelude::*;

verus! {

// Make the PIC's types visible to Verus. Verus checks that their fields
// match the real definitions. (`DualPic`, `Ocw3`, and `SavedState` are
// `verus_verify` themselves: their fields are private to their modules.)

#[verifier::external_type_specification]
pub(crate) struct ExPic(Pic);

#[verifier::external_type_specification]
pub(crate) struct ExInitStage(InitStage);

#[verifier::external_type_specification]
pub struct ExSavedPic(SavedPic);

#[verifier::external_type_specification]
pub struct ExSavedInitStage(SavedInitStage);

// Runtime glue, as opaque types (`LineInterrupt` is declared with the PIT).

#[verifier::external_type_specification]
#[verifier::external_body]
pub(crate) struct ExDualPicStats(DualPicStats);

#[verifier::external_trait_specification]
pub trait ExControlPortIoIntercept: Send + Sync {
    type ExternalTraitSpecificationFor: ControlPortIoIntercept;
}

} // verus!
