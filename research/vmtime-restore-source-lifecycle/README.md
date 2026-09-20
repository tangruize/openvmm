# Initial restore-source lifecycle

## Scope and unresolved bindings at intake

This diagnostic concerns only the source built in
`InitializedVm::new_with_hypervisor` and accesses sharing its `TimerState`.
It does not prove `LoadedVm::restore_snapshot_state` or cover separately built
or remotely transferred sources. Production code, frozen specifications/TCB,
existing dirty proof work, and the unapplied Duration request are not edited.

Accepted input: `research/vmtime-reset-installation/` and reviewed handoff
`0f9ad90da43f`. Its successful-secondary installation semantics are reused.
Its custom scheduler counterexample is not a production-lifecycle
counterexample. Its failed pool-drop experiment and completed tests are not
rerun.

Before any new experiment, the following bindings require source evidence:

- The initial source and primary keeper must actually select the same
  `ThreadDriverBackend` default device pool, not a caller-supplied scheduler.
- The registration sender must remain in the primary until the secondary
  really disconnects; the secondary's pending future must retain a wake path,
  not merely share an `Arc<TimerState>` with an accessor.
- Detachment must not cancel the secondary; no reachable cancellation handle
  or ordinary task-completion branch may remove it during the held restore.
- Queue closure and pool/thread teardown must be distinguished from a queued
  or sleeping task that is never scheduled. A strong scheduler reference alone
  does not prove that the pool's receiver remains alive.
- The selected `async-task` panic policy and the application's panic profile
  must distinguish cancellation of one task from termination of the device
  thread or process.
- Normal restore return must be connected to the retained state-unit handle,
  keeper task, and successful secondary reply, without assuming that ignored
  secondary errors were successes.

The frozen manifests parse as JSON and match the specified TOP/TCB entries.
The native call-structure reader was attempted once and cannot read the
absent `.verus_agent/proof_state.json`. No graph or checker repair is attempted;
the connection evidence below must come from current implementation source.

## Result: source-grounded exclusion for the selected construction

**Diagnostic, not a Verus proof.** For this initial source on the concrete
Worker construction path with the checked-in dev/release panic policy, ordinary
completion, cancellation, and queue teardown do not supply a path to an
acknowledged reset that leaves a shared accessor at the old stopped value.
The enforcing roots are the retained driver and keeper,
the local request endpoints, and the secondary's queued/running/receive-waker
ownership. Accessor `Arc` ownership is NOT the task-liveness argument.

Here successful restore means `Ok(())`, at return before any subsequent control
transition or VM teardown. A normal `Err` can occur before any reset
(`dispatch.rs:3834-3865`); it promises no restored clock, and is not a
counterexample to the frozen successful-result clause. This diagnostic does
not establish that the decoded saved state actually selects the needed reset.

### Concrete construction, roots, and completion path

Paths below are relative to the repository. Dependency citations refer to the
installed primary sources under
`/home/ruize/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/`.
`Cargo.lock` selects `async-task 4.7.1`, `async-channel 2.5.0`, and
`futures-concurrency 7.6.3`; no dependency was installed or replaced.

