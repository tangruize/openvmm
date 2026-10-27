// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

// Declarations supporting `partition_unit.spec.rs`; no human review needed.

use super::PartitionUnitRunner;
use vstd::prelude::*;

verus! {

/// Declares `PartitionUnitRunner` to Verus, as an opaque type.
#[verifier::external_type_specification]
#[verifier::external_body]
pub(super) struct ExPartitionUnitRunner(PartitionUnitRunner);

} // verus!
