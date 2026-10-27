// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

//! VM state machine unit handling.
//!
//! A state unit is a VM component (such as a device) that needs to react to
//! changes in the VM state machine. It needs to start, stop, reset, and
//! save/restore with the VM. (Save/restore is not really a state change but is
//! modeled as such since it must be synchronized with actual state changes.)
//!
//! This module contains types and functions for defining and manipulating state
//! units. It does this in three parts:
//!
//! 1. It defines an RPC enum [`StateRequest`] which is used to request that a
//!    state unit change state (start, stop, etc.). Each state unit must handle
//!    incoming state requests on a mesh receiver. This is the foundational
//!    type of this model.
//!
//! 2. It defines a type [`StateUnits`], which is a collection of mesh senders
//!    for sending `StateRequest`s. This is used to initiate and wait for state
//!    changes across all the units in the VMM, handling any required dependency
//!    ordering.
//!
//! 3. It defines a trait [`StateUnit`] that can be used to define handlers for
//!    each state request. This is an optional convenience; not all state units
//!    will have a type that implements this trait.
//!
//! This model allows for asynchronous, highly concurrent state changes, and it
//! works across process boundaries thanks to `mesh`.

#![cfg_attr(not(verus_keep_ghost), forbid(unsafe_code))]
// Under Verus, `assume_specification` (in `registry_t`) expands to `unsafe`
// declarations; everything else still may not use `unsafe`.
#![cfg_attr(verus_keep_ghost, deny(unsafe_code))]

mod advance_time;
mod inventory;
#[allow(dead_code)] // `acquire` and `release` are for verified code, still to come
mod mutex_t;
pub mod quiesce;
mod registry;
#[cfg(verus_keep_ghost)]
#[allow(unsafe_code)]
mod registry_t;
mod start;
mod unit_channel;

pub use unit_channel::unit_channel;
pub use unit_channel::unit_channel_for;

/// Verus specification of state units.
#[cfg(verus_keep_ghost)]
#[allow(missing_docs)] // methods that Verus generates for spec enums
pub mod spec {
    include!("lib.spec.rs");

    /// Lemmas and declarations supporting the spec. Inside `spec`, so that
    /// lemmas can unfold its closed views.
    pub(crate) mod proof {
        include!("lib.proof.rs");
    }
}

use mesh_rpc_verus::MeshProtocol;
#[cfg(verus_keep_ghost)]
use mesh_rpc_verus::ReceiverAdditionalSpecFns;
#[cfg(verus_keep_ghost)]
use mesh_rpc_verus::RpcAdditionalSpecFns;
#[cfg(verus_keep_ghost)]
pub use spec::StateUnitSpec;
#[cfg(verus_keep_ghost)]
use spec::UnitSnapshot;
#[cfg(verus_keep_ghost)]
use vstd::resource::frac::FracGhost;

/// What `StateUnit`'s contract needs to know about a unit (see
/// `spec::StateUnitSpec`). Empty in normal builds.
#[cfg(not(verus_keep_ghost))]
pub trait StateUnitSpec {}

#[cfg(not(verus_keep_ghost))]
impl<T: ?Sized> StateUnitSpec for T {}
#[cfg(verus_keep_ghost)]
use vmcore::save_restore::spec::SavedStateBlobView;
// In both builds: `apply` and `run_unit` use its macros (`verus!`,
// `proof!`, `proof_with!`), which erase their ghost code in normal builds.
use vstd::prelude::*;

use futures::FutureExt;
use futures::StreamExt;
use futures::future::join_all;
use futures_concurrency::stream::Merge;
use inspect::Inspect;
use inspect::InspectMut;
use mesh::MeshPayload;
use mesh::Receiver;
use mesh::Sender;
use mesh::error::RemoteError;
use mesh::payload::Protobuf;
use mesh::rpc::FailableRpc;
use mesh::rpc::Rpc;
use mesh::rpc::RpcError;
use pal_async::task::Spawn;
use pal_async::task::Task;
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::collections::hash_map;
use std::fmt::Debug;
use std::fmt::Display;
use std::future::Future;
use std::pin::pin;
use std::sync::Arc;
use std::sync::Weak;
use std::sync::atomic::AtomicU32;
use std::sync::atomic::Ordering;
use std::time::Instant;
use thiserror::Error;
use tracing::Instrument;
use vmcore::save_restore::RestoreError;
use vmcore::save_restore::SaveError;
use vmcore::save_restore::SavedStateBlob;

/// A state change request.
#[derive(Debug, MeshPayload)]
pub enum StateRequest {
    /// Start asynchronous operations.
    Start(FailableRpc<(), ()>),

    /// Stop asynchronous operations.
    Stop(Rpc<(), ()>),

    /// Stop accepting new host input before a snapshot vCPU boundary.
    QuiesceInput(FailableRpc<(), ()>),

    /// Resume host input after a failed snapshot transaction.
    ResumeInput(FailableRpc<(), ()>),

    /// Reset a stopped unit to initial state.
    Reset(FailableRpc<(), ()>),

    /// Save state of a stopped unit.
    Save(FailableRpc<(), Option<SavedStateBlob>>),

    /// Restore state of a stopped unit.
    Restore(FailableRpc<SavedStateBlob, ()>),

    /// Advance guest-visible time while stopped after snapshot restore.
    AdvanceTime(FailableRpc<std::time::Duration, ()>),

    /// Inspect state.
    Inspect(inspect::Deferred),
}

