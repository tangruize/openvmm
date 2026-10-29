// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

// Declarations supporting `vp_set.spec.rs`; no human review needed.

use super::Halt;
use super::Inner;
use super::RegisterSetError;
use super::RegistersToSet;
use super::VpRunner;
use vm_topology::processor::TargetVpInfo;
use vstd::prelude::*;

verus! {

// `VpSet`, `Vp`, `VpEvent`, and `StateEvent` are `verus_verify` themselves;
// the types of their fields, declared to Verus, as opaque types.

#[verifier::external_type_specification]
#[verifier::external_body]
pub(super) struct ExInner(Inner);

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExHalt(Halt);

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExVpRunner(VpRunner);

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExRegistersToSet(RegistersToSet);

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExRegisterSetError(RegisterSetError);

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExTargetVpInfo(TargetVpInfo);

#[verifier::external_type_specification]
#[verifier::external_body]
#[verifier::reject_recursive_types(T)]
pub struct ExOneshotReceiver<T>(mesh::OneshotReceiver<T>);

#[verifier::external_type_specification]
#[verifier::external_body]
#[verifier::reject_recursive_types(T)]
pub struct ExOneshotSender<T>(mesh::OneshotSender<T>);

/// (Needed by `OneshotReceiver`'s `Future` impl.)
#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExRecvError(mesh::RecvError);

#[cfg(guest_arch = "x86_64")]
#[verifier::external_type_specification]
#[verifier::external_body]
pub(super) struct ExLapicAdvance(super::time_abi::LapicAdvance);

#[cfg(feature = "dump")]
#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExDumpVpState(hyperv_dump::VpState);

#[cfg(feature = "gdb")]
#[verifier::external_type_specification]
#[verifier::external_body]
pub(super) struct ExDebugEvent(super::DebugEvent);

} // verus!
