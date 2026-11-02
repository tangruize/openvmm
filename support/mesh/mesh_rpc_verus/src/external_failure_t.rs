// Trusted: external failures.
//
// Assumes nothing by itself. `ExternalFailure::happened()` is
// uninterpreted: only the contracts of trusted functions that call code
// outside the verified code (mesh, the hypervisor, the host OS, the clock)
// establish it, each on its own error results. A verified function can
// therefore return an error for an external reason only after such a call
// has failed.

use vstd::prelude::*;

verus! {

/// External failures (spec only).
pub struct ExternalFailure;

impl ExternalFailure {
    /// A call outside the verified code has failed: a mesh channel (its
    /// peer gone or closed), the hypervisor, the host OS, or a deadline.
    /// Established only by trusted functions' contracts, on their errors
    /// (e.g. `PendingCall::recv`). Contracts use it in their error
    /// conditions: "fails only if ... or an external failure happened".
    ///
    /// One fact for all external failures: after any one, every contract
    /// that admits external failures may fail. Failures that come from the
    /// verified code's own state (e.g. a device that refuses to save) are
    /// stated exactly, without it.
    pub uninterp spec fn happened() -> bool;
}

} // verus!
