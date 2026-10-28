// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

//! Traits and types for save/restore support.
//!
//! To implement save/restore for your device or state unit, you must do these
//! things:
//!
//! 1. You define a saved state type. This type needs to be stable across
//!    releases, so you should not share types with your implementation even if
//!    they are currently identical. By decoupling your runtime types and your
//!    saved state types, you are much less likely to accidentally break saved
//!    state compatibility.
//!
//! 2. `derive` some traits on your type. Specifically, you must derive
//!    [`Protobuf`] so that your type can be encoded into protobuf format, and
//!    any fields in your type must also implement [`Protobuf`].
//!
//!    So that your saved state type can show up in generated `.proto` files,
//!    you must also set an attribute `#[mesh(package = "my.package.name")]`.
//!    This specifies the protobuf package, as well as the file name of your
//!    protobuf file. For the root object of your device's saved state, this
//!    package name becomes part of the serialized state, so you cannot change
//!    it later.
//!
//!    Finally, in the root object of your saved state, you must additionally
//!    derive [`SavedStateRoot`]. This provides some additional metadata so that
//!    we can find your saved state type and generate a `.proto` file from it
//!    for analysis. You only need to put this on the root types or types that
//!    will be converted to [`SavedStateBlob`]; the infrastructure will find any
//!    dependent types. But it doesn't hurt to put it on other types, too.
//!
//! 3. Typically, you implement the [`SaveRestore`] trait. This trait allows you
//!    to specify your associated saved state type and to implement `save` and
//!    `restore` methods that act on this type.
//!
//!    For some device types (such as vmbus devices), you may need to use a
//!    device-specific trait that provides additional parameters. But the
//!    pattern should be the same.

// UNSAFETY: Needed to use linkme for deriving SavedStateRoot.
#![expect(unsafe_code)]

/// Derives [`SavedStateRoot`] for a type.
///
/// This ensures that a saved state blob's metadata can be found, so that it can
/// be used to generate .proto files and perform offline analysis of saved state
/// compatibility.
///
/// To use this, you must also derive [`Protobuf`] and set a protobuf package
/// for your type. The package name should be defined to group related types
/// together; typically the same package should be used for all types defined in
/// a module.
///
/// For example:
///
/// ```rust
/// # use vmcore::save_restore::{SavedStateRoot, SavedStateBlob};
/// # use mesh::payload::Protobuf;
/// #[derive(Protobuf, SavedStateRoot)]
/// #[mesh(package = "test.my_device")]
/// struct MySavedState {
///     #[mesh(1)]
///     active: bool,
/// }
///
/// // This will now compile.
/// let _blob = SavedStateBlob::new(MySavedState { active: true });
/// ```
pub use save_restore_derive::SavedStateRoot;

use mesh::payload;
use mesh::payload::DefaultEncoding;
use mesh::payload::DescribedProtobuf;
use mesh::payload::Protobuf;
use mesh::payload::encoding::ImpossibleField;
use mesh::payload::message::ProtobufAny;
use mesh::payload::protofile::MessageDescription;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

/// What `SaveRestore`'s contract needs of an object: in Verus builds, a
/// view (see `spec::SaveRestoreView`). Empty in normal builds.
#[cfg(not(verus_keep_ghost))]
pub trait SaveRestoreView {}

#[cfg(not(verus_keep_ghost))]
impl<T: ?Sized> SaveRestoreView for T {}

#[cfg(verus_keep_ghost)]
pub use spec::SaveRestoreView;

