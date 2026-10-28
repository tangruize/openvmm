// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

// Declarations supporting the microVM devices' spec; no human review needed.

use super::super::MicrovmPortbSavedState;
use super::super::MicrovmShutdown;
use super::super::PortbTimeAbi;
use chipset_resources::microvm::MicrovmPortbDrain;
use serial_core::SerialIo;
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

// Portb (`MicrovmPortb` is `verus_verify` itself: its fields are private).

#[verifier::external_type_specification]
pub struct ExMicrovmPortbSavedState(MicrovmPortbSavedState);

// The types of portb's other fields, as opaque types (`mesh::Receiver` and
// `Rpc` are declared in `mesh_rpc_verus`, `Waker` in vmcore).

#[verifier::external_type_specification]
#[verifier::external_body]
pub(crate) struct ExPortbTimeAbi(PortbTimeAbi);

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExMicrovmPortbDrain(MicrovmPortbDrain);

#[verifier::external_trait_specification]
pub trait ExAsyncRead {
    type ExternalTraitSpecificationFor: futures::AsyncRead;
}

#[verifier::external_trait_specification]
pub trait ExAsyncWrite {
    type ExternalTraitSpecificationFor: futures::AsyncWrite;
}

#[verifier::external_trait_specification]
pub trait ExSerialIo: futures::AsyncRead + futures::AsyncWrite + Send + inspect::InspectMut + Unpin {
    type ExternalTraitSpecificationFor: SerialIo;
}

} // verus!
