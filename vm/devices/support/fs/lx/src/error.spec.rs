// Spec of `Error`, for human review.

use super::Error;
use vstd::prelude::*;

verus! {

impl Error {
    /// The Linux error number, e.g. `lx::EROFS`.
    pub closed spec fn code(self) -> i32 {
        self.0
    }
}

} // verus!
