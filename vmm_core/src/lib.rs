// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

//! Core building blocks for managing vm and vm related state to build a vmm.
//! Used by both OpenVMM and underhill today.

#![expect(missing_docs)]
#![forbid(unsafe_code)]

// Verus requires every verified crate to import vstd.
#[cfg(verus_keep_ghost)]
#[expect(unused_imports)]
use vstd::prelude::*;

pub mod acpi_builder;
pub mod cpuid;
pub mod device_builder;
pub mod emuplat;
pub mod input_distributor;
pub mod partition_unit;
pub mod platform_resolvers;
pub mod vmbus_unit;
pub mod vmotherboard_adapter;
pub mod vmtime_unit;