/// Trait implemented by an object that can act as a state unit.
///
/// Implementing this is optional, to be used with [`UnitBuilder::spawn`] or
/// [`StateRequest::apply`]; state units can also directly process incoming
/// [`StateRequest`]s.
///
/// Verus: each method's contract is the request protocol (see
/// `MeshProtocol::request_ok` for `StateRequest`, in `lib.proof.rs`). `g`
/// carries the FracGhost half the request sends, and on return the half the
/// reply sends back. Every method keeps the unit's `wf()` and `id()`.
#[expect(async_fn_in_trait)] // Don't need Send bounds
#[cfg_attr(verus_keep_ghost, verus_verify)]
pub trait StateUnit: InspectMut + StateUnitSpec {
    /// Start asynchronous processing.
    ///
    /// Verus: takes the caller's half; the unit, now holding the whole,
    /// runs. Nothing comes back, also on `Err`.
    #[cfg_attr(verus_keep_ghost, verus_spec(result =>
        with Tracked(g): Tracked<&mut Option<FracGhost<UnitSnapshot>>>
        requires
            old(self).wf(),
            *old(g) matches Some(h) && h.valid(old(self).id(), 0.5real),
        ensures
            final(self).wf(),
            final(self).id() == old(self).id(),
            final(self).running(),
            *final(g) is None,
    ))]
    async fn start(&mut self) -> anyhow::Result<()>;

    /// Stop asynchronous processing.
    ///
    /// Verus: the running unit records its state and gives back a half
    /// recording it. (`running()` is the trusted fact that `StateUnits`
    /// stops only running units.)
    #[cfg_attr(verus_keep_ghost, verus_spec(
        with Tracked(g): Tracked<&mut Option<FracGhost<UnitSnapshot>>>
        requires
            old(self).wf(),
            old(self).running(),
            *old(g) is None,
        ensures
            final(self).wf(),
            final(self).id() == old(self).id(),
            !final(self).running(),
            *final(g) matches Some(h) && h.valid(final(self).id(), 0.5real)
                && !(h@ is Unknown),
    ))]
    async fn stop(&mut self);

    /// Stops accepting new host input while preserving runtime state.
    ///
    /// Verus: no ghost.
    #[cfg_attr(verus_keep_ghost, verus_spec(result =>
        requires
            old(self).wf(),
        ensures
            final(self).wf(),
            final(self).id() == old(self).id(),
            final(self).running() == old(self).running(),
    ))]
    async fn quiesce_input(&mut self) -> anyhow::Result<()> {
        Ok(())
    }

    /// Resumes host input after a failed snapshot transaction.
    ///
    /// Verus: no ghost.
    #[cfg_attr(verus_keep_ghost, verus_spec(result =>
        requires
            old(self).wf(),
        ensures
            final(self).wf(),
            final(self).id() == old(self).id(),
            final(self).running() == old(self).running(),
    ))]
    async fn resume_input(&mut self) -> anyhow::Result<()> {
        Ok(())
    }

    /// Reset to initial state.
    ///
    /// Must only be called while stopped.
    ///
    /// Verus: takes the caller's half and gives back a half (the unit's
    /// state may have changed), also on `Err`.
    #[cfg_attr(verus_keep_ghost, verus_spec(result =>
        with Tracked(g): Tracked<&mut Option<FracGhost<UnitSnapshot>>>
        requires
            old(self).wf(),
            *old(g) matches Some(h) && h.valid(old(self).id(), 0.5real),
        ensures
            final(self).wf(),
            final(self).id() == old(self).id(),
            final(self).running() == old(self).running(),
            *final(g) matches Some(h) && h.valid(final(self).id(), 0.5real),
    ))]
    async fn reset(&mut self) -> anyhow::Result<()>;

    /// Save state.
    ///
    /// Must only be called while stopped.
    ///
    /// Verus: the caller lends its half, recording a known state, and gets
    /// it back unchanged; `Ok(Some(blob))` iff the half records
    /// `Saved(blob@)`, `Ok(None)` iff it records `NoSavedState`.
    #[cfg_attr(verus_keep_ghost, verus_spec(result =>
        with Tracked(g): Tracked<&mut Option<FracGhost<UnitSnapshot>>>
        requires
            old(self).wf(),
            *old(g) matches Some(h) && h.valid(old(self).id(), 0.5real) && !(h@ is Unknown),
        ensures
            final(self).wf(),
            final(self).id() == old(self).id(),
            final(self).running() == old(self).running(),
            *final(g) == *old(g),
            result matches Ok(Some(blob)) ==> old(g)->Some_0@ == UnitSnapshot::Saved(blob@),
            result matches Ok(None) ==> old(g)->Some_0@ == UnitSnapshot::NoSavedState,
    ))]
    async fn save(&mut self) -> Result<Option<SavedStateBlob>, SaveError>;

    /// Restore state.
    ///
    /// Must only be called while stopped.
    ///
    /// Verus: takes the caller's half and gives back a half, also on `Err`;
    /// on `Ok`, it records `Saved(buffer@)`.
    #[cfg_attr(verus_keep_ghost, verus_spec(result =>
        with Tracked(g): Tracked<&mut Option<FracGhost<UnitSnapshot>>>
        requires
            old(self).wf(),
            *old(g) matches Some(h) && h.valid(old(self).id(), 0.5real),
        ensures
            final(self).wf(),
            final(self).id() == old(self).id(),
            final(self).running() == old(self).running(),
            *final(g) matches Some(h) && h.valid(final(self).id(), 0.5real)
                && (result is Ok ==> h@ == UnitSnapshot::Saved(buffer@)),
    ))]
    async fn restore(&mut self, buffer: SavedStateBlob) -> Result<(), RestoreError>;

    /// Advance guest-visible time after restore. Most units derive their
    /// deadlines from VM time and need no direct adjustment; only units added
    /// with [`UnitBuilder::advances_time`] receive this request.
    ///
    /// Verus: takes the caller's half and gives back a half, also on `Err`.
    #[cfg_attr(verus_keep_ghost, verus_spec(result =>
        with Tracked(g): Tracked<&mut Option<FracGhost<UnitSnapshot>>>
        requires
            old(self).wf(),
            *old(g) matches Some(h) && h.valid(old(self).id(), 0.5real),
        ensures
            final(self).wf(),
            final(self).id() == old(self).id(),
            final(self).running() == old(self).running(),
            *final(g) matches Some(h) && h.valid(final(self).id(), 0.5real),
    ))]
    async fn advance_time(&mut self, _duration: std::time::Duration) -> anyhow::Result<()> {
        Ok(())
    }
}

verus! {

/// Runs a simple unit that only needs to respond to state requests.
///
/// Verus: `recv` is the unit's own channel (its constant is the id of the
/// unit's FracGhost). The loop runs until every sender is gone, so it has
/// no termination measure.
#[verifier::exec_allows_no_decreases_clause]
pub async fn run_unit<T: StateUnit>(unit: T, recv: Receiver<StateRequest>) -> (result: T)
    requires
        unit.wf(),
        recv.constant() == unit.id(),
    ensures
        result.wf(),
        result.id() == unit.id(),
{
    let mut unit = unit;
    let mut recv = recv;
    let ghost id = unit.id();
    loop
        invariant
            unit.wf(),
            unit.id() == id,
            recv.constant() == id,
    {
        if !run_unit_step(&mut unit, &mut recv).await {
            break;
        }
    }
    unit
}

} // verus!

/// Answers the next request on `recv`; returns false when every sender is
/// gone. Verus: the request and the half it carries go to `apply`.
#[cfg_attr(verus_keep_ghost, verus_verify)]
#[cfg_attr(verus_keep_ghost, verus_spec(more =>
    requires
        old(unit).wf(),
        old(recv).constant() == old(unit).id(),
    ensures
        final(unit).wf(),
        final(unit).id() == old(unit).id(),
        final(recv).constant() == old(recv).constant(),
))]
async fn run_unit_step<T: StateUnit>(unit: &mut T, recv: &mut Receiver<StateRequest>) -> bool {
    let Some((req, gin)) = mesh_rpc_verus::recv_msg(recv).await else {
        return false;
    };
    handle_request(unit, req, gin).await;
    true
}

/// The ghost a request carries (`MeshProtocol::GhostIn`).
pub type RequestGhost = <StateRequest as MeshProtocol>::GhostIn;

/// Answers `req`, which came with the half `gin`, on `unit`'s own channel
/// (`StateRequest::apply`). For a unit's own request loop: unlike `apply`,
/// it takes the half as an argument, so `verus!` code can call it.
#[cfg_attr(verus_keep_ghost, verus_verify)]
#[cfg_attr(verus_keep_ghost, verus_spec(
    requires
        old(unit).wf(),
        StateRequest::request_ok(old(unit).id(), req, gin@),
    ensures
        final(unit).wf(),
        final(unit).id() == old(unit).id(),
))]
pub async fn handle_request<T: StateUnit>(
    unit: &mut T,
    req: StateRequest,
    gin: Tracked<RequestGhost>,
) {
    // (`gin` is used only by Verus builds.)
    let _ = &gin;
    let fut = {
        proof_with! {gin}
        req.apply(unit)
    };
    fut.await;
}

/// A state unit that serves inspect requests while one of its state changes
/// is in flight ([`run_async_unit`]), through an inspector separate from the
/// unit, so that the state change can hold the unit exclusively.
pub trait InspectConcurrently {
    /// Inspects the unit's state without borrowing the unit (for example, a
    /// clone of the sender to the task that owns the state).
    type Inspector: Inspect;

    /// The unit's inspector.
    fn inspector(&self) -> Self::Inspector;
}

