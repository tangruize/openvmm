// Spec of `LxStr`, for human review.

use super::LxStr;
use vstd::prelude::*;

verus! {

/// The view of an `LxStr`: its bytes.
impl View for LxStr {
    type V = Seq<u8>;

    closed spec fn view(&self) -> Seq<u8> {
        self.bytes@
    }
}

} // verus!
