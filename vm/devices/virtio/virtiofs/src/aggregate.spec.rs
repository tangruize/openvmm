// Spec of the aggregate's registry, for human review: its lock's rule.

use super::AggregateRegistry;
use super::AggregateState;
use super::ChildEntry;
use crate::spec::VirtioFsShareView;
use vstd::prelude::*;
use vstd::rwlock::RwLockPredicate;

verus! {

/// The rule of an aggregate's registry lock (`AggregateState::registry`):
/// with `fixed` (a microVM aggregate, whose children never change after
/// creation), its children are `shares`, by name.
///
/// SPEC: the core is part of the spec: it is what makes the shares that the
/// contracts name (`g`'s, through `owns`) the code's. Without `fixed` (an
/// aggregate whose host adds and removes children, `add_child`,
/// `remove_child`, outside any request) it says nothing, so the contracts
/// about such an aggregate's root cannot be proved (not covered).
/// TODO (proof agent): the rest; it may add facts or fields, but must keep
/// the core.
// Only Verus reads the rule's fields.
#[cfg_attr(not(verus_keep_ghost), expect(dead_code))]
pub ghost struct RegistryRule {
    /// The children never change.
    pub fixed: bool,
    /// The children, by name.
    pub shares: Map<Seq<u8>, VirtioFsShareView>,
}

impl RwLockPredicate<AggregateRegistry> for RegistryRule {
    closed spec fn inv(self, v: AggregateRegistry) -> bool {
        let es = v.entries@;
        self.fixed ==> {
            &&& forall|i: int| 0 <= i < es.len() ==> {
                let k = (#[trigger] es[i]).name_bytes();
                &&& self.shares.contains_key(k)
                &&& self.shares[k] == es[i].volume.share_view()
            }
            &&& forall|k: Seq<u8>| #[trigger] self.shares.contains_key(k)
                ==> exists|i: int| 0 <= i < es.len() && (#[trigger] es[i]).name_bytes() == k
        }
    }
}

impl ChildEntry {
    /// The child's name, as bytes (what `lookup_synthetic_root` compares).
    pub closed spec fn name_bytes(self) -> Seq<u8> {
        vstd::utf8::encode_utf8(self.name@)
    }
}

impl AggregateState {
    /// The children never change (the registry lock's rule).
    pub(crate) closed spec fn fixed(&self) -> bool {
        self.registry.pred().fixed
    }

    /// The children, by name, when they never change (the registry lock's
    /// rule).
    pub(crate) closed spec fn shares(&self) -> Map<Seq<u8>, VirtioFsShareView> {
        self.registry.pred().shares
    }
}

} // verus!
