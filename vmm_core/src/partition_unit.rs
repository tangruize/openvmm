// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

//! State unit for managing the VM partition and associated virtual processors.

mod debug;
mod snapshot;
mod vp_set;

/// Verus specification of the partition unit (its request channel and its
/// state unit).
#[cfg(verus_keep_ghost)]
mod spec {
    include!("partition_unit.spec.rs");
}

/// Trusted: the encoding of a halt reason (protobuf).
#[cfg(verus_keep_ghost)]
mod trusted {
    include!("partition_unit_t.rs");
}

pub use vp_set::Halt;
pub use vp_set::RequestYield;
pub use vp_set::RunCancelled;
pub use vp_set::RunnerCanceller;
pub use vp_set::VpRunner;
pub use vp_set::block_on_vp;

use self::vp_set::RegisterSetError;
#[cfg(feature = "dump")]
use anyhow::Context as _;
use async_trait::async_trait;
use futures::FutureExt;
use futures::StreamExt;
use guestmem::GuestMemory;
use hvdef::Vtl;
use inspect::InspectMut;
use mesh::Receiver;
use mesh::rpc::Rpc;
use mesh::rpc::RpcSend;
#[cfg(verus_keep_ghost)]
use mesh_rpc_verus::ReceiverAdditionalSpecFns;
use pal_async::task::Spawn;
use state_unit::NameInUse;
use state_unit::SpawnedUnit;
use state_unit::StateRequest;
use state_unit::StateUnit;
#[cfg(verus_keep_ghost)]
use state_unit::StateUnitSpec;
use state_unit::UnitBuilder;
use state_unit::UnitHandle;
use std::sync::Arc;
use thiserror::Error;
use virt::InitialPageImport;
use virt::InitialRegs;
use virt::InitialVpStateSource;
#[cfg(feature = "dump")]
use virt::VpIndex;
use vm_topology::processor::ProcessorTopology;
use vmcore::save_restore::ProtobufSaveRestore;
use vmcore::save_restore::RestoreError;
use vmcore::save_restore::SaveError;
use vmcore::save_restore::SavedStateBlob;
#[cfg(verus_keep_ghost)]
use vmcore::save_restore::spec::UnitSnapshot;
use vmm_core_defs::HaltReason;
use vp_set::VpSet;
use vstd::prelude::Ghost;
use vstd::prelude::Tracked;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
use vstd::resource::frac::FracGhost;

#[cfg(verus_keep_ghost)]
mod proof {
    include!("partition_unit.proof.rs");
}

/// The control point for managing a partition unit.
pub struct PartitionUnit {
    handle: SpawnedUnit<PartitionUnitRunner>,
    req_send: mesh::Sender<PartitionRequest>,
}

/// Trait with the minimal methods needed to run the partition.
#[async_trait]
pub trait VmPartition: 'static + Send + Sync + InspectMut + ProtobufSaveRestore {
    /// Returns the source of the initial virtual processor state.
    fn initial_vp_state_source(&self) -> InitialVpStateSource;

    /// Resets the partition.
    fn reset(&mut self) -> anyhow::Result<()>;

    /// Scrubs the VTL state for a partition.
    fn scrub_vtl(&mut self, vtl: Vtl) -> anyhow::Result<()>;

    /// Finalizes initial page imports on behalf of the loader.
    fn accept_initial_pages(&mut self, pages: Vec<InitialPageImport>) -> anyhow::Result<()>;

    /// Returns the guest OS ID (from `HV_X64_MSR_GUEST_OS_ID`).
    ///
    /// Returns 0 if the guest hasn't written the MSR (unenlightened guest)
    /// or if the backend doesn't support reading it.
    fn guest_os_id(&self) -> u64 {
        0
    }
}

/// An object to run the VM partition state unit.
/// The partition's processor topology, in a struct of its own. (Verus:
/// `ProcessorTopology`'s generic bound needs `AsRef`, which this toolchain's
/// Verus cannot declare, so this struct is declared by name only instead.)
struct Topology(ProcessorTopology);

#[cfg_attr(verus_keep_ghost, verus_verify)]
struct PartitionUnitRunner {
    partition: Box<dyn VmPartition>,
    vp_set: VpSet,
    unit_started: bool,
    vp_stop_count: usize,
    needs_reset: bool,
    halt_reason: Option<HaltReason>,
    halt_request_recv: Receiver<InternalHaltReason>,
    client_notify_send: mesh::Sender<HaltReason>,
    req_recv: Receiver<PartitionRequest>,
    topology: Topology,
    initial_regs: Option<Arc<InitialRegs>>,
    /// Whether restores drop the VPs' saved TSC values (NVX time ABI).
    #[cfg(guest_arch = "x86_64")]
    omit_saved_tsc: bool,

    #[cfg(feature = "gdb")]
    debugger_state: debug::DebuggerState,
    /// The unit's share of its FracGhost (Verus only).
    #[cfg(verus_keep_ghost)]
    share: Tracked<vmcore::device_state::spec::DeviceGhostState>,
    /// The authority of the stop tokens (Verus only): the ids of the
    /// outstanding ones, one per stop reference (`vp_stop_count`); see the
    /// request protocol (`partition_unit.spec.rs`).
    #[cfg(verus_keep_ghost)]
    stop_tokens: Tracked<vstd::resource::set::GhostSetAuth<u64>>,
}