| Binding | Enforcing implementation |
| --- | --- |
| Actual driver, not an arbitrary `Spawn` | `openvmm/openvmm_core/src/worker/dispatch.rs:198-199,378-399,463-473`: both Worker construction routes use `new_device_thread` and `ThreadDriverBackend`. `dispatch.rs:1067-1075` and `openvmm/openvmm_core/src/hypervisor_backend.rs:72-85` forward that driver to `new_with_hypervisor`. |
| Primary and initial secondary share that pool | `dispatch.rs:1102-1107`; `vm/vmcore/src/vm_task.rs:42-45,60-65,203-208,327-330,418-446,466-469`: `simple()` has no target VP, so both tasks use clones of the default driver, not the dedicated-thread branch. |
| Selected observer identity | `dispatch.rs:1191,2123` supplies the initial source to partition and chipset construction. `vm/vmcore/src/vmtime.rs:567-599,774-805` initializes the state before publication; source clones/accesses share its `Arc<RwLock<TimerState>>`. Calling `builder().build(...)` instead creates a different state and is outside this result. |
| Independent scheduler root through restore | `dispatch.rs:3261-3283` retains `inner.driver_source` and `_vmtime`. The driver backend retains its default `IoDriver`; `support/pal/pal_async/src/io_pool.rs:127-130` returns its scheduler. Thus queue-sender retention need not be inferred circularly from the secondary itself. |
| Keeper and unit are not removed during this call | `dispatch.rs:1831-1842,3282,3331-3333,3862-3873`; `vmm_core/state_unit/src/lib.rs:314-337,1367-1389,1394-1419`: `_vmtime` owns its registration handle and task. Removing it consumes/drops that handle; the borrowed restore path does neither. Its future owns the keeper, whose `_task` owns the primary task (`vmtime.rs:460-478`). |
| Primary cannot normally finish or prune a healthy initial secondary | `vmtime.rs:630-653,700-711`: the primary loops over its retained request/registration receivers, and only prunes senders reporting closed. The initial reset sender stays in `keepers`. Its receiver is private to the initial secondary; neither endpoint is exported/serialized on this path. |
| Local closure is not a spontaneous transport failure | `support/mesh/mesh_channel_core/src/mpsc.rs:58-65,164-202,454-467,488-499,549-586,664-675`: creation is local; send moves the typed request into the local queue; `is_closed` tests receiver destruction; empty receive closes when no sender remains. Remote error branches exist but are not the initial reset channel. |
| Secondary cannot normally finish while that sender remains | `vmtime.rs:736-771`: the timer branch always returns `Pending`; the only loop exit is receive `None`. Start/Stop/Reset/Inspect continue the loop. Reset does not replace the source, task, or channel. |
| Detachment is not cancellation, but requires task references | `pal_async/src/task.rs:295-315` (under `support/pal/`) schedules before returning the task; `async-task-4.7.1/src/task.rs:87-91,231-310,440-444` distinguishes detach from cancel-on-drop. Its last-reference closure branch is real; detachment alone is insufficient evidence. |
| The missing task reference is supplied concretely | Before first poll the `Runnable` is queued; during poll it is owned by the executor. When the secondary returns `Pending`, the local receive has cloned its task waker (`mpsc.rs:549-560`). `futures-concurrency-7.6.3/src/future/race/tuple.rs:66-87`, `utils/tuple.rs:18-29`, and `utils/indexer.rs:17-40` poll both pending branches with the same context; the permanently-pending timer branch cannot bypass receive polling. `async-task-4.7.1/src/raw.rs:382-430,612-670` retains/reschedules these references. Sending takes and wakes the receive waker, rather than dropping the only reference without scheduling (`mpsc.rs:176-178`). |
| Pool receiver is not owned by the caller or accessor | `io_pool.rs:82-93,117-124` moves the pool into the spawned thread, which runs the task queue to completion. `pal_async/src/task.rs:146-195` keeps the receiver private and drains it by running, not discarding, each runnable. The Worker joins the device thread only after VM execution ends (`dispatch.rs:493-505`); a `JoinHandle` is not a task-cancellation handle. |
| Queue closure requires an actual lifetime event | `task.rs:165-183` uses an unbounded queue. `async-channel-2.5.0/src/lib.rs:179-198,230-244,553-560,604-613` closes on last sender/receiver drop (or explicit close, not exposed by this scheduler). The live driver retains a sender; the thread retains the receiver. `Scheduler::schedule` DOES discard a failed send, and `async-task-4.7.1/src/runnable.rs:893-932` then cancels the runnable: the construction excludes that closed-queue premise, not the cancellation behavior. |
| Backends do not return merely because tasks are idle | Linux selects epoll (`pal_async/src/unix/mod.rs:17-23`); `unix/epoll.rs:179-220` exits on the supplied future's `Ready`. The equivalent completion branch is `windows/iocp.rs:159-178` and `unix/kqueue.rs:164-190`. No fairness or bounded response time is inferred. |
| Successful reset reply follows installation | `vmtime.rs:700-711,756-759,209-220` and `support/mesh/mesh_channel/src/rpc.rs:65-81`: the secondary mutates under the write lock, drops the guard, then sends its reply. The primary awaits every reply before sending its own. A local oneshot distinguishes sent data from closure (`mesh_channel_core/src/oneshot.rs:166-223,388-437`); failed replies remain errors, even though the primary discards them. |
| Successful outer completion is not a state-unit failure fallback | `vmtime.rs:504-512` unwraps the primary response; `vmm_core/src/vmtime_unit.rs:43-52` awaits restore/advance; `state_unit/src/lib.rs:1048-1069,1092-1185` awaits unit responses and panics on channel loss for a still-registered unit. The exception for a concurrently removed unit cannot be justified for the retained `_vmtime` handle. |

