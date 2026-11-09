// High-level spec of a microVM share's access policy, for human review.
//
// Written from the policy's intent, the module documentation of
// `policy.rs`: a denied path hides itself and everything below it; an
// allowed path shows itself and everything below it again, inside a denied
// path; the nearest denied or allowed path that contains a path decides
// whether the guest can see it; a hidden directory that leads to an allowed
// path is traverse-only; and when a read-write share has writable paths,
// they are the only parts of it that the guest can modify.

use super::PathVisibility;
use super::Rule;
use super::SubtreePolicy;
use std::path::PathBuf;
use vstd::prelude::*;
use vstd::std_specs::path::PathView;

verus! {

/// The view of a `SubtreePolicy`: its three sets of share-relative paths
/// (each a `PathView`: a path's components; the empty path is the share's
/// root).
pub struct SubtreePolicyView {
    /// The denied paths: each hides itself and everything below it.
    pub denied: Set<PathView>,
    /// The allowed paths: each shows itself and everything below it again,
    /// inside a denied path.
    pub allowed: Set<PathView>,
    /// The writable paths: if there are any, a read-write share's only
    /// parts that the guest can modify, each with everything below it.
    pub writable: Set<PathView>,
}

impl SubtreePolicyView {
    /// The kind of the nearest rule containing `p`: walk up from `p` toward
    /// the share's root; the first path on the way (`p` itself included)
    /// that is in the denied or the allowed list decides. `None` if none is.
    closed spec fn nearest_rule(self, p: PathView) -> Option<Rule> {
        self.first_rule_upward(p, p.len() as int)
    }

    /// Looks at the path made of `p`'s first `n` names, then at `n - 1`
    /// names, and so on up to the root (0 names): the first one that is in a
    /// list decides.
    closed spec fn first_rule_upward(self, p: PathView, n: int) -> Option<Rule>
        decreases n + 1,
    {
        if n < 0 || n > p.len() {
            None
        } else if self.denied.contains(p.subrange(0, n)) {
            Some(Rule::Deny)
        } else if self.allowed.contains(p.subrange(0, n)) {
            Some(Rule::Allow)
        } else {
            self.first_rule_upward(p, n - 1)
        }
    }

    /// Some allowed path lies below `p`: `p` is on the way to it.
    closed spec fn leads_to_allowed(self, p: PathView) -> bool {
        exists|a: PathView| #[trigger] self.allowed.contains(a) && p.is_prefix_of(a) && a != p
    }

    /// How the guest can reach `p`: visible if no rule contains it or the
    /// nearest one is an allowed path; if the nearest one is a denied path,
    /// traverse-only when it leads to an allowed path, else hidden.
    pub closed spec fn visibility(self, p: PathView) -> PathVisibility {
        match self.nearest_rule(p) {
            Some(Rule::Deny) => if self.leads_to_allowed(p) {
                PathVisibility::TraverseOnly
            } else {
                PathVisibility::Hidden
            },
            _ => PathVisibility::Visible,
        }
    }

    /// `p` is a writable path or lies below one.
    closed spec fn within_writable(self, p: PathView) -> bool {
        exists|w: PathView| #[trigger] self.writable.contains(w) && w.is_prefix_of(p)
    }

    /// The guest may modify `p`, in a read-write share: `p` is visible (a
    /// denied path inside a writable one hides part of it, and the guest
    /// must not change what it cannot see), and it is within a writable path,
    /// unless there are none. No writable paths at all (`writable` empty)
    /// means that a read-write share is writable everywhere the guest can
    /// see (`SubtreePolicy::writable_paths`).
    pub closed spec fn can_write(self, p: PathView) -> bool {
        &&& self.visibility(p) == PathVisibility::Visible
        &&& (self.writable.is_empty() || self.within_writable(p))
    }

    /// A denied path `d` is not redundant.
    closed spec fn denied_ok(self, d: PathView) -> bool {
        if d.len() == 0 {
            // Denying the root makes sense only with allowed paths, which
            // show parts of the share again.
            !self.allowed.is_empty()
        } else {
            // The nearest rule above `d` is not a denied path.
            self.nearest_rule(d.drop_last()) != Some(Rule::Deny)
        }
    }

    /// An allowed path `a` is not redundant: it is not also denied, not the
    /// root, and the nearest rule above it is a denied path.
    closed spec fn allowed_ok(self, a: PathView) -> bool {
        &&& !self.denied.contains(a)
        &&& a.len() > 0
        &&& self.nearest_rule(a.drop_last()) == Some(Rule::Deny)
    }

    /// A writable path `w` is not redundant: it is not the root, it is
    /// visible, and its parent is not within a writable path (so it lies
    /// under no other writable path).
    closed spec fn writable_ok(self, w: PathView) -> bool {
        &&& w.len() > 0
        &&& self.visibility(w) == PathVisibility::Visible
        &&& !self.within_writable(w.drop_last())
    }

    /// The lists fit a share's access mode: a read-only share has no
    /// writable paths.
    pub(crate) closed spec fn fits_access_mode(self, read_only: bool) -> bool {
        read_only ==> self.writable.is_empty()
    }

    /// The rules on the three lists, which `SubtreePolicy::new` checks, so
    /// that no path in the policy is redundant: one per list.
    pub(crate) closed spec fn inv(self) -> bool {
        &&& forall|d: PathView| #[trigger] self.denied.contains(d) ==> self.denied_ok(d)
        &&& forall|a: PathView| #[trigger] self.allowed.contains(a) ==> self.allowed_ok(a)
        &&& forall|w: PathView| #[trigger] self.writable.contains(w) ==> self.writable_ok(w)
    }
}

impl View for SubtreePolicy {
    type V = SubtreePolicyView;

    /// The three lists, as sets of paths.
    closed spec fn view(&self) -> SubtreePolicyView {
        SubtreePolicyView {
            denied: self.denied@.map_values(|p: PathBuf| p@).to_set(),
            allowed: self.allowed@.map_values(|p: PathBuf| p@).to_set(),
            writable: self.writable@.map_values(|p: PathBuf| p@).to_set(),
        }
    }
}

impl SubtreePolicy {
    /// Well formed: the view's rules hold.
    pub closed spec fn wf(&self) -> bool {
        self@.inv()
    }
}

} // verus!
