// Declarations supporting `policy.spec.rs`; no human review needed.

use crate::microvm::profile::MicroVmProfileError;
use vstd::prelude::*;

verus! {

/// Declares `MicroVmProfileError` to Verus, by name only (fields hidden).
#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExMicroVmProfileError(MicroVmProfileError);

} // verus!