/// Runs a state unit that can handle inspect requests while there is an active
/// state transition.
///
/// Verus: as [`run_unit`]; `recv` is the unit's own channel.
#[cfg_attr(verus_keep_ghost, verus_verify(external_body))]
#[cfg_attr(verus_keep_ghost, verus_spec(result =>
    requires
        unit.wf(),
        recv.constant() == unit.id(),
    ensures
        result.wf(),
        result.id() == unit.id(),
))]
pub async fn run_async_unit<T>(mut unit: T, mut recv: Receiver<StateRequest>) -> T
where
    T: StateUnit + InspectConcurrently,
{
    let inspector = unit.inspector();
    while let Some((req, _ghost)) = mesh_rpc_verus::recv_msg(&mut recv).await {
        req.apply_with_concurrent_inspects(&mut unit, &inspector, &mut recv)
            .await;
    }
    unit
}

impl StateRequest {
    /// Runs this state request against `unit`, polling `recv` for incoming
    /// inspect requests and applying them, through `inspector`, while any
    /// state transition is in flight.
    ///
    /// Panics if a state transition arrives on `recv` while this one is being
    /// processed. This would indicate a contract violation with [`StateUnits`].
    ///
    /// Verus: as [`StateRequest::apply`]; inspects keep the unit's `wf()`
    /// (trusted, as for `apply`'s `Inspect`).
    #[cfg_attr(verus_keep_ghost, verus_verify(external_body))]
    #[cfg_attr(verus_keep_ghost, verus_spec(
        with Tracked(gin): Tracked<Option<FracGhost<UnitSnapshot>>>
        requires
            old(unit).wf(),
            StateRequest::request_ok(old(unit).id(), self, gin),
        ensures
            final(unit).wf(),
            final(unit).id() == old(unit).id(),
    ))]
    pub async fn apply_with_concurrent_inspects<T: StateUnit>(
        self,
        unit: &mut T,
        inspector: &impl Inspect,
        recv: &mut Receiver<StateRequest>,
    ) {
        match self {
            StateRequest::Inspect(_) => {
                // This request has no response and completes synchronously,
                // so don't wait for concurrent requests.
                self.apply(unit).await;
            }

            StateRequest::Start(_)
            | StateRequest::Stop(_)
            | StateRequest::QuiesceInput(_)
            | StateRequest::ResumeInput(_)
            | StateRequest::AdvanceTime(_)
            | StateRequest::Reset(_)
            | StateRequest::Save(_)
            | StateRequest::Restore(_) => {
                // Handle for concurrent inspect requests.
                enum Event {
                    OpDone,
                    Req(StateRequest),
                }
                let op = pin!(self.apply(unit).into_stream());
                let mut stream = (op.map(|()| Event::OpDone), recv.map(Event::Req)).merge();
                while let Some(Event::Req(next_req)) = stream.next().await {
                    match next_req {
                        StateRequest::Inspect(req) => req.inspect(inspector),
                        _ => panic!(
                            "unexpected state transition {next_req:?} during state transition"
                        ),
                    }
                }
            }
        }
    }

    /// Runs this state request against `unit`.
    ///
    /// Replies through the mesh RPC wrapper (`mesh_rpc_verus::split`, and
    /// `reply`). Verus: `gin` is the half the request carries; each request
    /// is answered by the request protocol, from `StateUnit`'s contract.
    /// (Equivalent to `Rpc::handle` / `handle_failable`.)
    #[cfg_attr(verus_keep_ghost, verus_verify)]
    #[cfg_attr(verus_keep_ghost, verus_spec(
        with Tracked(gin): Tracked<Option<FracGhost<UnitSnapshot>>>
        requires
            old(unit).wf(),
            StateRequest::request_ok(old(unit).id(), self, gin),
        ensures
            final(unit).wf(),
            final(unit).id() == old(unit).id(),
    ))]
    pub async fn apply(self, unit: &mut impl StateUnit) {
        use mesh_rpc_verus::split;
        proof_decl! {
            let tracked mut g = gin;
        }
        match self {
            StateRequest::Start(rpc) => {
                let ((), rpc) = split::<_, _, ReplyGhost>(rpc);
                let fut = {
                    proof_with! {Tracked(&mut g)}
                    unit.start()
                };
                let r = match fut.await {
                    Ok(()) => Ok(()),
                    Err(err) => Err(anyhow_error(err)),
                };
                proof_with! {Tracked(g)}
                reply(rpc, r);
            }
            StateRequest::Stop(rpc) => {
                let ((), rpc) = split::<_, _, ReplyGhost>(rpc);
                proof! {
                    // TRUSTED: a unit receives `Stop` only while it runs
                    // ("`StateUnits`' unit states are the units' real
                    // states").
                    assume(unit.running());
                }
                let fut = {
                    proof_with! {Tracked(&mut g)}
                    unit.stop()
                };
                fut.await;
                proof_with! {Tracked(g)}
                reply(rpc, ());
            }
            StateRequest::QuiesceInput(rpc) => {
                let ((), rpc) = split::<_, _, ReplyGhost>(rpc);
                let r = match unit.quiesce_input().await {
                    Ok(()) => Ok(()),
                    Err(err) => Err(anyhow_error(err)),
                };
                proof_with! {Tracked(g)}
                reply(rpc, r);
            }
            StateRequest::ResumeInput(rpc) => {
                let ((), rpc) = split::<_, _, ReplyGhost>(rpc);
                let r = match unit.resume_input().await {
                    Ok(()) => Ok(()),
                    Err(err) => Err(anyhow_error(err)),
                };
                proof_with! {Tracked(g)}
                reply(rpc, r);
            }
            StateRequest::Reset(rpc) => {
                let ((), rpc) = split::<_, _, ReplyGhost>(rpc);
                let fut = {
                    proof_with! {Tracked(&mut g)}
                    unit.reset()
                };
                let r = match fut.await {
                    Ok(()) => Ok(()),
                    Err(err) => Err(anyhow_error(err)),
                };
                proof_with! {Tracked(g)}
                reply(rpc, r);
            }
            StateRequest::Save(rpc) => {
                let ((), rpc) = split::<_, _, ReplyGhost>(rpc);
                let fut = {
                    proof_with! {Tracked(&mut g)}
                    unit.save()
                };
                let r = match fut.await {
                    Ok(blob) => Ok(blob),
                    Err(err) => Err(save_error(err)),
                };
                proof_with! {Tracked(g)}
                reply(rpc, r);
            }
            StateRequest::Restore(rpc) => {
                let (buffer, rpc) = split::<_, _, ReplyGhost>(rpc);
                let fut = {
                    proof_with! {Tracked(&mut g)}
                    unit.restore(buffer)
                };
                let r = match fut.await {
                    Ok(()) => Ok(()),
                    Err(err) => Err(restore_error(err)),
                };
                proof_with! {Tracked(g)}
                reply(rpc, r);
            }
            StateRequest::AdvanceTime(rpc) => {
                let (duration, rpc) = split::<_, _, ReplyGhost>(rpc);
                let fut = {
                    proof_with! {Tracked(&mut g)}
                    unit.advance_time(duration)
                };
                let r = match fut.await {
                    Ok(()) => Ok(()),
                    Err(err) => Err(anyhow_error(err)),
                };
                proof_with! {Tracked(g)}
                reply(rpc, r);
            }
            StateRequest::Inspect(req) => inspect_unit(req, unit),
        }
    }
}

/// The ghost a unit's reply carries (`MeshProtocol::GhostOut`).
type ReplyGhost = <StateRequest as MeshProtocol>::GhostOut;