### Reachability classification

| Event while an accessor survives | Can it explain old-value success on the selected path? |
| --- | --- |
| Secondary completes normally | No during the retained restore interval: that requires request-channel termination, whose live local sender/receiver roots are described above. It can complete after keeper/VM teardown; an accessor alone does not prevent that. |
| Explicit secondary cancellation | No exposed `Task`/cancellation handle survives the immediate detach. Dropping a source clone or accessor removes neither the task nor its channel. Dropping an actual queued runnable WOULD cancel it, but the production queue does not deliberately do that. |
| Detached task loses all wake references | Not at a pending receive on this path: the local queue retains the task waker. This discharges the source-level gap in an argument based only on detach or on `Arc<TimerState>`. Formal reference-count/waker reasoning remains owed. |
| Scheduler queue closes | Not by ordinary caller/accessor drop during restore: the retained driver keeps its sender and the spawned pool owns its receiver. Task references alone would NOT exclude pool destruction. No reachable ordinary shutdown operation on this receiver was found. |
| Secondary or executor panics | Configured dev/release profiles use `panic = 'abort'` (`Cargo.toml:681-692`), so there is no continuing successful restore. Also, `async-task` defaults to `propagate_panic: false` (`runnable.rs:184-189`, `raw.rs:538-560`); `pal_async` does not change it or catch around `task.run`. In an unwind build a task panic unwinds the device executor, not just an isolated secondary. The primary and vmtime unit are on that same executor; pending work can hang or lose its outer channel, not obtain a new successful reset acknowledgement. This is not a proof for arbitrary alternate panic-catching executors/build arrangements. |
| Restore future is cancelled | That invocation does not return `Ok` or `Err`. A dispatched reset can still complete; cancellation does not retract the queued request. If the VM itself is then dropped, the ownership interval ends and stale surviving accesses are possible, but not a successful return of the cancelled invocation. |
| Task is live but never scheduled, or cannot acquire the state lock | Reset remains pending for that participant. Neither scheduling starvation nor lock contention is an error reply or a successful reply. Fairness is unnecessary for this conditional-on-return fact. |
| Teardown after successful return | Task/channel loss can then coexist with a surviving accessor, but does not undo the already installed stopped value. After a later Start/Stop/Reset no unchanged-value claim is made. Teardown following an early `Err` may leave the original value; the successful TOP clause does not apply. |

This is not an empirical claim based on a bounded passing test. No new runtime
experiment was needed or run. The accepted custom-scheduler counterexample
remains valid for the unconstrained API, but its deliberate `Runnable` discard
does not instantiate this pool lifecycle. The earlier failed pool-drop test
remains superseded: receiver drop closes rather than drains the async queue;
it is not evidence that a secondary's future was destroyed. In addition, the
production caller does not own that pool to drop it during restore.

### Intended intermediate fact and remaining proof obligations

The source-supported, **unproved** lifecycle fact is:

