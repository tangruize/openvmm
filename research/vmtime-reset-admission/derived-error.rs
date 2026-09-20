// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

use vstd::prelude::*;

#[derive(Debug)]
#[verus_verify(external_body)]
struct DerivedError;

impl core::fmt::Display for DerivedError {
    fn fmt(&self, _formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        Ok(())
    }
}

// Isolate the admission rule used by thiserror, without any missing project type.
#[automatically_derived]
impl core::error::Error for DerivedError {}