/// Answers `rpc` with `r` and the reply ghost `gout`.
#[cfg_attr(verus_keep_ghost, verus_verify)]
#[cfg_attr(verus_keep_ghost, verus_spec(
    with Tracked(gout): Tracked<ReplyGhost>
    requires
        rpc.reply_ok(r, gout),
))]
fn reply<R: 'static + Send>(rpc: Rpc<(), R>, r: R) {
    // The same call in every build; only the verification pass
    // (`verus_keep_ghost_body`) has the ghost. (Its argument is what `verus!`
    // makes of `Tracked(gout)`.)
    #[cfg(verus_keep_ghost_body)]
    mesh_rpc_verus::complete(
        rpc,
        r,
        #[verifier::ghost_wrapper]
        tracked_exec(
            #[verifier::tracked_block_wrapped]
            gout,
        ),
    );
    #[cfg(not(verus_keep_ghost_body))]
    mesh_rpc_verus::complete(rpc, r, Tracked::<ReplyGhost>::assume_new());
}

/// Inspects `unit`. Trusted: an inspect keeps the unit's `wf()`, `id()`, and
/// `running()` (`InspectMut::inspect_mut` has no contract yet; see
/// `SNAPSHOT-SPEC-TRUSTED.md`).
#[cfg_attr(verus_keep_ghost, verus_verify(external_body))]
#[cfg_attr(verus_keep_ghost, verus_spec(
    ensures
        final(unit).wf() == old(unit).wf(),
        final(unit).id() == old(unit).id(),
        final(unit).running() == old(unit).running(),
))]
fn inspect_unit(req: inspect::Deferred, unit: &mut impl StateUnit) {
    req.inspect(unit)
}

/// An error as a `RemoteError` (no contract).
#[cfg_attr(verus_keep_ghost, verus_verify(external_body))]
fn anyhow_error(err: anyhow::Error) -> RemoteError {
    RemoteError::new(err)
}

/// A save error as a `RemoteError` (no contract).
#[cfg_attr(verus_keep_ghost, verus_verify(external_body))]
fn save_error(err: SaveError) -> RemoteError {
    RemoteError::new(err)
}

/// A restore error as a `RemoteError` (no contract).
#[cfg_attr(verus_keep_ghost, verus_verify(external_body))]
fn restore_error(err: RestoreError) -> RemoteError {
    RemoteError::new(err)
}

/// A set of state units.
#[derive(Debug)]
#[cfg_attr(verus_keep_ghost, verus_verify)]
pub struct StateUnits {
    inner: Arc<InnerMutex>,
    running: bool,
    /// Verus only (zero-sized): the units collected so far and the FracGhost
    /// half held for each.
    #[cfg(verus_keep_ghost)]
    units_ghost: Tracked<spec::proof::UnitHalves>,
}

/// The mutex holding a [`StateUnits`]'s registry, with its invariant
/// [`InnerPred`] (Verus only; see `lib.proof.rs`).
type InnerMutex = mutex_t::Mutex<Inner, InnerPred>;

/// Names the invariant of [`InnerMutex`].
enum InnerPred {}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Inspect)]
enum State {
    Stopped,
    Starting,
    Running,
    Stopping,
    Resetting,
    Saving,
    Restoring,
    AdvancingTime,
    QuiesceUncertain,
}

#[derive(Debug)]
struct Inner {
    next_id: u64,
    units: BTreeMap<u64, Unit>,
    names: HashMap<Arc<str>, u64>,
    /// Verus only (zero-sized): the registry's ghost state (`RegistryGhost`,
    /// `lib.proof.rs`).
    #[cfg(verus_keep_ghost)]
    registry_ghost: Tracked<spec::proof::RegistryGhost>,
}

#[derive(Debug)]
struct Unit {
    name: Arc<str>,
    send: Sender<StateRequest>,
    dependencies: Vec<u64>,
    dependents: Vec<u64>,
    state: State,
    inspect_sensitivity: inspect::SensitivityLevel,
    advances_time: bool,
}

/// An error returned when a state unit name is already in use.
#[derive(Debug, Error)]
#[error("state unit name {0} is in use")]
pub struct NameInUse(Arc<str>);

#[derive(Debug, Error)]
#[error("critical unit communication failure: {name}")]
struct UnitRecvError {
    name: Arc<str>,
    #[source]
    source: RpcError,
}

#[derive(Debug, Clone)]
struct UnitId {
    name: Arc<str>,
    id: u64,
}

/// A handle returned by [`StateUnits::add`], used to remove the state unit.
#[must_use]
#[derive(Debug)]
pub struct UnitHandle {
    id: UnitId,
    inner: Option<Weak<InnerMutex>>,
}

impl Drop for UnitHandle {
    fn drop(&mut self) {
        self.remove_if();
    }
}

impl UnitHandle {
    /// Remove the state unit.
    pub fn remove(mut self) {
        self.remove_if();
    }

    /// Detach this handle, leaving the unit in place indefinitely.
    pub fn detach(mut self) {
        self.inner = None;
    }

    fn remove_if(&mut self) {
        if let Some(inner) = self.inner.take().and_then(|inner| inner.upgrade()) {
            let mut inner = inner.lock();
            assert!(inner.remove_unit(self.id.id), "unit exists");
            inner.names.remove(&self.id.name).expect("unit exists");
        }
    }
}

/// An object returned by [`StateUnits::inspector`] to inspect state units while
/// state transitions may be in flight.
pub struct StateUnitsInspector {
    inner: Weak<InnerMutex>,
}

impl Inspect for StateUnits {
    fn inspect(&self, req: inspect::Request<'_>) {
        self.inner.lock().inspect(req);
    }
}

impl Inspect for StateUnitsInspector {
    fn inspect(&self, req: inspect::Request<'_>) {
        if let Some(inner) = self.inner.upgrade() {
            inner.lock().inspect(req);
        }
    }
}

impl Inspect for Inner {
    fn inspect(&self, req: inspect::Request<'_>) {
        let mut resp = req.respond();
        for unit in self.units.values() {
            resp.sensitivity_child(unit.name.as_ref(), unit.inspect_sensitivity, |req| {
                let mut resp = req.respond();
                if !unit.dependencies.is_empty() {
                    resp.field_with("dependencies", || {
                        // Removing a unit leaves stale ids in other units'
                        // dependency lists (consumers tolerate this via
                        // `get`), so skip any that no longer resolve.
                        unit.dependencies
                            .iter()
                            .filter_map(|id| self.units.get(id).map(|u| u.name.as_ref()))
                            .collect::<Vec<_>>()
                            .join(",")
                    });
                }
                if !unit.dependents.is_empty() {
                    resp.field_with("dependents", || {
                        // See the note above: skip stale ids for removed units.
                        unit.dependents
                            .iter()
                            .filter_map(|id| self.units.get(id).map(|u| u.name.as_ref()))
                            .collect::<Vec<_>>()
                            .join(",")
                    });
                }
                resp.field("unit_state", unit.state)
                    .merge(&inspect::send(&unit.send, StateRequest::Inspect));
            });
        }
    }
}

/// The saved state for an individual unit.
#[derive(Protobuf)]
#[mesh(package = "state_unit")]
pub struct SavedStateUnit {
    /// The name of the state unit.
    #[mesh(1)]
    pub name: String,
    /// The opaque saved state blob.
    #[mesh(2)]
    pub state: SavedStateBlob,
}

/// An error from a state transition.
#[derive(Debug, Error)]
#[error("{op} failed")]
pub struct StateTransitionError {
    op: &'static str,
    #[source]
    errors: UnitErrorSet,
}

fn extract<T, E: Into<anyhow::Error>, U>(
    op: &'static str,
    iter: impl IntoIterator<Item = (Arc<str>, Result<T, E>)>,
    mut f: impl FnMut(Arc<str>, T) -> Option<U>,
) -> Result<Vec<U>, StateTransitionError> {
    let mut result = Vec::new();
    let mut errors = Vec::new();
    for (name, item) in iter {
        match item {
            Ok(t) => {
                if let Some(u) = f(name, t) {
                    result.push(u);
                }
            }
            Err(err) => errors.push((name, err.into())),
        }
    }
    if errors.is_empty() {
        Ok(result)
    } else {
        Err(StateTransitionError {
            op,
            errors: UnitErrorSet(errors),
        })
    }
}

