// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

//! This crate implements the core VM worker for OpenVMM. This includes static
//! configuration of OpenVMM platform resources, such as the clock source and
//! memory management.
//!
//! Try not to add new functionality to this crate. Add it to other crates, and
//! reference new functionality via `Resource`s when you can to minimize build
//! time.

#![forbid(unsafe_code)]
#![cfg_attr(verus_keep_ghost, feature(proc_macro_hygiene, sized_hierarchy))]

// Verus requires every verified crate to import vstd.
#[cfg(verus_keep_ghost)]
#[expect(unused_imports)]
use vstd::prelude::*;

mod emuplat;
pub mod hypervisor_backend;
mod partition;
mod vmgs_non_volatile_store;
mod worker;

pub use worker::dispatch::VmWorker;
