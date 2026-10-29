// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

//! Infrastructure to define VM state.

pub(crate) use macros::state_trait;

use hvdef::HvRegisterValue;
use inspect::Inspect;
use std::fmt::Debug;
use thiserror::Error;

pub trait StateElement<C, V>: Sized + Inspect {
    /// Returns whether this state is present for the partition, based on the
    /// partition capabilities.
    fn is_present(caps: &C) -> bool;

    /// Returns the value at VM reset.
    fn at_reset(caps: &C, vp: &V) -> Self;

    /// Returns whether it's possible to read this value and compare it to an
    /// expected value.
    ///
    /// This will be false when the value may change as soon as its set (e.g., a
    /// timestamp counter for hypervisors that cannot freeze time).
    fn can_compare(_caps: &C) -> bool {
        true
    }
}

pub trait HvRegisterState<T, const COUNT: usize>: Default {
    fn names(&self) -> &'static [T; COUNT];
    fn get_values<'a>(&self, it: impl Iterator<Item = &'a mut HvRegisterValue>);
    fn set_values(&mut self, it: impl Iterator<Item = HvRegisterValue>);
}

#[derive(Debug, Error)]
#[error("state access error, phase {phase}")]
pub struct StateError<T: 'static + Debug + std::error::Error> {
    pub(crate) phase: &'static str,
    #[source]
    pub(crate) err: T,
}

/// Verus: `std::error::Error`, declared once for the state traits' `Error`
/// bound.
#[cfg(verus_keep_ghost)]
mod error_spec {
    use vstd::prelude::*;

    verus! {
        #[verifier::external_trait_specification]
        pub trait ExError: core::fmt::Debug + core::fmt::Display {
            type ExternalTraitSpecificationFor: core::error::Error;
        }

        /// `StateError`, as an opaque type (`save_all` / `restore_all`'s
        /// errors).
        #[verifier::external_type_specification]
        #[verifier::external_body]
        #[verifier::reject_recursive_types(T)]
        pub struct ExStateError<T: 'static + core::fmt::Debug + core::error::Error>(
            super::StateError<T>,
        );
    }
}