fn check<E: Into<anyhow::Error>>(
    op: &'static str,
    iter: impl IntoIterator<Item = (Arc<str>, Result<(), E>)>,
) -> Result<(), StateTransitionError> {
    extract(op, iter, |_, _| Some(()))?;
    Ok(())
}

#[derive(Debug)]
struct UnitErrorSet(Vec<(Arc<str>, anyhow::Error)>);

impl Display for UnitErrorSet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut map = f.debug_map();
        for (name, err) in &self.0 {
            map.entry(&format_args!("{}", name), &format_args!("{:#}", err));
        }
        map.finish()
    }
}

impl std::error::Error for UnitErrorSet {}

impl StateUnits {
    /// Creates a new instance with no initial units.
    pub fn new() -> Self {
        #[cfg(verus_keep_ghost)]
        let (registry_ghost, units_ghost) = spec::proof::new_ghosts();
        Self {
            inner: Arc::new(InnerMutex::new(Inner {
                next_id: 0,
                units: BTreeMap::new(),
                names: HashMap::new(),
                #[cfg(verus_keep_ghost)]
                registry_ghost,
            })),
            running: false,
            #[cfg(verus_keep_ghost)]
            units_ghost,
        }
    }

    /// Returns an inspector that can be used to inspect the state units while
    /// state transitions are in process.
    pub fn inspector(&self) -> StateUnitsInspector {
        StateUnitsInspector {
            inner: Arc::downgrade(&self.inner),
        }
    }

    /// Save and restore will use `name` as the save ID, so this forms part of
    /// the saved state.
    ///
    /// Note that the added unit will not be running after it is built/spawned,
    /// even if the other units are running. Call
    /// [`StateUnits::start_stopped_units`] when finished adding units.
    pub fn add(&self, name: impl Into<Arc<str>>) -> UnitBuilder<'_> {
        UnitBuilder {
            units: self,
            name: name.into(),
            dependencies: Vec::new(),
            dependents: Vec::new(),
            inspect_sensitivity: inspect::SensitivityLevel::Unspecified,
            advances_time: false,
        }
    }

    /// Check if state units are currently running.
    pub fn is_running(&self) -> bool {
        self.running
    }

    /// Starts any units that are individually stopped, because they were added
    /// via [`StateUnits::add`] while the VM was running.
    ///
    /// Does nothing if all units are stopped, via [`StateUnits::stop`].
    pub async fn start_stopped_units(&mut self) -> anyhow::Result<()> {
        if self.is_running() {
            self.start().await?;
        }
        Ok(())
    }

    /// Starts all the state units.
    pub async fn start(&mut self) -> anyhow::Result<()> {
        self.start_with_rollback().await
    }

    /// Stops all the state units.
    #[cfg_attr(verus_keep_ghost, verus_verify(external_body))]
    #[cfg_attr(verus_keep_ghost, verus_spec(
        requires
            old(self).wf(),
            old(self)@.running,
        ensures
            final(self).wf(),
            !final(self)@.running,
            final(self)@.inventory == old(self)@.inventory,
            final(self)@.advances_time == old(self)@.advances_time,
            final(self)@.units.dom() == old(self)@.units.dom(),
            // Each unit has recorded its state.
            final(self)@.all_recorded(),
    ))]
    pub async fn stop(&mut self) {
        assert!(self.running);
        // Stop units in reverse dependency order so that a dependency is not
        // stopped before its dependant.
        self.run_op(
            "stop",
            None,
            State::Running,
            State::Stopping,
            State::Stopped,
            StateRequest::Stop,
            |_, _| Some(()),
            |unit| &unit.dependents,
        )
        .await;
        self.running = false;
    }

    /// Resets all the state units.
    ///
    /// Panics if running.
    #[cfg_attr(verus_keep_ghost, verus_verify(external_body))]
    #[cfg_attr(verus_keep_ghost, verus_spec(
        requires
            old(self).wf(),
            !old(self)@.running,
        ensures
            final(self).wf(),
            final(self)@.running == old(self)@.running,
            final(self)@.inventory == old(self)@.inventory,
            final(self)@.advances_time == old(self)@.advances_time,
            // The units' states may have changed (they are reset).
            final(self)@.units.dom() == old(self)@.units.dom(),
    ))]
    pub async fn reset(&mut self) -> Result<(), StateTransitionError> {
        assert!(!self.running);
        // Reset in dependency order so that dependants observe their
        // dependencies' reset state.
        let r = self
            .run_op(
                "reset",
                None,
                State::Stopped,
                State::Resetting,
                State::Stopped,
                StateRequest::Reset,
                |_, _| Some(()),
                |unit| &unit.dependencies,
            )
            .await;

        check("reset", r)?;
        Ok(())
    }

    /// Saves all the state units.
    ///
    /// Panics if running.
    #[cfg_attr(verus_keep_ghost, verus_verify(external_body))]
    #[cfg_attr(verus_keep_ghost, verus_spec(result =>
        requires
            old(self).wf(),
            !old(self)@.running,
            old(self)@.all_recorded(),
        ensures
            final(self).wf(),
            final(self)@ == old(self)@,
            // One entry per unit with saved state, holding that state.
            result matches Ok(states) ==> {
                &&& SavedStateUnit::distinct_names(states@)
                &&& SavedStateUnit::map_of(states@) == old(self)@.saved_units()
            },
    ))]
    pub async fn save(&mut self) -> Result<Vec<SavedStateUnit>, StateTransitionError> {
        assert!(!self.running);
        // Save can occur in any order since it will not observably mutate
        // state.
        let r = self
            .run_op(
                "save",
                None,
                State::Stopped,
                State::Saving,
                State::Stopped,
                StateRequest::Save,
                |_, _| Some(()),
                |_| &[],
            )
            .await;

        let states = extract("save", r, |name, state| {
            state.map(|state| SavedStateUnit {
                name: name.to_string(),
                state,
            })
        })?;

        Ok(states)
    }

    /// Restores all the state units.
    ///
    /// Panics if running.
    #[cfg_attr(verus_keep_ghost, verus_verify(external_body))]
    #[cfg_attr(verus_keep_ghost, verus_spec(result =>
        requires
            old(self).wf(),
            !old(self)@.running,
        ensures
            final(self).wf(),
            final(self)@.running == old(self)@.running,
            final(self)@.inventory == old(self)@.inventory,
            final(self)@.advances_time == old(self)@.advances_time,
            final(self)@.units.dom() == old(self)@.units.dom(),
            // Each unit in `states` (by name, once each, and registered)
            // records its saved state; the others keep theirs.
            result is Ok ==> {
                &&& SavedStateUnit::distinct_names(states@)
                &&& SavedStateUnit::map_of(states@).dom() <= old(self)@.units.dom()
                &&& final(self)@.units == old(self)@.units.union_prefer_right(
                    SavedStateUnit::map_of(states@).map_values(
                        |blob: SavedStateBlobView| UnitSnapshot::Saved(blob),
                    ),
                )
            },
    ))]
    pub async fn restore(
        &mut self,
        states: Vec<SavedStateUnit>,
    ) -> Result<(), StateTransitionError> {
        assert!(!self.running);

        #[derive(Debug, Error)]
        enum RestoreUnitError {
            #[error("unknown unit name")]
            Unknown,
            #[error("duplicate unit name")]
            Duplicate,
        }

        let mut states_by_id = HashMap::new();
        let mut r = Vec::new();
        {
            let inner = self.inner.lock();
            for state in states {
                match inner.names.get(state.name.as_str()) {
                    Some(&id) => {
                        // The same `Arc` as the key in `names` (see `build`).
                        let name = &inner.units[&id].name;
                        if states_by_id
                            .insert(id, (name.clone(), state.state))
                            .is_some()
                        {
                            r.push((name.clone(), Err(RestoreUnitError::Duplicate)));
                        }
                    }
                    None => {
                        r.push((state.name.into(), Err(RestoreUnitError::Unknown)));
                    }
                }
            }
        }

        check("restore", r)?;

        let r = self
            .run_op(
                "restore",
                None,
                State::Stopped,
                State::Restoring,
                State::Stopped,
                StateRequest::Restore,
                |id, _| states_by_id.remove(&id).map(|(_, blob)| blob),
                |unit| &unit.dependencies,
            )
            .await;

        // Make sure all the saved state was consumed. This could hit if a unit
        // was removed concurrently with the restore.
        check(
            "restore",
            states_by_id
                .into_iter()
                .map(|(_, (name, _))| (name, Err(RestoreUnitError::Unknown))),
        )?;

        check("restore", r)?;

        Ok(())
    }

    /// Runs a state change operation on a set of units.
    ///
    /// `op` gives the name of the operation for tracing and error reporting
    /// purposes.
    ///
    /// `unit_ids` is the set of the units whose state should be changed. If
    /// `unit_ids` is `None`, all units change states.
    ///
    /// The old state for each unit must be `old_state`. During the operation,
    /// the unit is temporarily places in `interim_state`. When complete, the
    /// unit is placed in `new_state`.
    ///
    /// Each unit waits for its dependencies to complete their state change
    /// operation before proceeding with their own state change. The
    /// dependencies list is computed for a unit by calling `deps`.
    ///
    /// To perform the state change, the unit is sent a request generated using
    /// `request`, with input generated by `input`. If `input` returns `None`,
    /// then communication with the unit is skipped, but the unit still
    /// transitions through the interim and into the new state, and its
    /// dependencies are still waited on by its dependents.
    async fn run_op<I: 'static, R: 'static + Send>(
        &self,
        op: &str,
        unit_ids: Option<&[u64]>,
        old_state: State,
        interim_state: State,
        new_state: State,
        request: impl Copy + FnOnce(Rpc<I, R>) -> StateRequest,
        mut input: impl FnMut(u64, &Unit) -> Option<I>,
        mut deps: impl FnMut(&Unit) -> &[u64],
    ) -> Vec<(Arc<str>, R)> {
        let mut done = Vec::new();
        let ready_set;
        {
            let mut inner = self.inner.lock();
            ready_set = inner.ready_set(unit_ids);
            let ids: Vec<u64> = inner.units.keys().copied().collect();
            for id in ids {
                if !ready_set.0.contains_key(&id) {
                    continue;
                }
                let unit = inner.units.get_mut(&id).unwrap();
                if unit.state != old_state {
                    assert_eq!(
                        unit.state, new_state,
                        "unit {} in {:?} state, should be {:?} or {:?}",
                        unit.name, unit.state, old_state, new_state
                    );
                    ready_set.done(id, true);
                } else {
                    let name = unit.name.clone();
                    let input = (input)(id, unit);
                    let deps = deps(unit).to_vec();
                    let fut = state_change(name.clone(), unit.send.clone(), request, input);
                    let recv = wait_then_change(ready_set.clone(), op, id, deps, name, fut);
                    done.push(recv);
                    unit.state = interim_state;
                }
            }
        }

        let results = join_all_timed(done)
            .instrument(tracing::info_span!("state_change", operation = op))
            .await;

        let mut inner = self.inner.lock();
        let mut r = Vec::new();
        for (name, id, result) in results {
            match result {
                Ok(Some(result)) => r.push((name, result)),
                Ok(None) => {}
                Err(err) => {
                    // If the unit was removed during the operation, then
                    // ignore the failure. Otherwise, panic because unit
                    // failure is not recoverable. FUTURE: reconsider this
                    // position.
                    if inner.units.contains_key(&id) {
                        panic!("{:?}", err);
                    }
                }
            }
        }
        let ids: Vec<u64> = inner.units.keys().copied().collect();
        for id in ids {
            if !ready_set.0.contains_key(&id) {
                continue;
            }
            let unit = inner.units.get_mut(&id).unwrap();
            if unit.state == interim_state {
                unit.state = new_state;
            } else {
                assert_eq!(
                    unit.state, new_state,
                    "unit {} in {:?} state, should be {:?} or {:?}",
                    unit.name, unit.state, interim_state, new_state
                );
            }
        }
        r
    }
}

