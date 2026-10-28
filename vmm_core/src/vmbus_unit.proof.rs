// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

// Declarations supporting `vmbus_unit.spec.rs`; no human review needed.

use super::VmbusServerUnit;
use vstd::prelude::*;

verus! {

// (The channel units' device-type bounds, `VmbusDevice`, `SimpleVmbusDevice`,
// `RingMem`, `GpadlRingMem`, `Any`, are declared in `vmbus_channel`.)

/// Declares `VmbusServerUnit` to Verus, as an opaque type.
#[verifier::external_type_specification]
#[verifier::external_body]
pub(super) struct ExVmbusServerUnit(VmbusServerUnit);

} // verus!