impl InspectMut for PartitionUnitRunner {
    fn inspect_mut(&mut self, req: inspect::Request<'_>) {
        req.respond()
            .field(
                "power_state",
                self.halt_reason.as_ref().map_or("running", |_| "halted"),
            )
            .merge(&self.halt_reason)
            .merge(&self.vp_set)
            .field_mut_with("clear_halt", |clear| {
                // Clear halt if "true" is specified.
                if let Some(clear) = clear {
                    match clear.parse::<bool>() {
                        Ok(x) => {
                            if x {
                                self.clear_halt();
                            }
                            Ok(x)
                        }
                        Err(err) => Err(err),
                    }
                } else {
                    Ok(false)
                }
            })
            .field("topology", &self.topology.0)
            .merge(&mut self.partition);
    }
}

#[cfg_attr(verus_keep_ghost, verus_verify)]
enum PartitionRequest {
    ClearHalt(Rpc<(), bool>), // TODO: remove this, and use DebugRequest::Resume
    SetInitialRegs(Rpc<(Vtl, Arc<InitialRegs>), Result<(), InitialRegError>>),
    AcceptInitialPages(Rpc<Vec<InitialPageImport>, Result<(), AcceptInitialPagesError>>),
    StopVps(Rpc<(), ()>),
    Snapshot(snapshot::SnapshotRequest),
    StartVps,
    /// Build the partition state blob for a dump file.
    #[cfg(feature = "dump")]
    BuildDumpPartitionState(Rpc<(), anyhow::Result<Vec<u8>>>),
}

pub struct PartitionUnitParams<'a> {
    pub vtl_guest_memory: [Option<&'a GuestMemory>; 3],
    pub processor_topology: &'a ProcessorTopology,
    /// VP prefix to instantiate. The full topology remains guest-visible.
    pub active_vp_count: Option<u32>,
    /// Tracks the halt state of VPs.
    pub halt_vps: Arc<Halt>,
    /// The receiver returned from `Halt::new()`.
    pub halt_request_recv: HaltReasonReceiver,
    /// Notified when the partition has been halted (due to a triple fault or
    /// other reason).
    pub client_notify_send: mesh::Sender<HaltReason>,
    pub debugger_rpc: Option<Receiver<vmm_core_defs::debug_rpc::DebugRequest>>,
}

/// The halt reason receiver to pass to put in [`PartitionUnitParams`].
pub struct HaltReasonReceiver(Receiver<InternalHaltReason>);

enum InternalHaltReason {
    Halt(HaltReason),
    ReplayMtrrs,
}

/// Error returned by [`PartitionUnit::new()`].
#[derive(Debug, Error)]
pub enum Error {
    #[error("debugging is not supported in this build")]
    DebuggingNotSupported,
    #[error(transparent)]
    NameInUse(NameInUse),
    #[error("missing guest memory required for gdb support")]
    MissingGuestMemory,
    #[error("active VP count {active} is outside topology capacity 1..={capacity}")]
    InvalidActiveVpCount { active: u32, capacity: u32 },
}

