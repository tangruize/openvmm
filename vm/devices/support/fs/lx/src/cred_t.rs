// The calling thread's host credentials, as a ghost token (written
// 2026-11-11, for property 3, identity).
//
// TRUSTED, do not attempt to prove anything in this file. Verus cannot see
// a thread's credentials, so the token is declared only, and its view is
// trusted to be as whom the thread performs host calls now. Only trusted
// code changes it: `lxutil`'s `switch_fs_identity` and its guard's
// `restore` (`lxutil/src/unix/credentials.rs`), and the keeper hands the one
// token out with the ghost state (`virtiofs/src/fs_ghost_t.rs`).

use vstd::prelude::*;

verus! {

/// As whom a thread performs host calls.
pub enum ThreadCredView {
    /// With its own credentials: OpenVMM's.
    Own,
    /// With filesystem user `uid` and group `gid`, no other supplementary
    /// group, and no effective capability.
    Switched { uid: u32, gid: u32 },
}

/// The calling thread's credentials. TRUSTED: its view is as whom the
/// thread performs host calls now. Declared only.
#[verifier::external_body]
pub struct ThreadCred {
    _private: (),
}

impl View for ThreadCred {
    type V = ThreadCredView;

    uninterp spec fn view(&self) -> ThreadCredView;
}

} // verus!
