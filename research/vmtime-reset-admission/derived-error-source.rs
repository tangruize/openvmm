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

// Unlike the empty impl control, this has the method emitted for RpcError.
#[automatically_derived]
impl core::error::Error for DerivedError {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        None
    }
}
