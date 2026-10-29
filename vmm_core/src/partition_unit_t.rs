// TRUSTED, do not attempt to prove anything in this file: Verus uses only
// what is stated here, a claim about protobuf encoding, which is external
// (`SNAPSHOT-SPEC-TRUSTED.md`, Part 1).

use super::spec::PartitionSavedView;
use vmm_core_defs::HaltReason;
use vstd::prelude::*;

verus! {

impl PartitionSavedView {
    /// The encoding of a halt reason (uninterpreted: what
    /// `mesh::payload::encode` gives, protobuf).
    pub uninterp spec fn encode_halt(reason: HaltReason) -> Seq<u8>;
}

} // verus!
