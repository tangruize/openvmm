// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

//! Virtual PCI bus emulator, providing a PCI bus over a vmbus transport.

#![forbid(unsafe_code)]

// Verus requires every verified crate to import vstd.
#[cfg(verus_keep_ghost)]
use vstd::prelude::verus as _;

pub mod bus;
pub mod bus_control;
mod device;
pub mod test_helpers;