/// Verus builds: declares the `SaveRestore` type `$t` to Verus (as the
/// opaque type `$ex`) and gives it the placeholder view `()`, for a device
/// whose save/restore is not yet specified (`SaveRestore` needs a view).
/// `placeholder_view!(saved T)` gives the saved-state type `T` of such a
/// device a placeholder view, also outside Verus (`SaveRestore::SavedState`
/// needs a view). For a generic type, name its parameters on `$ex`, e.g.
/// `placeholder_view!(pub ExFoo<T>, Foo<T>)`. Expands to nothing in normal
/// builds.
#[macro_export]
macro_rules! placeholder_view {
    (saved $t:ty) => {
        // Outside `verus!`, so external to Verus: only rustc needs it (the
        // device's `SaveRestore` impl, outside `verus!`, names the type).
        #[cfg(verus_keep_ghost)]
        impl ::vstd::prelude::View for $t {
            type V = $t;

            fn view(&self) -> $t {
                unreachable!()
            }
        }
    };
    (external [$($gen:tt)*] $t:ty) => {
        // Outside `verus!`, so external to Verus: only rustc needs it.
        #[cfg(verus_keep_ghost)]
        impl<$($gen)*> ::vstd::prelude::View for $t {
            type V = ();

            fn view(&self) {}
        }
    };
    ($vis:vis $ex:ident < $($lt:lifetime,)* $($g:ident),+ >, $t:ty) => {
        #[cfg(verus_keep_ghost)]
        ::vstd::prelude::verus! {
            /// Verus: the type, declared as opaque (placeholder view).
            #[verifier::external_type_specification]
            #[verifier::external_body]
            $(#[verifier::reject_recursive_types($g)])+
            $vis struct $ex<$($lt,)* $($g),+>($t);

            impl<$($lt,)* $($g),+> ::vstd::prelude::View for $t {
                type V = ();

                open spec fn view(&self) -> () {
                    ()
                }
            }
        }
    };
    ($vis:vis $ex:ident, $t:ty) => {
        #[cfg(verus_keep_ghost)]
        ::vstd::prelude::verus! {
            /// Verus: the type, declared as opaque (placeholder view).
            #[verifier::external_type_specification]
            #[verifier::external_body]
            $vis struct $ex($t);

            impl ::vstd::prelude::View for $t {
                type V = ();

                open spec fn view(&self) -> () {
                    ()
                }
            }
        }
    };
}

/// Implemented by objects which can be saved/restored with an associated
/// type that can be serialized as a protobuf message.
pub trait SaveRestore: SaveRestoreView {
    /// The concrete saved state type.
    type SavedState: SaveRestoreView;

    /// Saves the object's state.
    fn save(&mut self) -> Result<Self::SavedState, SaveError>;
    /// Restores the object's state.
    fn restore(&mut self, state: Self::SavedState) -> Result<(), RestoreError>;
}

/// Convenience type for objects that do not support being saved.
pub enum SavedStateNotSupported {}

// Verus builds: `SaveRestore::SavedState` needs a view; no values, so an
// empty one. Outside `verus!` (only rustc needs it).
#[cfg(verus_keep_ghost)]
impl View for SavedStateNotSupported {
    type V = ();

    fn view(&self) {}
}

impl DefaultEncoding for SavedStateNotSupported {
    type Encoding = ImpossibleField;
}

impl SavedStateRoot for SavedStateNotSupported {
    // This type should not be included in the .proto output.
    fn do_not_impl_this_manually(&self) {}
}

/// Convenience type for objects that have no saved state.
#[derive(Protobuf, SavedStateRoot)]
#[mesh(package = "save_restore")]
pub struct NoSavedState;

/// Trait implemented by objects that implement `SaveRestore`
pub trait ProtobufSaveRestore {
    /// Save the object.
    fn save(&mut self) -> Result<SavedStateBlob, SaveError>;
    /// Restore the object.
    fn restore(&mut self, state: SavedStateBlob) -> Result<(), RestoreError>;
}

/// An opaque saved state blob, encoded as a protobuf message.
#[derive(Clone, Debug, Protobuf)]
#[mesh(transparent)]
pub struct SavedStateBlob(ProtobufAny);

/// Trait implemented by "root" saved state blobs, which are ones that either
/// form the root of a saved state tree, or at points in the tree where the type
/// is not known at compile time.
///
/// **Do not implement this trait manually.** Derive it with
/// `#[derive(SavedStateRoot)]`. This emits extra code to ensure that the saved
/// state type metadata is included in the binary.
pub trait SavedStateRoot: DescribedProtobuf {
    #[doc(hidden)]
    fn do_not_impl_this_manually(&self);
}

impl SavedStateBlob {
    /// Encodes `data` as a protobuf message.
    pub fn new<T: SavedStateRoot>(data: T) -> Self {
        Self(ProtobufAny::new(data))
    }

    /// Decodes the protobuf message into `T`.
    pub fn parse<T: SavedStateRoot>(&self) -> Result<T, payload::Error> {
        self.0.parse()
    }

    /// Returns the encoded state payload length without decoding it.
    pub fn encoded_len(&self) -> usize {
        self.0.value_len()
    }
}

// The blanket impl below relates a saved state's blob to its view.
#[cfg(verus_keep_ghost)]
verus! {
    broadcast use spec::SavedStateBlobView::axiom_of_is_of_view;
}

// Verified against `ProtobufSaveRestore`'s contract
// (`save_restore.spec.rs`).
#[cfg_attr(verus_keep_ghost, verus_verify)]
impl<T: SaveRestore> ProtobufSaveRestore for T
where
    T::SavedState: 'static + Send + SavedStateRoot,
{
    fn save(&mut self) -> Result<SavedStateBlob, SaveError> {
        match SaveRestore::save(self) {
            Ok(state) => Ok(SavedStateBlob::new(state)),
            Err(err) => Err(err),
        }
    }

    fn restore(&mut self, state: SavedStateBlob) -> Result<(), RestoreError> {
        match state.parse() {
            Ok(state) => SaveRestore::restore(self, state),
            Err(err) => Err(RestoreError::from(err)),
        }
    }
}

/// A restore error.
#[derive(Debug, thiserror::Error)]
pub enum RestoreError {
    /// unknown entry ID
    #[error("unknown entry id: {0}")]
    UnknownEntryId(String),
    /// restore failure in a child object
    #[error("failed to restore child device {0}")]
    ChildError(String, #[source] Box<RestoreError>),
    /// failure to decode a protobuf object
    #[error("failed to decode protobuf")]
    ProtobufDecode(#[from] payload::Error),
    /// this object does not support save state
    #[error("unexpected saved state")]
    SavedStateNotSupported,
    /// custom saved state corruption error
    #[error("saved state is invalid")]
    InvalidSavedState(#[source] anyhow::Error),
    /// non-state-related restore failure
    #[error(transparent)]
    Other(anyhow::Error),
}

/// A save error.
#[derive(Debug, thiserror::Error)]
pub enum SaveError {
    /// This object does not support saved state.
    #[error("save state not supported")]
    NotSupported,
    /// Save failed in child object.
    #[error("failed to save child device {0}")]
    ChildError(String, #[source] Box<SaveError>),
    /// Save failed due to some other error.
    #[error(transparent)]
    Other(anyhow::Error),
    /// The child saved state is invalid.
    #[error("child saved state is invalid")]
    InvalidChildSavedState(#[source] anyhow::Error),
}

/// Gets the message descriptions for all types deriving [`SavedStateRoot`].
///
/// This can be used with
/// [`DescriptorWriter`](mesh::payload::protofile::DescriptorWriter) to write
/// `.proto` files for the saved states.
pub fn saved_state_roots() -> impl Iterator<Item = &'static MessageDescription<'static>> {
    private::SAVED_STATE_ROOTS.iter().flatten().copied()
}

// For `save_restore_derive`
#[doc(hidden)]
pub mod private {
    pub use linkme;
    pub use mesh::payload::protofile;

    // Use Option<&X> in case the linker inserts some stray nulls, as we think
    // it might on Windows.
    //
    // See <https://devblogs.microsoft.com/oldnewthing/20181108-00/?p=100165>.
    #[linkme::distributed_slice]
    pub static SAVED_STATE_ROOTS: [Option<&'static protofile::MessageDescription<'static>>] = [..];

    // Always have at least one entry to work around linker bugs.
    //
    // See <https://github.com/llvm/llvm-project/issues/65855>.
    #[linkme::distributed_slice(SAVED_STATE_ROOTS)]
    static WORKAROUND: Option<&'static protofile::MessageDescription<'static>> = None;

    #[doc(hidden)]
    #[macro_export]
    macro_rules! declare_saved_state_root {
        ($ident:ty) => {
            impl $crate::save_restore::SavedStateRoot for $ident {
                fn do_not_impl_this_manually(&self) {}
            }
            const _: () = {
                use $crate::save_restore::private::SAVED_STATE_ROOTS;
                use $crate::save_restore::private::linkme;
                use $crate::save_restore::private::protofile;

                #[linkme::distributed_slice(SAVED_STATE_ROOTS)]
                #[linkme(crate = linkme)]
                static DESCRIPTION: Option<&'static protofile::MessageDescription<'static>> =
                    Some(&protofile::message_description::<$ident>());
            };
        };
    }
}

/// Verus specification of saved-state blobs and of `SaveRestore`.
#[cfg(verus_keep_ghost)]
#[allow(missing_docs)] // items that Verus generates for the trait extension
pub mod spec {
    include!("save_restore.spec.rs");

    /// Examples supporting the spec. Inside `spec`, so that lemmas can
    /// unfold its closed views.
    mod proof {
        include!("save_restore.proof.rs");
    }
}

/// Trusted encode/decode contract of saved-state blobs.
#[cfg(verus_keep_ghost)]
mod trusted {
    include!("save_restore_t.rs");
}

#[cfg(test)]
mod tests {
    use super::NoSavedState;
    use super::SavedStateBlob;

    #[test]
    fn cloned_saved_state_blob_parses() {
        let state = SavedStateBlob::new(NoSavedState);
        state.clone().parse::<NoSavedState>().unwrap();
        state.parse::<NoSavedState>().unwrap();
    }
}