impl Inner {
    fn ready_set(&self, unit_ids: Option<&[u64]>) -> ReadySet {
        let map = |id, unit: &Unit| {
            (
                id,
                ReadyState {
                    name: unit.name.clone(),
                    ready: Arc::new(Ready::default()),
                },
            )
        };
        let units = if let Some(unit_ids) = unit_ids {
            unit_ids
                .iter()
                .map(|id| map(*id, &self.units[id]))
                .collect()
        } else {
            self.units.iter().map(|(id, unit)| map(*id, unit)).collect()
        };
        ReadySet(Arc::new(units))
    }
}

#[derive(Clone)]
struct ReadySet(Arc<BTreeMap<u64, ReadyState>>);

#[derive(Clone)]
struct ReadyState {
    name: Arc<str>,
    ready: Arc<Ready>,
}

impl ReadySet {
    async fn wait(&self, op: &str, id: u64, deps: &[u64]) -> bool {
        for dep in deps {
            if let Some(dep) = self.0.get(dep) {
                if !dep.ready.is_ready() {
                    tracing::debug!(
                        device = self.0[&id].name.as_ref(),
                        dependency = dep.name.as_ref(),
                        operation = op,
                        "waiting on dependency"
                    );
                }
                if !dep.ready.wait().await {
                    return false;
                }
            }
        }
        true
    }

    fn done(&self, id: u64, success: bool) {
        self.0[&id].ready.signal(success);
    }
}

/// Waits for the dependencies `deps` of unit `id` to finish, runs `fut` (the
/// unit's state change), and marks unit `id` done.
async fn wait_then_change<F: Future>(
    ready_set: ReadySet,
    op: &str,
    id: u64,
    deps: Vec<u64>,
    name: Arc<str>,
    fut: F,
) -> (Arc<str>, u64, F::Output) {
    ready_set.wait(op, id, &deps).await;
    let r = fut.await;
    ready_set.done(id, true);
    (name, id, r)
}

/// Awaits all of `futs`, logging how long it took.
async fn join_all_timed<F: Future>(futs: Vec<F>) -> Vec<F::Output> {
    let start = Instant::now();
    let results = join_all(futs).await;
    tracing::info!(duration = ?Instant::now() - start, "state change complete");
    results
}

/// Sends state change `request` to the unit at `send` with `input`, wrapping
/// the call with a span, and wrapping its error with something more
/// informative.
///
/// `name` is used in tracing and error construction.
async fn state_change<I: 'static, R: 'static + Send, Req: FnOnce(Rpc<I, R>) -> StateRequest>(
    name: Arc<str>,
    send: Sender<StateRequest>,
    request: Req,
    input: Option<I>,
) -> Result<Option<R>, UnitRecvError> {
    let Some(input) = input else { return Ok(None) };
    let span = tracing::info_span!("device_state_change", device = name.as_ref());
    call_unit(name, send, request, input)
        .instrument(span)
        .await
        .map(Some)
}

