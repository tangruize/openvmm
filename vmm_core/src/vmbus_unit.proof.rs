// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

// OUT OF SCOPE (vmbus): the Verus spec in this file is parked. vmbus is not
// on the microVM's snapshot path (`snapshot-spec-docs/DESIGN.md`, "Out of scope
// for now"). Do not attempt to prove anything here. (Kept because the
// `StateUnit` trait requires the vmbus units to follow its protocol, and so
// that the work is not lost if vmbus comes back into scope.)

// Declarations supporting `vmbus_unit.spec.rs`; no human review needed.

use super::VmbusServerUnit;
use vstd::prelude::*;

verus! {

// (The channel units' device-type bounds, `VmbusDevice`, `SimpleVmbusDevice`,
// `RingMem`, `GpadlRingMem`, `Any`, are declared in `vmbus_channel`.)

/// Declares `VmbusServerUnit` to Verus, by name only (fields hidden).
#[verifier::external_type_specification]
#[verifier::external_body]
pub(super) struct ExVmbusServerUnit(VmbusServerUnit);

} // verus!