> On the configured production path, from successful publication of the initial
> source through the retained restore operation, its registration is not
> pruned and its local reset receiver remains owned by the same uncancelled
> secondary on the device pool. This is ownership/availability, not eventual
> or timely scheduling. For a reset that returns successfully in this interval,
> that participant's reply is a successful handler reply, not a channel error.

Use this fact with the accepted successful-secondary reset semantics, not
instead of them: for the actual reset argument `t`, the shared timer has
`time = Stopped(t)`, `last = t`, `next = None`; a sharing accessor's `now()` is
`t` until a subsequent transition. Access-specific timeout requests are not
erased. This is the installation/observation part of
`restored_virtual_time(...).vm_time_100ns`, not a definition or proof of
`loaded_vm_representation`. No frozen TOP precondition is proposed.

The separate proof problem must establish the initial registration and local
endpoint identities, preserve the ownership relation across every real
poll/await/receive/wake/drop, rule out pruning/error of this participant, and
compose the actual `reset_to`, primary/secondary handlers, RPC, `run_vmtime`,
and `StateUnits::run_op` bodies. Rust ownership, Arc/RwLock visibility,
async-task reference transitions, local mesh/oneshot transport, queue
ownership, executor/thread and panic semantics still need admitted, sound
interfaces and the required project/library body proofs. Source inspection
does not sanction any of these as new BOTTOM-level TCB.

Other-source coverage is still open: passing the source to partition/chipset
does not prove all guest-visible clocks share this state; builder-created and
remote sources need their own relation. Duration-to-decoded-downtime
correspondence, correct saved-state selection, elapsed-time correspondence,
asynchronous body admission/proofs, and all four bridges remain unresolved:
`decoded_restore_request_view`, `decoded_load_restore_request_view`,
`initialized_vm_representation`, `loaded_vm_representation`
(`dispatch.proof.rs:28-57`). The TOP `external_body` and existing
InitializedVm/LoadedVmInner declaration cuts remain. No marker or contract
was added, removed, weakened, or discharged. No installed invariant, freeze
request, constructor-as-function repair, or checker repair is claimed.

## Evidence freshness and check status

`validation.log` records one successful current input-equivalence check:
2,559 source/manifest/verification inputs match the accepted workspace,
including frozen manifests and existing dirty scalar proof work. The only
normalization removes the known isolated test-target suffix from that
workspace's `vmm_core/Cargo.toml`; no executable or specification is normalized.
The live root lockfile and committed unapplied Duration request are unchanged.
Inputs compared include tracked Rust files and Cargo manifests, `.cargo/`,
`verification/`, Makefile, rust-toolchain, and the untracked scalar proof file.

Per the unchanged-source/evidence-reuse rule, the four prescribed Verus checks
are **reused accepted results, not newly executed commands**. Full logs remain
in `research/vmtime-reset-installation/{name}.complete.log`; its original
`checks.py` records the exact authoritative-Python module invocations.

| Command module and root arguments | Reused result |
| --- | --- |
| `argus_verus.tools.checks.make_verify --crate-root .` | Exit 0; **0 verified / 0 errors**, 2.580 s inner elapsed. The unchanged wrapper selects the scaffolded TOP with `--no-lifetime`: no lifecycle or full-crate body proof. |
| `argus_verus.tools.checks.boundary --crate-root . --baseline-dir .verus_agent check` | Exit 1; existing TCB-manifest provenance rejection; summary reports 7 temporary locations, 0 permanent violations, 0 assumptions; 1.035 s. This setup/guard issue is not repaired or reclassified as success. |
| `argus_verus.tools.checks.spec_drift --crate-root . --baseline-dir .verus_agent` | Exit 0; no frozen specification drift; 1.191 s. |
| `argus_verus.tools.checks.exec_drift --crate-root . --baseline-dir .verus_agent` | Exit 0; no executable drift; 5.248 s. |

The current native call-structure attempt separately failed because its
maintained snapshot is missing. This diagnostic adds no source annotations
or verification experiment, so there is no experimental source to restore,
no lifetime-disabled result promoted to proof, and no rlimit change.
The Wiki is read-only for this mission's write boundary.