/// Sends `request` with `input` to the unit at `send` and waits for the
/// reply, logging how long it took.
async fn call_unit<I: 'static, R: 'static + Send, Req: FnOnce(Rpc<I, R>) -> StateRequest>(
    name: Arc<str>,
    send: Sender<StateRequest>,
    request: Req,
    input: I,
) -> Result<R, UnitRecvError> {
    let start = Instant::now();
    // Through the mesh RPC wrapper, with placeholder ghosts: the halves are
    // not yet passed (`run_op` is unverified).
    let r = mesh_rpc_verus::call(
        &send,
        request,
        input,
        Tracked::assume_new(),
        Ghost::assume_new(),
    )
    .recv()
    .await
    .map(|(r, _ghost)| r)
    .map_err(|err| UnitRecvError { name, source: err });
    tracing::debug!(duration = ?Instant::now() - start, "device state change complete");
    r
}

/// A builder returned by [`StateUnits::add`].
#[derive(Debug)]
#[must_use]
pub struct UnitBuilder<'a> {
    units: &'a StateUnits,
    name: Arc<str>,
    dependencies: Vec<u64>,
    dependents: Vec<u64>,
    inspect_sensitivity: inspect::SensitivityLevel,
    advances_time: bool,
}

impl UnitBuilder<'_> {
    /// Marks this new unit as one whose [`StateUnit::advance_time`] does
    /// work, so that [`StateUnits::advance_time`] sends it the request.
    /// Other units derive their deadlines from VM time and are not contacted.
    pub fn advances_time(mut self) -> Self {
        self.advances_time = true;
        self
    }

    /// Adds `handle` as a dependency of this new unit.
    ///
    /// Operations will be ordered to ensure that a dependency will stop after
    /// its dependants, and that it will reset or restore before its dependants.
    pub fn depends_on(mut self, handle: &UnitHandle) -> Self {
        self.dependencies.push(self.handle_id(handle));
        self
    }

    /// Adds this new unit as a dependency of `handle`.
    ///
    /// Operations will be ordered to ensure that a dependency will stop after
    /// its dependants, and that it will reset or restore before its dependants.
    pub fn dependency_of(mut self, handle: &UnitHandle) -> Self {
        self.dependents.push(self.handle_id(handle));
        self
    }

    /// Sets the sensitivity level for this unit's inspect data. Defaults to Unspecified.
    pub fn inspect_sensitivity(mut self, sensitivity: inspect::SensitivityLevel) -> Self {
        self.inspect_sensitivity = sensitivity;
        self
    }

    fn handle_id(&self, handle: &UnitHandle) -> u64 {
        // Ensure this handle is associated with this set of state units.
        assert_eq!(
            Weak::as_ptr(handle.inner.as_ref().unwrap()),
            Arc::as_ptr(&self.units.inner)
        );
        handle.id.id
    }

    /// Adds a new state unit sending requests to `send`.
    pub fn build(mut self, send: Sender<StateRequest>) -> Result<UnitHandle, NameInUse> {
        let id = {
            let mut inner = self.units.inner.lock();
            let id = inner.next_id;
            let entry = match inner.names.entry(self.name.clone()) {
                hash_map::Entry::Occupied(_) => return Err(NameInUse(self.name)),
                hash_map::Entry::Vacant(e) => e,
            };
            entry.insert(id);

            // Dedup the dependencies and update the dependencies' lists of
            // dependents.
            self.dependencies.sort();
            self.dependencies.dedup();
            for &dep in &self.dependencies {
                inner.units.get_mut(&dep).unwrap().dependents.push(id);
            }

            // Dedup the depenedents and update the dependents' lists of
            // dependencies.
            self.dependents.sort();
            self.dependents.dedup();
            for &dep in &self.dependents {
                inner.units.get_mut(&dep).unwrap().dependencies.push(id);
            }
            inner.insert_unit(
                id,
                Unit {
                    name: self.name.clone(),
                    send,
                    dependencies: self.dependencies,
                    dependents: self.dependents,
                    state: State::Stopped,
                    inspect_sensitivity: self.inspect_sensitivity,
                    advances_time: self.advances_time,
                },
            );
            let unit_id = UnitId {
                name: self.name,
                id,
            };
            inner.next_id += 1;
            unit_id
        };
        Ok(UnitHandle {
            id,
            inner: Some(Arc::downgrade(&self.units.inner)),
        })
    }

    /// Adds a unit as in [`Self::build`], then spawns a task for running the
    /// unit.
    ///
    /// The channel to receive state change requests is passed to `f`, which
    /// should return the future to evaluate to run the unit.
    #[track_caller]
    pub fn spawn<F, Fut>(
        self,
        spawner: impl Spawn,
        f: F,
    ) -> Result<SpawnedUnit<Fut::Output>, NameInUse>
    where
        F: FnOnce(Receiver<StateRequest>) -> Fut,
        Fut: 'static + Send + Future,
        Fut::Output: 'static + Send,
    {
        self.spawn_with_channel(spawner, unit_channel(), f)
    }

    /// As [`Self::spawn`], with the unit's channel made by the caller
    /// (`unit_channel_for`, for a unit whose state was created before its
    /// channel and keeps its own share of the unit's FracGhost).
    #[track_caller]
    pub fn spawn_with_channel<F, Fut>(
        self,
        spawner: impl Spawn,
        (send, recv): (Sender<StateRequest>, Receiver<StateRequest>),
        f: F,
    ) -> Result<SpawnedUnit<Fut::Output>, NameInUse>
    where
        F: FnOnce(Receiver<StateRequest>) -> Fut,
        Fut: 'static + Send + Future,
        Fut::Output: 'static + Send,
    {
        let task_name = format!("unit-{}", self.name);
        let handle = self.build(send)?;
        let fut = (f)(recv);
        let task = spawner.spawn(task_name, fut);
        Ok(SpawnedUnit { task, handle })
    }
}

/// A handle to a spawned unit.
#[must_use]
pub struct SpawnedUnit<T> {
    handle: UnitHandle,
    task: Task<T>,
}

impl<T> Debug for SpawnedUnit<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SpawnedUnit")
            .field("handle", &self.handle)
            .field("task", &self.task)
            .finish()
    }
}

impl<T> SpawnedUnit<T> {
    /// Removes the unit and returns it.
    pub async fn remove(self) -> T {
        self.handle.remove();
        self.task.await
    }

    /// Gets the unit handle for use with methods like
    /// [`UnitBuilder::depends_on`].
    pub fn handle(&self) -> &UnitHandle {
        &self.handle
    }
}

#[derive(Default)]
struct Ready {
    state: AtomicU32,
    event: event_listener::Event,
}

impl Ready {
    /// Wakes everyone with `success`.
    fn signal(&self, success: bool) {
        self.state.store(success as u32 + 1, Ordering::Release);
        self.event.notify(usize::MAX);
    }

    fn is_ready(&self) -> bool {
        self.state.load(Ordering::Acquire) != 0
    }

    /// Waits for `signal` to be called and returns its `success` parameter.
    async fn wait(&self) -> bool {
        loop {
            let listener = self.event.listen();
            let state = self.state.load(Ordering::Acquire);
            if state != 0 {
                return state - 1 != 0;
            }
            listener.await;
        }
    }
}

#[cfg(test)]
mod tests {
    mod concurrent_inspect;
    mod inventory;
    mod quiesce;
    mod start;

    use super::StateUnit;
    use super::StateUnits;
    use crate::run_unit;
    use inspect::InspectMut;
    use mesh::payload::Protobuf;
    use pal_async::DefaultDriver;
    use pal_async::async_test;
    use std::sync::Arc;
    use std::sync::atomic::AtomicBool;
    use std::sync::atomic::Ordering;
    use std::time::Duration;
    use test_with_tracing::test;
    use vmcore::save_restore::RestoreError;
    use vmcore::save_restore::SaveError;
    use vmcore::save_restore::SavedStateBlob;
    use vmcore::save_restore::SavedStateRoot;