/// Error returned by [`PartitionUnit::set_initial_regs()`].
#[derive(Debug, Error)]
pub enum InitialRegError {
    #[error("failed to set registers")]
    RegisterSet(#[source] RegisterSetError),
    #[error("failed to scrub VTL state")]
    ScrubVtl(#[source] anyhow::Error),
}

/// Error returned by [`PartitionUnit::accept_initial_pages()`].
#[derive(Debug, Error)]
pub enum AcceptInitialPagesError {
    #[error("failed to finalize initial page imports")]
    Finalize(#[source] anyhow::Error),
}

impl PartitionUnit {
    /// Creates a new VM partition state unit.
    ///
    /// The caller is responsible for launching a thread for each VP and running
    /// the VP using the returned [`VpRunner`]s.
    pub fn new(
        spawner: impl Spawn,
        builder: UnitBuilder<'_>,
        partition: impl VmPartition,
        params: PartitionUnitParams<'_>,
    ) -> Result<(Self, Vec<VpRunner>), Error> {
        #[cfg(not(feature = "gdb"))]
        if params.debugger_rpc.is_some() {
            return Err(Error::DebuggingNotSupported);
        }

        let active_vp_count = snapshot::active_vp_count(&params)?;
        let mut vp_set = VpSet::new(
            params.vtl_guest_memory.map(|m| m.cloned()),
            params.halt_vps,
            params.processor_topology.vp_count() as usize,
        );
        let vps = params
            .processor_topology
            .vps_arch()
            .take(active_vp_count as usize)
            .map(|vp| vp_set.add(vp))
            .collect();

        let (req_send, req_recv) = mesh::channel();

        // The runner's parts, computed here (in the same order as before, and
        // with the same error), so that the unit's task, once its channel
        // exists, only assembles them (`run_partition_unit`).
        let parts = RunnerParts {
            partition: Box::new(partition),
            vp_set,
            halt_request_recv: params.halt_request_recv.0,
            client_notify_send: params.client_notify_send,
            req_recv,
            topology: Topology(params.processor_topology.clone()),
            #[cfg(feature = "gdb")]
            debugger_state: debug::DebuggerState::new(
                params.vtl_guest_memory[0]
                    .ok_or(Error::MissingGuestMemory)?
                    .clone(),
                params.debugger_rpc,
            ),
        };

        let handle = builder
            .spawn(spawner, |recv| async move {
                run_partition_unit(parts, recv).await
            })
            .unwrap();

        Ok((Self { handle, req_send }, vps))
    }

    /// Gets the handle for the partition unit.
    pub fn unit_handle(&self) -> &UnitHandle {
        self.handle.handle()
    }

    /// Tears down the state unit, returning the `client_notify_send` sender
    /// passed to [`Self::new()`].
    pub async fn teardown(self) -> mesh::Sender<HaltReason> {
        let runner = self.handle.remove().await;
        runner.vp_set.teardown().await;
        runner.client_notify_send
    }

    /// Clears the current halt reason from the partition, resuming the VPs if
    /// they are stopped.
    pub async fn clear_halt(&mut self) -> bool {
        self.req_send
            .call(PartitionRequest::ClearHalt, ())
            .await
            .unwrap()
    }

    /// Temporarily stops the VPs, returning a guard that will resume them when
    /// dropped.
    pub async fn temporarily_stop_vps(&mut self) -> StopGuard {
        // Through the mesh RPC wrapper.
        // PROOF TODO: replace these placeholders (`assume_new()`) with the
        // real ghosts: none sent (`None`), and the protocol's reply
        // expectation for `StopVps`; keep the returned stop token in the
        // `StopGuard`.
        mesh_rpc_verus::call(
            &self.req_send,
            PartitionRequest::StopVps,
            (),
            Tracked::assume_new(),
            Ghost::assume_new(),
        )
        .recv()
        .await
        .unwrap();

        StopGuard {
            0: self.req_send.clone(),
            // PROOF TODO: replace this placeholder (`assume_new()`) with the
            // real ghost: the stop token from `StopVps`'s reply.
            #[cfg(verus_keep_ghost)]
            1: Tracked::assume_new(),
        }
    }

    /// Sets the register state for the VPs for initial boot.
    ///
    /// If the VM has been run before and has not been reset since it last ran,
    /// the target VTL will be scrubbed first so that the partition state is
    /// clean.
    pub async fn set_initial_regs(
        &mut self,
        vtl: Vtl,
        state: Arc<InitialRegs>,
    ) -> Result<(), InitialRegError> {
        self.req_send
            .call(PartitionRequest::SetInitialRegs, (vtl, state))
            .await
            .unwrap()
    }

    pub async fn accept_initial_pages(
        &mut self,
        initial_pages: Vec<InitialPageImport>,
    ) -> Result<(), AcceptInitialPagesError> {
        self.req_send
            .call(PartitionRequest::AcceptInitialPages, initial_pages)
            .await
            .unwrap()
    }

    /// Builds the partition state blob for a `.vmrs` dump file.
    ///
    /// Stops VPs internally for a consistent snapshot and resumes them
    /// afterward. Returns the serialized partition state (VP registers
    /// as hypervisor save/restore chunks).
    #[cfg(feature = "dump")]
    pub async fn build_dump_partition_state(&mut self) -> anyhow::Result<Vec<u8>> {
        self.req_send
            .call(PartitionRequest::BuildDumpPartitionState, ())
            .await
            .unwrap()
    }
}

/// What `PartitionUnit::new` computes for the runner before its channel
/// exists: the runner's fields that are not its initial state.
#[cfg_attr(verus_keep_ghost, verus_verify)]
struct RunnerParts {
    partition: Box<dyn VmPartition>,
    vp_set: VpSet,
    halt_request_recv: Receiver<InternalHaltReason>,
    client_notify_send: mesh::Sender<HaltReason>,
    req_recv: Receiver<PartitionRequest>,
    topology: Topology,
    #[cfg(feature = "gdb")]
    debugger_state: debug::DebuggerState,
}

#[cfg_attr(verus_keep_ghost, verus_verify)]
impl PartitionUnitRunner {
    /// The runner, stopped, from its parts, taking its half of its FracGhost
    /// from `recv`, its channel's receiver.
    #[cfg_attr(verus_keep_ghost, verus_verify(external_body))]
    #[cfg_attr(verus_keep_ghost, verus_spec(runner =>
        requires
            old(recv).attached() matches Some(h) && h.valid(old(recv).constant(), 0.5real),
        ensures
            runner.wf(),
            runner.id() == old(recv).constant(),
            !runner.running(),
            final(recv).constant() == old(recv).constant(),
    ))]
    fn new(parts: RunnerParts, recv: &mut Receiver<StateRequest>) -> Self {
        let _ = recv;
        let RunnerParts {
            partition,
            vp_set,
            halt_request_recv,
            client_notify_send,
            req_recv,
            topology,
            #[cfg(feature = "gdb")]
            debugger_state,
        } = parts;
        Self {
            partition,
            vp_set,
            unit_started: false,
            vp_stop_count: 0,
            needs_reset: false,
            halt_reason: None,
            halt_request_recv,
            client_notify_send,
            req_recv,
            topology,
            initial_regs: None,
            #[cfg(guest_arch = "x86_64")]
            omit_saved_tsc: false,
            #[cfg(feature = "gdb")]
            debugger_state,
            // PROOF TODO: replace this placeholder (`assume_new()`) with the
            // real ghost: the half taken from `recv` (`take_receiver_ghost`, as
            // in `KeeperUnit::new`).
            #[cfg(verus_keep_ghost)]
            share: Tracked::assume_new(),
            // PROOF TODO: replace this placeholder (`assume_new()`) with the
            // real ghost: a new stop-token authority, with no token
            // outstanding, whose id is the request channel's constant.
            #[cfg(verus_keep_ghost)]
            stop_tokens: Tracked::assume_new(),
        }
    }
}

