// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

use vstd::prelude::*;
use core::str::pattern::{Pattern, Searcher, ReverseSearcher};

verus! {

#[verifier::external_trait_specification]
pub trait ExSearcher<'a> {
    type ExternalTraitSpecificationFor: Searcher<'a>;
}

#[verifier::external_trait_specification]
pub trait ExReverseSearcher<'a>: Searcher<'a> {
    type ExternalTraitSpecificationFor: ReverseSearcher<'a>;
}

#[verifier::external_trait_specification]
pub trait ExPattern: Sized {
    type ExternalTraitSpecificationFor: Pattern;
}

pub enum PatternObservation {
    StringPrefix(Seq<char>),
    Dot,
    Unsupported,
}

// Only the two concrete bindings in observation.proof.rs are exposed.
pub uninterp spec fn pattern_observation<P: Pattern>(pattern: P) -> PatternObservation;

pub open spec fn prefix_result(s: Seq<char>, prefix: Seq<char>, result: Option<&str>) -> bool {
    match result {
        Some(rest) => prefix.is_prefix_of(s) && rest@ == s.skip(prefix.len() as int),
        None => !prefix.is_prefix_of(s),
    }
}

pub open spec fn last_dot_result(s: Seq<char>, result: Option<(&str, &str)>) -> bool {
    match result {
        Some((left, right)) => s == left@ + seq!['.'] + right@ && !right@.contains('.'),
        None => !s.contains('.'),
    }
}

#[verifier::external_fn_specification]
pub fn str_strip_prefix<P: Pattern>(s: &str, prefix: P) -> (result: Option<&str>)
    requires pattern_observation(prefix) is StringPrefix,
    ensures prefix_result(s@, pattern_observation(prefix)->StringPrefix_0, result),
{
    s.strip_prefix(prefix)
}

#[verifier::external_fn_specification]
pub fn str_rsplit_once_dot<P: Pattern>(s: &str, delimiter: P) -> (result: Option<(&str, &str)>)
    where for<'a> P::Searcher<'a>: ReverseSearcher<'a>,
    requires pattern_observation(delimiter) is Dot,
    ensures last_dot_result(s@, result),
{
    s.rsplit_once(delimiter)
}

} // verus!
