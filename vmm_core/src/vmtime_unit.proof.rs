// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

// Declarations supporting `vmtime_unit.spec.rs`; no human review needed.

use super::KeeperUnit;
use vstd::prelude::*;

verus! {

/// Declares `KeeperUnit` to Verus, as an opaque type.
#[verifier::external_type_specification]
#[verifier::external_body]
pub(super) struct ExKeeperUnit<'a>(KeeperUnit<'a>);

} // verus!
