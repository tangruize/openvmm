// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

//! Client-facing definitions for the VM worker.

#![expect(missing_docs)]
#![cfg_attr(not(verus_keep_ghost), forbid(unsafe_code))]
// Under Verus, `assume_specification` (in `rpc_t.rs`) expands to `unsafe`
// declarations; everything else still may not use `unsafe`.
#![cfg_attr(verus_keep_ghost, deny(unsafe_code))]

// Verus requires every verified crate to import vstd.
#[cfg(verus_keep_ghost)]
#[expect(unused_imports)]
use vstd::prelude::*;

pub mod config;
pub mod entrypoint;
pub mod microvm;
pub mod profile;
pub mod rpc;
pub mod time_abi;
pub mod worker;