    #[derive(Default)]
    struct TestUnit {
        value: Arc<AtomicBool>,
        dep: Option<Arc<AtomicBool>>,
        /// If we should support saved state or not.
        support_saved_state: bool,
    }

    #[derive(Protobuf, SavedStateRoot)]
    #[mesh(package = "test")]
    struct SavedState(bool);

    impl StateUnit for TestUnit {
        async fn start(&mut self) -> anyhow::Result<()> {
            Ok(())
        }

        async fn stop(&mut self) {}

        async fn reset(&mut self) -> anyhow::Result<()> {
            Ok(())
        }

        async fn save(&mut self) -> Result<Option<SavedStateBlob>, SaveError> {
            if self.support_saved_state {
                let state = SavedState(self.value.load(Ordering::Relaxed));
                Ok(Some(SavedStateBlob::new(state)))
            } else {
                Ok(None)
            }
        }

        async fn restore(&mut self, state: SavedStateBlob) -> Result<(), RestoreError> {
            assert!(self.dep.as_ref().is_none_or(|v| v.load(Ordering::Relaxed)));

            if self.support_saved_state {
                let state: SavedState = state.parse()?;
                self.value.store(state.0, Ordering::Relaxed);
                Ok(())
            } else {
                Err(RestoreError::SavedStateNotSupported)
            }
        }
    }

    impl InspectMut for TestUnit {
        fn inspect_mut(&mut self, req: inspect::Request<'_>) {
            req.respond();
        }
    }

    struct TestUnitSetDep {
        dep: Arc<AtomicBool>,
        driver: DefaultDriver,
    }

    impl StateUnit for TestUnitSetDep {
        async fn start(&mut self) -> anyhow::Result<()> {
            Ok(())
        }

        async fn stop(&mut self) {}

        async fn reset(&mut self) -> anyhow::Result<()> {
            Ok(())
        }

        async fn save(&mut self) -> Result<Option<SavedStateBlob>, SaveError> {
            Ok(Some(SavedStateBlob::new(SavedState(true))))
        }

        async fn restore(&mut self, _state: SavedStateBlob) -> Result<(), RestoreError> {
            pal_async::timer::PolledTimer::new(&self.driver)
                .sleep(Duration::from_millis(100))
                .await;

            self.dep.store(true, Ordering::Relaxed);

            Ok(())
        }
    }

    impl InspectMut for TestUnitSetDep {
        fn inspect_mut(&mut self, req: inspect::Request<'_>) {
            req.respond();
        }
    }

    /// A unit that records whether it was asked to advance time.
    struct TimeUnit(Arc<AtomicBool>);

    impl StateUnit for TimeUnit {
        async fn start(&mut self) -> anyhow::Result<()> {
            Ok(())
        }

        async fn stop(&mut self) {}

        async fn reset(&mut self) -> anyhow::Result<()> {
            Ok(())
        }

        async fn save(&mut self) -> Result<Option<SavedStateBlob>, SaveError> {
            Ok(None)
        }

        async fn restore(&mut self, _state: SavedStateBlob) -> Result<(), RestoreError> {
            Ok(())
        }

        async fn advance_time(&mut self, _duration: Duration) -> anyhow::Result<()> {
            self.0.store(true, Ordering::Relaxed);
            Ok(())
        }
    }

    impl InspectMut for TimeUnit {
        fn inspect_mut(&mut self, req: inspect::Request<'_>) {
            req.respond();
        }
    }

    #[async_test]
    async fn advance_time_reaches_only_units_that_advance_time(driver: DefaultDriver) {
        let mut units = StateUnits::new();
        let flagged = Arc::new(AtomicBool::new(false));
        let unflagged = Arc::new(AtomicBool::new(false));
        let a = units
            .add("flagged")
            .advances_time()
            .spawn(&driver, |recv| run_unit(TimeUnit(flagged.clone()), recv))
            .unwrap();
        let _b = units
            .add("unflagged")
            .depends_on(a.handle())
            .spawn(&driver, |recv| run_unit(TimeUnit(unflagged.clone()), recv))
            .unwrap();

        units.advance_time(Duration::from_secs(1)).await.unwrap();
        assert!(flagged.load(Ordering::Relaxed));
        assert!(!unflagged.load(Ordering::Relaxed));

        // Both units are stopped again and can start.
        units.start().await.unwrap();
        units.stop().await;
    }

    #[async_test]
    async fn test_state_change(driver: DefaultDriver) {
        let mut units = StateUnits::new();

        let a_val = Arc::new(AtomicBool::new(true));

        let _a = units
            .add("a")
            .spawn(&driver, |recv| {
                run_unit(
                    TestUnit {
                        value: a_val.clone(),
                        dep: None,
                        support_saved_state: true,
                    },
                    recv,
                )
            })
            .unwrap();
        let _b = units
            .add("b")
            .spawn(&driver, |recv| run_unit(TestUnit::default(), recv))
            .unwrap();
        units.start().await.unwrap();

        let _c = units
            .add("c")
            .spawn(&driver, |recv| run_unit(TestUnit::default(), recv));
        units.stop().await;
        units.start().await.unwrap();

        units.stop().await;

        let state = units.save().await.unwrap();

        a_val.store(false, Ordering::Relaxed);

        units.restore(state).await.unwrap();

        assert!(a_val.load(Ordering::Relaxed));
    }

    #[async_test]
    async fn test_dependencies(driver: DefaultDriver) {
        let mut units = StateUnits::new();

        let a_val = Arc::new(AtomicBool::new(true));

        let a = units
            .add("zzz")
            .spawn(&driver, |recv| {
                run_unit(
                    TestUnit {
                        value: a_val.clone(),
                        dep: None,
                        support_saved_state: true,
                    },
                    recv,
                )
            })
            .unwrap();

        let _b = units
            .add("aaa")
            .depends_on(a.handle())
            .spawn(&driver, |recv| {
                run_unit(
                    TestUnit {
                        dep: Some(a_val.clone()),
                        value: Default::default(),
                        support_saved_state: true,
                    },
                    recv,
                )
            })
            .unwrap();
        units.start().await.unwrap();
        units.stop().await;

        let state = units.save().await.unwrap();

        a_val.store(false, Ordering::Relaxed);

        units.restore(state).await.unwrap();
    }

    #[async_test]
    async fn test_dep_no_saved_state(driver: DefaultDriver) {
        let mut units = StateUnits::new();

        let true_val = Arc::new(AtomicBool::new(true));
        let shared_val = Arc::new(AtomicBool::new(false));

        let a = units
            .add("a")
            .spawn(&driver, |recv| {
                run_unit(
                    TestUnit {
                        value: true_val.clone(),
                        dep: Some(shared_val.clone()),
                        support_saved_state: true,
                    },
                    recv,
                )
            })
            .unwrap();

        // no saved state
        // Note that restore is never called for this unit.
        let b = units
            .add("b_no_saved_state")
            .dependency_of(a.handle())
            .spawn(&driver, |recv| {
                run_unit(
                    TestUnit {
                        value: true_val.clone(),
                        dep: Some(shared_val.clone()),
                        support_saved_state: false,
                    },
                    recv,
                )
            })
            .unwrap();

        // A has a transitive dependency on C via B.
        let _c = units
            .add("c")
            .dependency_of(b.handle())
            .spawn(&driver, |recv| {
                run_unit(
                    TestUnitSetDep {
                        dep: shared_val,
                        driver: driver.clone(),
                    },
                    recv,
                )
            })
            .unwrap();

        let state = units.save().await.unwrap();

        units.restore(state).await.unwrap();
    }
}