mod macros {
    /// Generates a trait for getting and setting some aspect of partition state
    /// (e.g. per-partition state, per-VP state, per-VTL state).
    ///
    /// The trait is made up of methods for getting and setting individual
    /// pieces of state. Each piece can be individually present or missing for a
    /// partition, e.g. because certain processor features are enabled or not.
    ///
    /// A function is generated that will reset each individual piece of state.
    ///
    /// Another generated function will validate that each piece of state is
    /// equal to its reset state (which is useful to validate that a partition's
    /// initial state is equal to its reset state).
    ///
    /// The intent is that the partition types implement these traits to get and
    /// set individual pieces of state.
    macro_rules! state_trait {
        ($doc:tt, $trait:ident, $caps:ty, $vp:ty, $save_state:ident, $package:expr,
            $(($field_num:literal, $id:expr, $get:ident, $set:ident, $ty:ty $(,)?)),* $(,)?
        ) => {
            #[doc = $doc]
            #[expect(clippy::allow_attributes)]
            pub trait $trait {
                type Error: 'static + std::error::Error + Send + Sync;

                /// Gets the partition's capabilities.
                fn caps(&self) -> &$caps;

                /// Commits any state changes made with the `set_*` methods.
                fn commit(&mut self) -> Result<(), Self::Error>;

                $(
                /// Gets the specified state.
                fn $get(&mut self) -> Result<$ty, Self::Error>;
                /// Sets the specified state.
                fn $set(&mut self, value: &$ty) -> Result<(), Self::Error>;
                )*

                /// Save all state that can be restored by a call to restore.
                #[allow(unused_mut)]
                fn save_all(&mut self) -> Result<$save_state, $crate::state::StateError<Self::Error>> {
                    let mut save_state = $save_state::default();
                    $(
                        if <$ty as $crate::state::StateElement<$caps, $vp>>::is_present(self.caps()) {
                            save_state.$get = Some(self.$get().map_err(|err| $crate::state::StateError{phase: concat!("save ", stringify!($id)), err})?);
                        }
                    )*

                    Ok(save_state)
                }

                /// Restore state elements saved in save state.
                #[allow(unused_variables)]
                fn restore_all(&mut self, state: &$save_state) -> Result<(), $crate::state::StateError<Self::Error>> {
                    $(
                        if let Some(value) = state.$get.as_ref() {
                            // TODO: assert good or not?
                            assert!(<$ty as $crate::state::StateElement<$caps, $vp>>::is_present(self.caps()));
                            self.$set(value).map_err(|err| $crate::state::StateError{phase: concat!("restore ", stringify!($id)), err})?;
                            if cfg!(debug_assertions) {
                                if <$ty as $crate::state::StateElement<$caps, $vp>>::can_compare(self.caps()) {
                                    assert_eq!(&self.$get().expect($id), value, "restore state mismatch (actual/expected)");
                                }
                            }
                        }
                    )*

                    self.commit().map_err(|err| $crate::state::StateError{phase: "commit restore", err})
                }

                /// Resets all the state elements to their initial state (after machine reset).
                fn reset_all(&mut self, vp_info: &$vp) -> Result<(), $crate::state::StateError<Self::Error>> {
                    $(
                        if <$ty as $crate::state::StateElement<$caps, $vp>>::is_present(self.caps()) {
                            self.$set(&<$ty as $crate::state::StateElement<$caps, $vp>>::at_reset(self.caps(), vp_info)).map_err(|err| $crate::state::StateError{phase: concat!("reset ", stringify!($id)), err})?;
                        }
                    )*

                    if cfg!(debug_assertions) {
                        self.check_reset_all(vp_info);
                    }
                    Ok(())
                }

                /// Validates that all state elements are in their initial state (after machine reset).
                #[allow(unused_variables)]
                fn check_reset_all(&mut self, vp_info: &$vp) {
                    $(
                        if <$ty as $crate::state::StateElement<$caps, $vp>>::can_compare(self.caps()) && <$ty as $crate::state::StateElement<$caps, $vp>>::is_present(self.caps()) {
                            assert_eq!(self.$get().expect($id), <$ty as $crate::state::StateElement<$caps, $vp>>::at_reset(self.caps(), vp_info), "reset state mismatch (actual/expected)");
                        }
                    )*
                }

                #[allow(unused_variables, unused_mut)]
                fn inspect_all(&mut self) -> impl ::inspect::InspectMut {
                    inspect::adhoc_mut(|req| {
                        let mut resp = req.respond();
                        $(
                            if <$ty as $crate::state::StateElement<$caps, $vp>>::is_present(self.caps()) {
                                resp.field_with($id, || self.$get().ok());
                            }
                        )*
                    })
                }
            }

            /// Saved state type that can be used to save or restore.
            #[derive(Debug, Default, PartialEq, Eq, mesh_protobuf::Protobuf, vmcore::save_restore::SavedStateRoot)]
            #[mesh(package = $package)]
            #[cfg_attr(verus_keep_ghost, ::vstd::prelude::verus_verify)]
            #[cfg_attr(verus_keep_ghost, verifier::external_derive)]
            pub struct $save_state {
                $(
                    #[mesh($field_num)]
                    $get: Option<$ty>,
                )*
            }

            /// Verus: the trusted contract of the hypervisor boundary for
            /// this state (`SNAPSHOT-SPEC-DESIGN.md`, "The hypervisor
            /// boundary"). Each backend's implementation of the trait above
            /// talks to the hypervisor; its state is the abstract
            /// `hv_state()`, a value of the saved-state type: each element's
            /// current value.
            #[cfg(verus_keep_ghost)]
            pub mod state_spec {
                #[allow(unused_imports)]
                use super::*;
                use ::vstd::prelude::*;

                // The element types, declared to Verus as opaque types
                // (values the hypervisor holds; their contents are not
                // specified).
                $(
                    #[allow(non_snake_case)]
                    mod $get {
                        #[allow(unused_imports)]
                        use super::super::*;
                        use ::vstd::prelude::*;

                        verus! {
                            #[verifier::external_type_specification]
                            #[verifier::external_body]
                            pub struct Ex($ty);
                        }
                    }
                )*

                verus! {

                impl $save_state {
                    $(
                        /// The element's value, if present.
                        pub closed spec fn $get(self) -> Option<$ty> {
                            self.$get
                        }

                        /// This state with the element set to `value`.
                        pub closed spec fn $set(self, value: $ty) -> Self {
                            Self { $get: Some(value), ..self }
                        }
                    )*

                    /// This state with every element present in `other` set
                    /// to `other`'s value (what `restore_all(other)` does).
                    pub closed spec fn overwrite(self, other: Self) -> Self {
                        Self {
                            $(
                                $get: if other.$get is Some { other.$get } else { self.$get },
                            )*
                        }
                    }
                }

                /// NOT TRUSTED: the getters', setters', and `commit`'s
                /// contracts are OBLIGATIONS of each backend's implementation
                /// (the user's decision, 2026-10-30: the trust boundary is the
                /// hypervisor's API, not OpenVMM's backends, which have
                /// significant code). To prove, per backend, against the
                /// trusted contracts of its hypervisor API (in a `_t.rs` file
                /// of the backend's crate): KVM (`virt_kvm`) and MSHV
                /// (`virt_mshv`) are in scope; WHP and hvf (Windows and macOS
                /// hosts) are out of scope. `hv_state()` is the handle's view
                /// of the hypervisor's state, each backend defining it. A
                /// getter returns the element's current value and changes
                /// nothing; a setter sets the element (and only it); `commit`
                /// changes nothing (every host backend applies a set at
                /// once). On an error from a getter, nothing changes; on an
                /// error from a setter, the state is unspecified. None of
                /// them changes the handle's `hv_id()`.
                ///
                /// NOT TRUSTED, PROOF TODO: `save_all` and `restore_all`, the
                /// trait's default methods generated above (our code). Their
                /// contracts are stated here and are to prove from the
                /// getters' and setters' contracts; that needs the generated
                /// trait to be `verus_verify`, so that their bodies are
                /// checked.
                #[verifier::external_trait_specification]
                #[verifier::external_trait_extension(StateAccessSpec via StateAccessSpecImpl)]
                pub trait ExStateAccess {
                    type ExternalTraitSpecificationFor: super::$trait;

                    type Error: 'static + std::error::Error + Send + Sync;

                    /// The hypervisor's state, as each element's current
                    /// value (`None` for an element not present: its
                    /// `is_present` is false) (TRUSTED: uninterpreted; the
                    /// hypervisor's).
                    spec fn hv_state(&self) -> $save_state;

                    /// The id of the state token this handle was opened
                    /// with (`open_vm_state`; uninterpreted).
                    spec fn hv_id(&self) -> ::vstd::resource::Loc;

                    fn commit(&mut self) -> (result: Result<(), Self::Error>)
                        ensures
                            final(self).hv_state() == old(self).hv_state(),
                            final(self).hv_id() == old(self).hv_id(),
                    ;

                    fn save_all(&mut self) -> (result: Result<$save_state, $crate::state::StateError<Self::Error>>)
                        ensures
                            final(self).hv_state() == old(self).hv_state(),
                            final(self).hv_id() == old(self).hv_id(),
                            result matches Ok(s) ==> s == old(self).hv_state(),
                    ;

                    fn restore_all(&mut self, state: &$save_state) -> (result: Result<(), $crate::state::StateError<Self::Error>>)
                        ensures
                            final(self).hv_id() == old(self).hv_id(),
                            result is Ok ==> final(self).hv_state() == old(self).hv_state().overwrite(*state),
                    ;

                    $(
                        fn $get(&mut self) -> (result: Result<$ty, Self::Error>)
                            ensures
                                final(self).hv_state() == old(self).hv_state(),
                                final(self).hv_id() == old(self).hv_id(),
                                result matches Ok(v) ==> old(self).hv_state().$get() == Some(v),
                        ;

                        fn $set(&mut self, value: &$ty) -> (result: Result<(), Self::Error>)
                            ensures
                                final(self).hv_id() == old(self).hv_id(),
                                result is Ok ==> final(self).hv_state() == old(self).hv_state().$set(*value),
                        ;
                    )*
                }

                } // verus!
            }
        };
    }

    pub(crate) use state_trait;
}
