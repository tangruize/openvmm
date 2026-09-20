// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

use vstd::prelude::*;

verus! {

fn string_slice(name: &String) -> (result: bool)
    ensures result == (name@ == "partition"@),
{
    *name == "partition"
}

fn slice_slice(name: &String) -> (result: bool)
    ensures result == (name@ == "partition"@),
{
    name.as_str() == "partition"
}

}

fn main() {}
