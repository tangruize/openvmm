// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

verus! {

pub(crate) open spec fn without_https(input: Seq<char>) -> Seq<char> {
    if "https://"@.is_prefix_of(input) {
        input.skip("https://"@.len() as int)
    } else {
        input
    }
}

impl vstd::std_specs::cmp::PartialEqSpecImpl<str> for TypeUrl<'_> {
    open spec fn obeys_eq_spec() -> bool { true }

    open spec fn eq_spec(&self, other: &str) -> bool {
        self.accepts(other@)
    }
}

impl vstd::std_specs::cmp::PartialEqSpecImpl<TypeUrl<'_>> for str {
    open spec fn obeys_eq_spec() -> bool { true }

    open spec fn eq_spec(&self, other: &TypeUrl<'_>) -> bool {
        other.accepts(self@)
    }
}

} // verus!
