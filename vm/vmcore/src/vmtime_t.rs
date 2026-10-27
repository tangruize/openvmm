// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

// Trusted: vmtime's saved state is all time.
//
// Everything in this file is TRUSTED, and listed in
// `SNAPSHOT-SPEC-TRUSTED.md`.

use super::SavedState;
use crate::save_restore::spec::SavedStateBlobView;
use vstd::prelude::*;

verus! {

impl SavedState {
    /// AXIOM: every field of vmtime's `SavedState` (`vmtime`,
    /// `stop_host_time`) is time, so any two are equal except for time
    /// (`equal_except_time`, trusted per type).
    #[verifier::external_body]
    pub broadcast proof fn axiom_all_time(a: SavedState, b: SavedState)
        ensures
            #[trigger] SavedStateBlobView::equal_except_time(a, b),
    {
    }
}

} // verus!
