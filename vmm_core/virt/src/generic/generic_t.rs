//! The partition's VM-state handles (`snapshot-spec-docs/DESIGN.md`, "The
//! partition's state").
//!
//! TRUSTED, do not attempt to prove anything in this file: Verus uses only
//! the contracts here (`snapshot-spec-docs/TRUSTED.md`, Part 1). They are thin
//! wrappers (`access_state`, a drop) that state how a partition's VM state,
//! which the hypervisor holds, relates to its handles and to the
//! partition's state token: all handles see one state, which the token
//! records. Exception, not trusted: that `new_vm_state_token` is called once
//! per partition is about our code (a PROOF TODO, at its caller).

use super::PartitionAccessState;
#[cfg(verus_keep_ghost)]
use crate::vm::VmSavedState;
#[cfg(verus_keep_ghost)]
use crate::vm::state_spec::StateAccessSpec;
use hvdef::Vtl;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
use vstd::resource::frac::FracGhost;

/// Opens a handle to `partition`'s VM state for `vtl` (`access_state`).
///
/// Verus: TRUSTED, do not attempt to prove (the hypervisor boundary; design
/// doc, "The partition's state"): the handle starts with the partition's state, which the
/// partition's token records; the token stays with the handle until
/// `close_vm_state`. Only VTL 0's VM state has a token.
#[cfg_attr(verus_keep_ghost, verus_verify(external_body))]
#[cfg_attr(verus_keep_ghost, verus_spec(result =>
    with Tracked(token): Tracked<&mut Option<FracGhost<VmSavedState>>>
    requires
        vtl == Vtl::Vtl0,
        (*old(token)) matches Some(t) && t.frac() == 1real,
    ensures
        (*final(token)) is None,
        result.hv_id() == (*old(token))->Some_0.id(),
        result.hv_state() == (*old(token))->Some_0@,
))]
pub fn open_vm_state<'a, P: PartitionAccessState>(
    partition: &'a P,
    vtl: Vtl,
) -> P::StateAccess<'a> {
    partition.access_state(vtl)
}

/// Closes a handle opened by `open_vm_state` (drops it).
///
/// Verus: TRUSTED, do not attempt to prove (as `open_vm_state`): the partition's state is what the
/// handle leaves; the token comes back, recording it.
#[cfg_attr(verus_keep_ghost, verus_verify(external_body))]
#[cfg_attr(verus_keep_ghost, verus_spec(
    with Tracked(token): Tracked<&mut Option<FracGhost<VmSavedState>>>
    requires
        (*old(token)) is None,
    ensures
        (*final(token)) matches Some(t) && t.frac() == 1real && t.id() == access.hv_id()
            && t@ == access.hv_state(),
))]
pub fn close_vm_state<A: crate::vm::AccessVmState>(access: A) {
    drop(access)
}

#[cfg(verus_keep_ghost)]
verus! {

/// Creates the token of a partition's VM state (VTL 0): the whole
/// FracGhost, whose value is the state. TRUSTED, do not attempt to prove
/// (the hypervisor): the
/// value is the partition's current state (unknown to the proof). PROOF
/// TODO, not trusted: that this is called once per partition, when its
/// `WrappedPartition` is built (about our own code: provable, e.g. with
/// `into_vm_partition` verified as the only caller).
#[verifier::external_body]
#[allow(unused_variables)]
pub fn new_vm_state_token<P: PartitionAccessState>(partition: &P) -> (token: Tracked<
    Option<FracGhost<VmSavedState>>,
>)
    ensures
        token@ matches Some(t) && t.frac() == 1real,
{
    // TRUSTED: keep this `assume_new()`. This trusted function's contract
    // hands out the token; its body has none to give. (Its "once per
    // partition" is a PROOF TODO; see above.)
    Tracked::assume_new()
}

} // verus!