/// The partition unit's task: builds the runner from its parts once its
/// channel exists (taking its share from it), then runs it.
#[cfg_attr(verus_keep_ghost, verus_verify)]
#[cfg_attr(verus_keep_ghost, verus_spec(runner =>
    requires
        recv.attached() matches Some(h) && h.valid(recv.constant(), 0.5real),
    ensures
        runner.wf(),
        runner.id() == recv.constant(),
))]
async fn run_partition_unit(
    parts: RunnerParts,
    recv: Receiver<StateRequest>,
) -> PartitionUnitRunner {
    let mut recv = recv;
    let mut runner = PartitionUnitRunner::new(parts, &mut recv);
    runner.run(recv).await;
    runner
}

impl PartitionUnitRunner {
    /// Runs the VM partition, handling state change requests from `recv`.
    ///
    /// Verus: `recv` is the unit's channel; the runner stays well formed.
    #[cfg_attr(verus_keep_ghost, verus_verify(external_body))]
    #[cfg_attr(verus_keep_ghost, verus_spec(
        requires
            old(self).wf(),
            recv.constant() == old(self).id(),
        ensures
            final(self).wf(),
            final(self).id() == old(self).id(),
    ))]
    async fn run(&mut self, mut recv: Receiver<StateRequest>) {
        loop {
            enum Event {
                State(Option<StateRequest>),
                Halt(InternalHaltReason),
                Request(PartitionRequest),
                #[cfg(feature = "gdb")]
                Debug(vmm_core_defs::debug_rpc::DebugRequest),
            }

            #[cfg(feature = "gdb")]
            let debug = self.debugger_state.wait_rpc();
            #[cfg(not(feature = "gdb"))]
            let debug = std::future::pending();

            let event = futures::select! {  // merge semantics
                request = recv.next() => Event::State(request),
                request = self.halt_request_recv.select_next_some() => Event::Halt(request),
                request = self.req_recv.select_next_some() => Event::Request(request),
                request = debug.fuse() => {
                    #[cfg(feature = "gdb")]
                    {
                        Event::Debug(request)
                    }
                    #[cfg(not(feature = "gdb"))]
                    {
                        let _: std::convert::Infallible = request;
                        unreachable!()
                    }
                }
            };

            match event {
                Event::State(request) => {
                    if let Some(request) = request {
                        request.apply(self).await;
                    } else {
                        break;
                    }
                }
                Event::Halt(reason) => {
                    // Wait for the VPs to stop before reporting this anywhere.
                    // This is generally good behavior, but it is especially
                    // necessary because Self::clear_halt() will call
                    // VpSet::clear_halt(), which relies on the VPs being
                    // affirmatively stopped.
                    self.vp_set.stop().await;
                    self.handle_halt(reason).await;
                }
                Event::Request(request) => match request {
                    PartitionRequest::ClearHalt(rpc) => rpc.handle_sync(|()| self.clear_halt()),
                    PartitionRequest::SetInitialRegs(rpc) => {
                        rpc.handle(async |(vtl, state)| self.set_initial_regs(vtl, state).await)
                            .await
                    }
                    PartitionRequest::AcceptInitialPages(rpc) => {
                        rpc.handle(async |initial_pages| {
                            self.accept_initial_pages(initial_pages).await
                        })
                        .await
                    }
                    PartitionRequest::StopVps(rpc) => {
                        let ((), rpc) = mesh_rpc_verus::split::<_, _, ()>(rpc);
                        self.stop_vps().await;
                        // PROOF TODO: replace this placeholder (`assume_new()`) with the
                        // real ghost: a new stop token, from `stop_tokens.insert`.
                        mesh_rpc_verus::complete(rpc, (), Tracked::<()>::assume_new());
                    }
                    PartitionRequest::Snapshot(req) => self.handle_snapshot(req).await,
                    PartitionRequest::StartVps => {
                        self.resume_vps();
                    }
                    #[cfg(feature = "dump")]
                    PartitionRequest::BuildDumpPartitionState(rpc) => {
                        rpc.handle(async |()| self.build_dump_partition_state().await)
                            .await
                    }
                },
                #[cfg(feature = "gdb")]
                Event::Debug(request) => {
                    self.handle_gdb(request).await;
                }
            }
        }

        if self.unit_started {
            self.vp_set.stop().await;
        }
    }

    async fn handle_halt(&mut self, reason: InternalHaltReason) {
        match reason {
            InternalHaltReason::Halt(reason) => {
                // Only report the first halt request per boot so that the
                // client does not have to deal with multiple halt reasons
                // due to race conditions.
                if self.halt_reason.is_none() {
                    self.record_halt(reason);
                } else {
                    // Clear this specific halt.
                    self.vp_set.clear_halt();
                }
            }
            InternalHaltReason::ReplayMtrrs => {
                if let Some(initial_regs) = self.initial_regs.clone() {
                    if let Err(err) = self
                        .vp_set
                        .set_initial_regs(
                            Vtl::Vtl0,
                            initial_regs,
                            vp_set::RegistersToSet::MtrrsOnly,
                        )
                        .await
                    {
                        tracing::error!(
                            error = &err as &dyn std::error::Error,
                            "failed to replay mtrrs, guest may see inconsistent results"
                        );
                    }
                } else {
                    tracing::warn!("no initial mtrrs to replay");
                }
                self.vp_set.clear_halt();
                self.try_start();
            }
        }
    }

    fn record_halt(&mut self, reason: HaltReason) {
        assert!(self.halt_reason.is_none());
        self.halt_reason = Some(reason.clone());

        // Report the halt to the debugger.
        #[cfg(feature = "gdb")]
        let reported = self.debugger_state.report_halt_to_debugger(&reason);
        #[cfg(not(feature = "gdb"))]
        let reported = false;

        // If the debugger is not attached, then report the halt to the client.
        if !reported {
            self.client_notify_send.send(reason);
        }
    }

    /// Clears the halt and resumes the VPs if the partition is started. Returns
    /// `false` if VPs were not already halted.
    fn clear_halt(&mut self) -> bool {
        if self.halt_reason.is_some() {
            self.halt_reason = None;
            self.vp_set.clear_halt();
            self.try_start();
            true
        } else {
            false
        }
    }

    async fn set_initial_regs(
        &mut self,
        vtl: Vtl,
        state: Arc<InitialRegs>,
    ) -> Result<(), InitialRegError> {
        assert!(!self.unit_started || self.vp_stop_count > 0);

        // If this VM has been run before, then automatically scrub the target
        // VTL state.
        if self.needs_reset {
            self.partition
                .scrub_vtl(vtl)
                .map_err(InitialRegError::ScrubVtl)?;
            self.vp_set
                .scrub(vtl)
                .await
                .map_err(InitialRegError::ScrubVtl)?;
            self.needs_reset = false;
        }

        match self.partition.initial_vp_state_source() {
            InitialVpStateSource::Registers => {
                self.vp_set
                    .set_initial_regs(vtl, state.clone(), vp_set::RegistersToSet::All)
                    .await
                    .map_err(InitialRegError::RegisterSet)?;
            }
            InitialVpStateSource::ImportedContext => {}
        }

        self.initial_regs = Some(state);
        Ok(())
    }

    async fn accept_initial_pages(
        &mut self,
        initial_pages: Vec<InitialPageImport>,
    ) -> Result<(), AcceptInitialPagesError> {
        assert!(!self.unit_started);

        self.partition
            .accept_initial_pages(initial_pages)
            .map_err(AcceptInitialPagesError::Finalize)
    }

    fn try_start(&mut self) {
        if self.unit_started && self.halt_reason.is_none() && self.vp_stop_count == 0 {
            self.needs_reset = true;
            self.vp_set.start();
        }
    }

    async fn stop_vps(&mut self) {
        self.vp_set.stop().await;
        self.vp_stop_count += 1;
    }

    fn resume_vps(&mut self) {
        assert!(
            self.vp_stop_count > 0,
            "resume_vps called without matching stop"
        );
        self.vp_stop_count -= 1;
        self.try_start();
    }
}

