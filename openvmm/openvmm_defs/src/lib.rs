// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

//! Client-facing definitions for the VM worker.

#![expect(missing_docs)]
#![forbid(unsafe_code)]

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
