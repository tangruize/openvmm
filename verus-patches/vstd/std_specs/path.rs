//! Specifications for the standard-library types `std::path::Path` and
//! `std::path::PathBuf` (OpenVMM's patch vstd-0003): their declarations, and
//! their view, `PathView`.
//!
//! A path's view is the sequence of its components, as `Path::components`
//! yields them, each as bytes: a normal component as its name's bytes, the
//! root as `/`, `.` (which `components` yields only at the start) as `.`,
//! and `..` as `..` (on Windows, a prefix as its text). So a path is
//! relative and of normal components only exactly when every component of
//! its view is a plain name (not empty, no `/` and no NUL, not `.` or `..`):
//! the form that OpenVMM's microVM profile checks its paths have, and that
//! `lxutil` requires of the paths it is given (`LxVolume::is_share_path`).
//! The view is uninterpreted: `std` is external, so what it returns is a
//! trusted fact about `std`.

use super::super::prelude::*;
use super::convert::AsRefSpecImpl;
use std::path::{Path, PathBuf};

verus! {

/// The view of a `Path` or a `PathBuf`: its components, each as bytes (see
/// above). The empty sequence is the empty path.
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

// `AsRef<Path>` for `Path` and `PathBuf` (OpenVMM's patch vstd-0004), so
// that a contract of a function taking `impl AsRef<Path>` can name the path
// it is given: a `Path` refers to itself, a `PathBuf` to a `Path` with the
// same view. Trusted facts about `std`.

/// A `Path` refers to itself.
impl AsRefSpecImpl<Path> for Path {
    open spec fn obeys_as_ref_spec() -> bool {
        true
    }

    open spec fn as_ref_spec(&self) -> &Path {
        self
    }
}

pub assume_specification<'a>[ <Path as AsRef<Path>>::as_ref ](p: &'a Path) -> (ret: &'a Path)
    ensures
        ret == p,
;

/// The `Path` that a `PathBuf` refers to (`as_ref`, or its `Deref`).
pub uninterp spec fn path_buf_as_path(p: &PathBuf) -> &Path;

/// It has the `PathBuf`'s view.
pub broadcast axiom fn axiom_path_buf_as_path_view(p: &PathBuf)
    ensures
        (#[trigger] path_buf_as_path(p))@ == p@,
;

/// A `PathBuf` refers to a `Path` with its view.
impl AsRefSpecImpl<Path> for PathBuf {
    open spec fn obeys_as_ref_spec() -> bool {
        true
    }

    open spec fn as_ref_spec(&self) -> &Path {
        path_buf_as_path(self)
    }
}

pub assume_specification<'a>[ <PathBuf as AsRef<Path>>::as_ref ](p: &'a PathBuf) -> (ret: &'a Path)
    ensures
        ret == path_buf_as_path(p),
;

} // verus!
