// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

//! Client-facing definitions for the VM worker.

#![expect(missing_docs)]
#![forbid(unsafe_code)]

pub mod config;
pub mod entrypoint;
pub mod microvm;
pub mod profile;
pub mod rpc;
#[cfg(verus_keep_ghost)]
pub mod snapshot_spec {
    include!("snapshot.spec.rs");
}
pub mod time_abi;
pub mod worker;
