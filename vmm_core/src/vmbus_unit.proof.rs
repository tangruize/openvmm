// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

// Declarations supporting `vmbus_unit.spec.rs`; no human review needed.

use super::ChannelUnit;
use super::SimpleChannelUnit;
use super::VmbusServerUnit;
use vmbus_channel::channel::VmbusDevice;
use vmbus_channel::gpadl_ring::GpadlRingMem;
use vmbus_channel::simple::SimpleVmbusDevice;
use vmbus_ring::RingMem;
use vmcore::save_restore::SavedStateRoot;
use vstd::prelude::*;

verus! {

// The bounds of the channel units' device types, declared to Verus (no
// methods: the units' contracts do not use them).

#[verifier::external_trait_specification]
pub trait ExAny: 'static {
    type ExternalTraitSpecificationFor: core::any::Any;
}

#[verifier::external_trait_specification]
pub trait ExVmbusDevice: Send + core::any::Any + inspect::InspectMut {
    type ExternalTraitSpecificationFor: VmbusDevice;
}

#[verifier::external_trait_specification]
pub trait ExRingMem: Send {
    type ExternalTraitSpecificationFor: RingMem;
}

/// Declares `GpadlRingMem` to Verus, as an opaque type.
#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExGpadlRingMem(GpadlRingMem);

#[verifier::external_trait_specification]
pub trait ExSimpleVmbusDevice<M: RingMem = GpadlRingMem>: 'static + Send {
    type ExternalTraitSpecificationFor: SimpleVmbusDevice<M>;

    type SavedState: SavedStateRoot + Send;

    type Runner: 'static + Send;
}

/// Declares `VmbusServerUnit` to Verus, as an opaque type.
#[verifier::external_type_specification]
#[verifier::external_body]
pub(super) struct ExVmbusServerUnit(VmbusServerUnit);

/// Declares `ChannelUnit` to Verus, as an opaque type.
#[verifier::external_type_specification]
#[verifier::external_body]
#[verifier::reject_recursive_types(T)]
pub struct ExChannelUnit<T: ?Sized>(ChannelUnit<T>);

/// Declares `SimpleChannelUnit` to Verus, as an opaque type.
#[verifier::external_type_specification]
#[verifier::external_body]
#[verifier::reject_recursive_types(T)]
pub struct ExSimpleChannelUnit<T: SimpleVmbusDevice>(SimpleChannelUnit<T>);

} // verus!
