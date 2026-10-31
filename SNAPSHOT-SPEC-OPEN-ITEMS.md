# OpenVMM snapshot spec: open items

What is still open, as of code change 96 (2026-10-31), grouped by who or
what each item waits for. For the terms used here, see
`SNAPSHOT-SPEC-OVERVIEW.md`; for the details of each item, the sections of
`SNAPSHOT-SPEC-DESIGN.md` named in parentheses.

## 1. Specification still to write

Properties that belong in an entrance contract and are not there yet.

1. **What the snapshot means for the guest.** Today the spec says the
   saved state is each unit's state **once stopped** (at the quiesce).
   Between the guest's request (the snapshot boundary, when only the
   vCPUs are stopped) and the quiesce, the devices keep running: timers
   fire, virtio devices may process queued requests. The spec should
   eventually say: the stopped state is the state at the guest's
   request, with the requests already queued applied, and nothing more
   (the user, 2026-10-31). Related, and also unstated: host input is
   gated during the boundary, and releasing the boundary completes the
   guest's request. Not easy: it needs a model of the guest's requests
   and of what devices do while only the vCPUs are stopped.

## 2. TODOs for the proof agent

1. **Proofs.** Many functions on the snapshot path are tagged
   `external_body`. They are of two kinds, told apart by a comment at
   each:
   - **To be left alone** (marked): the trusted base, our wrappers of
     external code such as mesh, the locks, and the hypervisor (in
     `*_t.rs` files, or marked `TRUSTED`); bodies Verus cannot handle
     yet (marked `PROOF DEFERRED`, section 3); and code out of scope
     (marked `OUT OF SCOPE`).
   - **To be proved** (most of them; no such marker): the proof agent
     should write the proof and remove the `external_body` tag.
     Inside these bodies, `PROOF TODO:` comments mark ghost placeholders
     to replace.

   `scripts/verus-spec-audit.py --list` lists every such function by
   kind. Other functions may need proofs too, beyond those listed (e.g.
   the functions that proved code calls); the proof agent will find them
   as it goes.

2. **Specs for non-entrance functions, and others.** So far we have
   specified only the entrance functions (e.g.
   `LoadedVm::quiesce_for_snapshot`, `save`, `restore_from_snapshot`,
   `resume`), the RPC protocols on the snapshot path, the key functions
   for device units, etc. To finish the proof, more will be needed:
   - contracts for many non-entrance functions (e.g. the vCPUs' device
     accesses, `StateUnits`' stop and start loops);
   - possibly more fractional ghosts or lock invariants than the spec has
     today;
   - important internal properties, such as: the units stop, start, and
     restore in dependency order (the overview, 2.3), which is why a vCPU
     never reaches a stopped device; and the facts about our code that
     the trusted doc, Part 3, assumes until proved (e.g. "Stopped devices
     do not change").

## 3. Waiting for Verus (not to be done for now)

Gap G5: Verus cannot yet state contracts on methods of `async` trait
objects (`async_trait`) or on futures. Not to be done for now (the user,
2026-10-30). Waiting on it ("The trust audit"):

1. **The guest's writes while its vCPUs run** (part of gap G3, "The
   partition's state"): the partition's VM state, and the same "guest
   ran" refresh for virtio-MMIO's interrupt status (`ghost_status`).
2. **The vCPU layer** (gap G4): `WrappedVp`'s `SaveRestore`, `BoundVp`'s
   `ProtobufSaveRestore`, the backends' VP-state access (`AccessVpState`,
   marked PROOF DEFERRED).
3. **The partition runner's other events** (decided 2026-10-28, the
   user). Besides state requests, `PartitionUnitRunner::run` handles halt
   events and partition requests (`ClearHalt`, `SetInitialRegs`,
   `AcceptInitialPages`, ...), some through `VmPartition`'s
   `async_trait` methods (`reset`, `scrub_vtl`, `accept_initial_pages`).
   Those that change the saved state (the halt reason, vCPU registers)
   must keep the unit's ghost in step: allowed while running or before
   the first stop or restore (the ghost is `Unknown`); while stopped after
   a snapshot is recorded, only if the change is reflected in the ghost,
   which the stopped unit cannot do alone. To check per request in the
   proof, or in `PartitionRequest`'s protocol (written for the snapshot
   requests, code change 87; these requests are forbidden there for
   now). The runner's `stop` also handles pending halt requests after
   stopping the vCPUs, so the halt reason is settled at `stop`.
4. **Two deferred proofs**: the erasure point
   `From<T> for ResolvedChipsetDevice` (gap G1's one deferred fact), and
   `ArcMutexChipsetDeviceUnit::next_event` (`select!`).

## 4. Reviews

1. **Soundness review** of the mesh RPC wrapper's reply-ghost type
   argument (`support/mesh/mesh_rpc_verus/src/mesh_rpc_t.rs`; the trusted
   doc, Part 1, "Reply ghost type").

## 5. Later layers (out of scope for now)

1. **The controller** (`VmController::handle_guest_snapshot_request`;
   "Roadmap", item 3). Its channel's protocol is written (code changes
   89, 90, 96). A first contract would be about the snapshot's outcome:
   after a committed snapshot the controller returns `Terminate` and never
   gives the units' halves back, so the source VM never runs again; it
   returns `Continue` only after the worker confirmed a rollback or a
   release. Harder: that the snapshot files hold exactly the saved state
   (it needs trusted specs for writing files).
2. **`LoadedVm::reset`** (and `VmRpc::Reset`): a reboot, not part of
   snapshot and restore; left for after the proof of save and restore
   (the user, 2026-10-31). ("Roadmap", step 3.)
3. **The inner virtio devices' state** (network, console, file system,
   disks), beyond what their transport saves.
4. **Guest time**: the content of the time capture, and the clock
   adjustment at restore.
5. **virtio-MMIO's legacy interrupt modes** (not used by the microVM,
   which uses the shared-status mode: "virtio-MMIO's interrupt status:
   the guest-memory boundary"): the interrupt-state lock protocol. In the
   legacy modes the status is `InterruptState.status`, behind
   `Arc<parking_lot::Mutex<InterruptState>>`, written by the transport,
   the device task's interrupt closures, and (legacy with accelerated
   acknowledgements, on Linux) the acknowledgement worker. To verify it:
   a lock with an invariant (an executable type change, as for
   `StateUnits`, code change 9) and a fractional ghost for the status
   (the lock's half and the transport's, the transport's half in the
   lock while the device runs), with ghost state through the interrupt
   closures. Also to check there: a guest acknowledgement written just
   before `stop` may be handled by the acknowledgement worker after
   `stop`, changing the status of a stopped device.

## 6. Documentation

   **The rest of the document split**: a contracts reference 
   (`SNAPSHOT-SPEC-CONTRACTS.md`), a conventions
   document (`SNAPSHOT-SPEC-CONVENTIONS.md`), and the design doc renamed
   as the decision log, with its executable-changes table moved to
   `SNAPSHOT-SPEC-CODE-CHANGES.md`.
