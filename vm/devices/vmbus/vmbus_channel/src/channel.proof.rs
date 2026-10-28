// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

// Declarations supporting `channel.spec.rs`; no human review needed.

use super::StateRequest;
use vstd::prelude::*;

verus! {

/// `core::any::Any`, declared to Verus (no methods), so that Verus knows
/// `IntoAny`'s blanket impl for every `T: Any`.
#[verifier::external_trait_specification]
pub trait ExAny: 'static {
    type ExternalTraitSpecificationFor: core::any::Any;
}

/// `IntoAny`, declared to Verus (no methods).
#[verifier::external_trait_specification]
pub trait ExIntoAny {
    type ExternalTraitSpecificationFor: crate::channel::IntoAny;
}

/// Declares the channel's internal `StateRequest` (its variants) to Verus.
#[verifier::external_type_specification]
pub(crate) struct ExStateRequest(StateRequest);

/// Declares `ChannelHandle` to Verus, as an opaque type (its
/// `PhantomData<fn() -> Box<T>>` field is not supported).
#[verifier::external_type_specification]
#[verifier::external_body]
#[verifier::reject_recursive_types(T)]
pub struct ExChannelHandle<T: ?Sized>(super::ChannelHandle<T>);

/// Declares the channel task's state (`Device`) to Verus, as an opaque type.
#[verifier::external_type_specification]
#[verifier::external_body]
pub(crate) struct ExDevice(super::Device);

/// Declares `ParentBus` to Verus (no methods).
#[verifier::external_trait_specification]
pub trait ExParentBus: Send + Sync {
    type ExternalTraitSpecificationFor: crate::bus::ParentBus;
}

/// Declares `pal_async::task::Spawn` to Verus (no methods).
#[verifier::external_trait_specification]
pub trait ExSpawn: Send + Sync {
    type ExternalTraitSpecificationFor: pal_async::task::Spawn;
}

/// Declares `GenericChannelHandle` to Verus, as an opaque type.
#[verifier::external_type_specification]
#[verifier::external_body]
pub(crate) struct ExGenericChannelHandle(super::GenericChannelHandle);

} // verus!