#[cfg(feature = "dump")]
impl PartitionUnitRunner {
    /// Builds the partition state blob for a `.vmrs` dump file.
    ///
    /// Collects VP register state and assembles the chunk stream.
    async fn build_dump_partition_state(&mut self) -> anyhow::Result<Vec<u8>> {
        // Stop VPs for a consistent snapshot (and to ensure guest_os_id
        // doesn't block on backends that retrieve it from a VP).
        self.stop_vps().await;
        let result = self.build_dump_partition_state_inner().await;
        self.resume_vps();
        result
    }

    async fn build_dump_partition_state_inner(&mut self) -> anyhow::Result<Vec<u8>> {
        use hyperv_dump::PartitionStateBuilder;
        use hyperv_dump::ProcessorArch;

        #[cfg(guest_arch = "x86_64")]
        let arch = ProcessorArch::X64;
        #[cfg(guest_arch = "aarch64")]
        let arch = ProcessorArch::Aarch64;

        let mut builder = PartitionStateBuilder::new(arch);
        builder.set_os_id(self.partition.guest_os_id());

        let vp_count = self.topology.0.vp_count();
        for vp_idx in 0..vp_count {
            let vtl = Vtl::Vtl0;
            let vp_state = self
                .vp_set
                .get_dump_vp_state(VpIndex::new(vp_idx), vtl)
                .await
                .with_context(|| format!("failed to get state for VP {vp_idx}"))?;

            builder.add_vp(vp_idx, vec![(vtl, vp_state)]);
        }

        Ok(builder.finish())
    }
}

