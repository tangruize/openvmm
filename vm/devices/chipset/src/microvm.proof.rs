// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

// Declarations supporting the microVM devices' spec; no human review needed.

use super::super::MicrovmShutdown;
use super::super::MicrovmSnapshotRequest;
use vstd::prelude::*;

verus! {

// The devices, as opaque types: their saved state does not depend on their
// fields.

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExMicrovmShutdown(MicrovmShutdown);

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExMicrovmSnapshotRequest(MicrovmSnapshotRequest);

} // verus!
