// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

//! Types for describing VM topology (processor packages and layout, memory
//! layout).

#![forbid(unsafe_code)]

// Verus requires every verified crate to import vstd.
#[cfg(verus_keep_ghost)]
use vstd::prelude::verus as _;

pub mod cxl;
pub mod layout;
pub mod memory;
pub mod pcie;
pub mod processor;
