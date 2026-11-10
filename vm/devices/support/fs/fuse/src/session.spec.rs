// Spec of `Session`, for human review.

use super::Session;
use crate::spec::FuseGhost;
use crate::spec::FuseOwns;
use crate::spec::FuseState;
use vstd::prelude::*;

verus! {

impl<G: FuseGhost> Session<G> where G::V: FuseState {
    /// The session's file system owns ghost state `g` (`FuseOwns`): the
    /// session only passes `g` on to its file system's operations.
    // STYLE OK: a relation between the session and a ghost state, which
    // `dispatch`'s contract and the trusted ghost keeper state; a session has
    // no view of its own.
    pub closed spec fn owns_ghost(&self, g: &G) -> bool {
        (*self.fs).owns(g)
    }
}

} // verus!
