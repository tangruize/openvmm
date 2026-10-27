// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

// The `StateUnit` spec of a line set's state unit, for human review.

use super::LineSetUnit;
use state_unit::StateUnitSpec;
use vstd::prelude::*;
use vstd::resource::Loc;

verus! {

/// TODO (per-unit spec): where the unit keeps its share of its FracGhost,
/// and its invariant (`wf`). Uninterpreted until then; the impl's method
/// bodies are `external_body` meanwhile (`SNAPSHOT-SPEC-TRUSTED.md`).
impl StateUnitSpec for LineSetUnit<'_> {
    uninterp spec fn wf(&self) -> bool;

    uninterp spec fn id(&self) -> Loc;

    uninterp spec fn running(&self) -> bool;
}

} // verus!