#[must_use = "when dropped, the VPs will be resumed"]
#[cfg_attr(verus_keep_ghost, verus_verify)]
pub struct StopGuard(
    mesh::Sender<PartitionRequest>,
    /// The stop token (Verus only): while the guard holds it, the VPs are
    /// stopped (the request protocol).
    #[cfg(verus_keep_ghost)]
    Tracked<vstd::resource::set::GhostSingleton<u64>>,
);

impl Drop for StopGuard {
    fn drop(&mut self) {
        // PROOF TODO: send the stop token back with `StartVps` (through
        // `mesh_rpc_verus::send_msg`, an executable change to approve).
        self.0.send(PartitionRequest::StartVps);
    }
}

#[cfg_attr(verus_keep_ghost, verus_verify)]
impl StateUnit for PartitionUnitRunner {
    #[cfg_attr(verus_keep_ghost, verus_verify(external_body))]
    #[cfg_attr(verus_keep_ghost, verus_spec(with Tracked(g): Tracked<&mut Option<FracGhost<UnitSnapshot>>>))]
    async fn start(&mut self) -> anyhow::Result<()> {
        self.unit_started = true;
        self.try_start();
        Ok(())
    }

    #[cfg_attr(verus_keep_ghost, verus_verify(external_body))]
    #[cfg_attr(verus_keep_ghost, verus_spec(with Tracked(g): Tracked<&mut Option<FracGhost<UnitSnapshot>>>))]
    async fn stop(&mut self) {
        self.vp_set.stop().await;
        self.unit_started = false;

        // Now that the VM is stopped, flush any guest-initiated
        // power state change that may have raced with this request.
        while let Ok(reason) = self.halt_request_recv.try_recv() {
            self.handle_halt(reason).await;
        }
    }

    #[cfg_attr(verus_keep_ghost, verus_verify(external_body))]
    #[cfg_attr(verus_keep_ghost, verus_spec(with Tracked(g): Tracked<&mut Option<FracGhost<UnitSnapshot>>>))]
    async fn reset(&mut self) -> anyhow::Result<()> {
        self.partition.reset()?;
        self.vp_set.reset().await?;
        self.clear_halt();
        self.needs_reset = false;
        Ok(())
    }

    #[cfg_attr(verus_keep_ghost, verus_verify(external_body))]
    #[cfg_attr(verus_keep_ghost, verus_spec(with Tracked(g): Tracked<&mut Option<FracGhost<UnitSnapshot>>>))]
    async fn save(&mut self) -> Result<Option<SavedStateBlob>, SaveError> {
        let state = self.save().await?;
        Ok(Some(SavedStateBlob::new(state)))
    }

    #[cfg_attr(verus_keep_ghost, verus_verify(external_body))]
    #[cfg_attr(verus_keep_ghost, verus_spec(with Tracked(g): Tracked<&mut Option<FracGhost<UnitSnapshot>>>))]
    async fn restore(&mut self, buffer: SavedStateBlob) -> Result<(), RestoreError> {
        self.needs_reset = true;
        self.restore(buffer.parse()?).await?;
        Ok(())
    }
}

mod save_restore {
    use super::PartitionUnitRunner;
    use virt::VpIndex;
    use vmcore::save_restore::RestoreError;
    use vmcore::save_restore::SaveError;

    mod state {
        use mesh::payload::Protobuf;
        use vmcore::save_restore::SavedStateBlob;
        use vmcore::save_restore::SavedStateRoot;

        #[derive(Protobuf, SavedStateRoot)]
        #[mesh(package = "partition")]
        #[cfg_attr(verus_keep_ghost, vstd::prelude::verus_verify)]
        pub struct Partition {
            #[mesh(1)]
            pub(super) partition: SavedStateBlob,
            #[mesh(2)]
            pub(super) vps: Vec<Vp>,
            #[mesh(3)]
            pub(super) halt_reason: Option<Vec<u8>>,
        }

        #[derive(Protobuf)]
        #[mesh(package = "partition")]
        #[cfg_attr(verus_keep_ghost, vstd::prelude::verus_verify)]
        pub struct Vp {
            #[mesh(1)]
            pub vp_index: u32,
            #[mesh(2)]
            pub data: SavedStateBlob,
        }
    }

    #[cfg(verus_keep_ghost)]
    use vstd::prelude::*;

    // Verus: the view of the partition unit's saved state.
    #[cfg(verus_keep_ghost)]
    vstd::prelude::verus! {

    /// The view of the saved state: the partition's blob, the VPs' (index,
    /// blob) pairs (a set: the order of the list carries no meaning, and
    /// restore accepts any order), and the encoded halt reason.
    impl View for state::Partition {
        type V = super::spec::PartitionSavedView;

        closed spec fn view(&self) -> super::spec::PartitionSavedView {
            super::spec::PartitionSavedView {
                partition: self.partition@,
                vps: self.vps@.map_values(|vp: state::Vp| (vp.vp_index, vp.data@)).to_set(),
                halt_reason: match self.halt_reason {
                    Some(bytes) => Some(bytes@),
                    None => None,
                },
            }
        }
    }

    impl super::spec::PartitionSavedView {
        /// The blob of a saved state with this view.
        pub(crate) open spec fn blob(self) -> vmcore::save_restore::spec::SavedStateBlobView {
            vmcore::save_restore::spec::SavedStateBlobView::of_view::<state::Partition>(self)
        }
    }

    } // verus!

