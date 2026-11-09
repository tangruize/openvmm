//! Specifications for the standard-library types `std::path::Path` and
//! `std::path::PathBuf` (OpenVMM's patch vstd-0003): their declarations, and
//! their view, `PathView`.
//!
//! A path's view is the sequence of its components, each as the bytes of its
//! name. That is meaningful for relative paths of normal components only (no
//! root, no `.`, no `..`), which are the paths OpenVMM's virtio-fs spec talks
//! about: its microVM profile checks that its paths have that form when it
//! parses them. The view is uninterpreted: `std` is external, so what it
//! returns is a trusted fact about `std`.

use super::super::prelude::*;
use std::path::{Path, PathBuf};

verus! {

/// The view of a `Path` or a `PathBuf`: its components, each the bytes of
/// its name. The empty sequence is the empty path.
pub type PathView = Seq<Seq<u8>>;

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExPathBuf(PathBuf);

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExPath(Path);

impl View for PathBuf {
    type V = PathView;

    /// The path's components, each the bytes of its name.
    uninterp spec fn view(&self) -> PathView;
}

impl View for Path {
    type V = PathView;

    /// The path's components, each the bytes of its name.
    uninterp spec fn view(&self) -> PathView;
}

} // verus!
