// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

verus! {

#[verifier::external_body]
pub proof fn observe_str_pattern(pattern: &str)
    ensures
        pattern_observation(pattern) == PatternObservation::StringPrefix(pattern@),
{
}

#[verifier::external_body]
pub proof fn observe_dot_pattern()
    ensures
        pattern_observation('.') == PatternObservation::Dot,
{
}

} // verus!