    impl PartitionUnitRunner {
        pub async fn save(&mut self) -> Result<state::Partition, SaveError> {
            let partition = self.partition.save()?;
            let vps = self.vp_set.save().await?;
            let vps = vps
                .into_iter()
                .map(|(vp_index, data)| state::Vp {
                    vp_index: vp_index.index(),
                    data,
                })
                .collect();
            let halt_reason = self.halt_reason.clone().map(mesh::payload::encode);

            Ok(state::Partition {
                partition,
                vps,
                halt_reason,
            })
        }

        pub async fn restore(&mut self, state: state::Partition) -> Result<(), RestoreError> {
            let state::Partition {
                partition,
                vps,
                halt_reason,
            } = state;
            let halt_reason = halt_reason
                .as_deref()
                .map(mesh::payload::decode)
                .transpose()?;
            if self.halt_reason.is_some() {
                return Err(RestoreError::Other(anyhow::anyhow!(
                    "cannot restore over an existing partition halt"
                )));
            }
            self.vp_set.validate_restore_halt()?;
            self.partition.restore(partition)?;
            let vps = vps
                .into_iter()
                .map(|state::Vp { vp_index, data }| {
                    #[cfg(guest_arch = "x86_64")]
                    let data = if self.omit_saved_tsc {
                        super::snapshot::without_saved_tsc(data)?
                    } else {
                        data
                    };
                    Ok((VpIndex::new(vp_index), data))
                })
                .collect::<Result<Vec<_>, RestoreError>>()?;
            self.vp_set.restore(vps).await?;
            self.vp_set.restore_halt(halt_reason.is_some())?;
            if let Some(reason) = halt_reason {
                self.record_halt(reason);
            }
            Ok(())
        }
    }

    #[cfg(test)]
    mod tests {
        use super::state;
        use crate::partition_unit::Halt;
        use crate::partition_unit::PartitionUnitRunner;
        use crate::partition_unit::VmPartition;
        use crate::partition_unit::VpSet;
        use async_trait::async_trait;
        use hvdef::Vtl;
        use inspect::InspectMut;
        use pal_async::local::block_on;
        use state_unit::StateUnit;
        use std::sync::Arc;
        use test_with_tracing::test;
        use virt::InitialPageImport;
        use virt::InitialVpStateSource;
        use vm_topology::processor::ProcessorTopology;
        use vm_topology::processor::TopologyBuilder;
        use vmcore::save_restore::NoSavedState;
        use vmcore::save_restore::RestoreError;
        use vmcore::save_restore::SaveError;
        use vmcore::save_restore::SaveRestore;
        use vmcore::save_restore::SavedStateBlob;
        use vmm_core_defs::HaltReason;

        /// A partition without state of its own.
        struct TestPartition;

        impl InspectMut for TestPartition {
            fn inspect_mut(&mut self, req: inspect::Request<'_>) {
                req.ignore();
            }
        }

        // Placeholder view (save/restore not yet specified).
        vmcore::placeholder_view!(pub ExTestPartition, TestPartition);

        impl SaveRestore for TestPartition {
            type SavedState = NoSavedState;

            fn save(&mut self) -> Result<NoSavedState, SaveError> {
                Ok(NoSavedState)
            }

            fn restore(&mut self, NoSavedState: NoSavedState) -> Result<(), RestoreError> {
                Ok(())
            }
        }

        #[async_trait]
        impl VmPartition for TestPartition {
            fn initial_vp_state_source(&self) -> InitialVpStateSource {
                InitialVpStateSource::Registers
            }

            fn reset(&mut self) -> anyhow::Result<()> {
                Ok(())
            }

            fn scrub_vtl(&mut self, _vtl: Vtl) -> anyhow::Result<()> {
                Ok(())
            }

            fn accept_initial_pages(
                &mut self,
                _pages: Vec<InitialPageImport>,
            ) -> anyhow::Result<()> {
                Ok(())
            }
        }

        fn test_topology() -> ProcessorTopology {
            #[cfg(guest_arch = "x86_64")]
            let builder = TopologyBuilder::new_x86();
            #[cfg(guest_arch = "aarch64")]
            let builder = {
                use vm_topology::processor::aarch64::Aarch64PlatformConfig;
                use vm_topology::processor::aarch64::GicMsiController;
                use vm_topology::processor::aarch64::GicVersion;

                TopologyBuilder::new_aarch64(Aarch64PlatformConfig {
                    gic_distributor_base: 0xffff0000,
                    gic_version: GicVersion::V3 {
                        redistributors_base: 0xefff0000,
                    },
                    gic_msi: GicMsiController::None,
                    pmu_gsiv: None,
                    virt_timer_ppi: 20,
                    gic_nr_irqs: 992,
                })
            };
            builder.build(1).unwrap()
        }

