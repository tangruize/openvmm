// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

// Trusted: the PIT's saved state encoded as a blob, and decoded.
//
// Everything in this file is TRUSTED, and listed in
// `SNAPSHOT-SPEC-TRUSTED.md`. These are per-type instances of the stated
// `SavedStateBlob::new` / `parse` contract (`vm/vmcore/src/save_restore_t.rs`),
// used until that contract can be attached to `new` and `parse` themselves.

use super::super::save_restore::state::SavedState;
use vmcore::save_restore::RestoreError;
use vmcore::save_restore::SavedStateBlob;
use vmcore::save_restore::spec::SavedStateBlobView;
use vstd::prelude::*;

verus! {

/// Encodes the PIT's saved state.
#[verifier::external_body]
pub(in crate::pit) fn encode(state: SavedState) -> (blob: SavedStateBlob)
    ensures
        blob@ == SavedStateBlobView::of(state),
{
    SavedStateBlob::new(state)
}

/// Decodes the PIT's saved state.
#[verifier::external_body]
pub(in crate::pit) fn decode(blob: &SavedStateBlob) -> (result: Result<SavedState, RestoreError>)
    ensures
        result matches Ok(state) ==> SavedStateBlobView::of(state) == blob@,
{
    Ok(blob.parse()?)
}

} // verus!
