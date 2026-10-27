// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

// Declarations supporting `vmtime_unit.spec.rs`; no human review needed.

use super::KeeperUnit;
use vstd::prelude::*;

verus! {

/// Declares `KeeperUnit` (its fields) to Verus.
#[verifier::external_type_specification]
pub(super) struct ExKeeperUnit<'a>(KeeperUnit<'a>);

} // verus!