        /// Returns a stopped runner without VPs, the halt latch of its VPs, and
        /// the receiver of the halts that it reports to its client.
        fn test_runner() -> (PartitionUnitRunner, Arc<Halt>, mesh::Receiver<HaltReason>) {
            let (halt, halt_request_recv) = Halt::new();
            let halt = Arc::new(halt);
            let (client_notify_send, client_notify_recv) = mesh::channel();
            let runner = PartitionUnitRunner {
                partition: Box::new(TestPartition),
                vp_set: VpSet::new([None, None, None], halt.clone(), 0),
                unit_started: false,
                vp_stop_count: 0,
                needs_reset: false,
                halt_reason: None,
                halt_request_recv: halt_request_recv.0,
                client_notify_send,
                req_recv: mesh::channel().1,
                topology: super::super::Topology(test_topology()),
                initial_regs: None,
                #[cfg(guest_arch = "x86_64")]
                omit_saved_tsc: false,
                #[cfg(feature = "gdb")]
                debugger_state: crate::partition_unit::debug::DebuggerState::new(
                    guestmem::GuestMemory::empty(),
                    None,
                ),
            };
            (runner, halt, client_notify_recv)
        }

        /// Raises `reason` as a VP does and stops `runner`, which records the
        /// halt and reports it to `client`.
        fn halt(
            runner: &mut PartitionUnitRunner,
            halt: &Halt,
            client: &mut mesh::Receiver<HaltReason>,
            reason: HaltReason,
        ) {
            halt.halt(reason.clone());
            block_on(StateUnit::stop(runner));
            assert_eq!(client.try_recv().unwrap(), reason);
        }

        fn save(runner: &mut PartitionUnitRunner) -> SavedStateBlob {
            block_on(StateUnit::save(runner)).unwrap().unwrap()
        }

        fn encoded_partition(halt_reason: Option<HaltReason>) -> SavedStateBlob {
            SavedStateBlob::new(state::Partition {
                partition: SavedStateBlob::new(NoSavedState),
                vps: Vec::new(),
                halt_reason: halt_reason.map(mesh::payload::encode),
            })
        }

        #[test]
        fn partition_state_preserves_halt_reason() {
            let decoded: state::Partition = encoded_partition(Some(HaltReason::PowerOff))
                .parse()
                .unwrap();

            assert_eq!(
                decoded
                    .halt_reason
                    .map(|reason| mesh::payload::decode(&reason).unwrap()),
                Some(HaltReason::PowerOff)
            );
        }

        #[test]
        fn partition_state_without_halt_reason_is_not_halted() {
            let decoded: state::Partition = encoded_partition(None).parse().unwrap();

            assert!(decoded.halt_reason.is_none());
        }

        #[test]
        fn restore_latches_and_reports_a_saved_halt() {
            let (mut source, source_halt, mut source_client) = test_runner();
            halt(
                &mut source,
                &source_halt,
                &mut source_client,
                HaltReason::PowerOff,
            );
            let saved = save(&mut source);

            let (mut target, _, mut target_client) = test_runner();
            block_on(StateUnit::restore(&mut target, saved)).unwrap();
            assert_eq!(target.halt_reason, Some(HaltReason::PowerOff));
            assert_eq!(target_client.try_recv().unwrap(), HaltReason::PowerOff);
            assert!(target.vp_set.validate_restore_halt().is_err());

            // Clearing the halt releases the one VP halt that the restore
            // latched.
            assert!(target.clear_halt());
            target.vp_set.validate_restore_halt().unwrap();
        }

        #[test]
        fn restore_without_a_saved_halt_reports_none() {
            let (mut source, _, _) = test_runner();
            let saved = save(&mut source);

            let (mut target, _, mut target_client) = test_runner();
            block_on(StateUnit::restore(&mut target, saved)).unwrap();
            assert!(target.halt_reason.is_none());
            assert!(target_client.try_recv().is_err());
            target.vp_set.validate_restore_halt().unwrap();
        }

        #[test]
        fn restore_refuses_a_halted_destination() {
            let (mut source, source_halt, mut source_client) = test_runner();
            halt(
                &mut source,
                &source_halt,
                &mut source_client,
                HaltReason::PowerOff,
            );
            let saved = save(&mut source);

            // A VP halt that the runner has not recorded yet refuses the
            // restore...
            let (mut target, target_halt, mut target_client) = test_runner();
            target_halt.halt(HaltReason::Reset);
            let err = block_on(StateUnit::restore(&mut target, saved.clone())).unwrap_err();
            assert_eq!(err.to_string(), "cannot restore over an existing VP halt");

            // ...and so does a recorded one, which the restore leaves alone.
            block_on(StateUnit::stop(&mut target));
            assert_eq!(target_client.try_recv().unwrap(), HaltReason::Reset);
            let err = block_on(StateUnit::restore(&mut target, saved)).unwrap_err();
            assert_eq!(
                err.to_string(),
                "cannot restore over an existing partition halt"
            );
            assert_eq!(target.halt_reason, Some(HaltReason::Reset));
            assert!(target_client.try_recv().is_err());
        }
    }
}
