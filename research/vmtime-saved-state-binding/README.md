# Omitted vmtime: saved-input binding diagnostic

## Classification

**Source-supported input-based interpretation, with unproved decoding and
lifecycle obligations; not a TOP proof or a demonstrated frozen-boundary
conflict.** On the current construction/entry path, omission is compatible
with interpreting the decoded saved virtual time as zero. A valid supplied
`"vmtime"` blob instead supplies the keeper ticks decoded from that blob.
Neither interpretation is installed in a bridge or sanctioned by this work.

Unrestricted `StateUnits` restore cannot determine the clock from saved input
alone: omission retains the pre-existing stopped value. However, the real
TOP's sole caller constructs a fresh keeper at zero and enters restore before
starting it. The nonzero API observation below is **not** a reachable TOP
counterexample. The clock after optional advancement depends on the retained
or supplied ticks and the time policy; 1234 ns advances these samples by
12 ticks.

The unresolved binding questions were recorded in `../GROUND_TRUTH.md`
before experimentation. Production, frozen inputs, existing proof work, and
pending freeze requests were not edited. No new freeze request is warranted
by this evidence.

## Frozen obligation and actual path

Paths and line numbers refer to the live repository at this diagnostic.

| Binding | Source evidence |
| --- | --- |
| Required result | `openvmm/openvmm_core/src/worker/dispatch.spec.rs:178-195`: `restored_virtual_time` uses `request.saved_state.virtual_time.vm_time_100ns`, with optional downtime adjustment, not the initial VM clock. Elapsed time is zero without adjustment and decoded downtime otherwise. |
| Available decoding inputs | `dispatch.proof.rs:28-32` in that directory: `decoded_restore_request_view` receives only `SavedState`, restore-time policy, and selected VP count. Neither keeper pre-state nor `LoadedVm` is an input. |
| What the frozen precondition does not say | `dispatch.spec.rs:254-284`: `valid_for_loaded_vm` requires `PreparingRestore`, VP identity/selection and saved-state compatibility; it does not require initial clock zero or a present vmtime blob. Saved elapsed time must be zero. Thus the precondition alone cannot justify a zero fallback. |
| Blob selection | `vmm_core/state_unit/src/lib.rs:983-1044`: lookup is by registered name; unknown or duplicate names cause an error. There is no all-registered-units coverage test. For an omitted name, `states_by_id.remove(&id)` returns `None`. |
| Omission is a skipped request, not a reset | `state_unit/src/lib.rs:1092-1185,1247-1269` under `vmm_core/`: `run_op` still orders the operation and updates bookkeeping, but `state_change` returns `Ok(None)` before sending when input is absent. `State::Stopped` bookkeeping does not assign the keeper clock. There is no post-restore hook in this path. |
| Supplied blob execution | `state_unit/src/lib.rs:239-241` dispatches `StateRequest::Restore` to the real unit. `vmm_core/src/vmtime_unit.rs:38-40` parses before awaiting `VmTimeKeeper::restore`; a parse error returns before reset. |
| Decoding and installation | `vm/vmcore/src/save_restore.rs:134-143` delegates `SavedStateBlob::parse` to protobuf Any parsing. `support/mesh/mesh_protobuf/src/message.rs:124-143` checks the type URL before decoding. `vm/vmcore/src/vmtime.rs:367-379,489-512` gives the saved `vmtime` field and delegates restoration to `reset_to`, which assigns stopped local time and awaits the primary RPC. |
| Inventory is not payload coverage | `dispatch.rs:4765-4770` optionally validates the complete inventory, then restores the supplied units. `state_unit/src/lib.rs:505-522` compares registered names, not blob coverage. An inventory containing `"vmtime"` can accompany an omitted vmtime payload. |
| Optional time adjustment | `dispatch.rs:3862-3873`, `state_unit/src/lib.rs:1048-1069`, `vmtime_unit.rs:43-45`, `vmtime.rs:496-512`: after successful restore, advancement is sent to every unit, so the keeper advances even when its restore blob was omitted. |

## Why the TOP pre-state is not the arbitrary API pre-state

There is one production call to `restore_snapshot_state`,
`dispatch.rs:3331-3333`, inside consuming `InitializedVm::load`.
The connection was checked against source, not inferred from the test.

1. `VmWorker::new` constructs `InitializedVm` before calling `load`
   (`dispatch.rs:394-400,428-434`). `VmWorker::restart` also constructs a
   new initialized VM, calls `load(Some(saved_state), ..., None, None)`,
   and only then optionally resumes (`468-480`). It does not restore into
   the previous worker's keeper.
2. The backend constructor forwards to `InitializedVm::new_with_hypervisor`
   (`openvmm/openvmm_core/src/hypervisor_backend.rs:72-84`).
   That constructor creates the keeper at `VmTime::from_100ns(0)` and awaits
   its initial source (`dispatch.rs:1102-1107`); `VmTimeKeeper::new`
   initializes local and primary state as stopped (`vmtime.rs:459-479`).
   The initialized VM retains this keeper/source (`dispatch.rs:1513-1530`).
3. `load` consumes and destructures that VM (`dispatch.rs:1562-1592`).
   It moves the keeper into the `"vmtime"` unit's `run_vmtime` task
   (`1831-1842`), retaining the `SpawnedUnit` in `LoadedVmInner._vmtime`
   (`3261-3283`). No clock-mutating keeper call intervenes. Before constructing
   `LoadedVm`, `state_units` is a non-mutable local used for registration;
   construction does not start the units.
4. `LoadedVm` is created with `running: false` and no restore guard
   (`3261-3268`), then invokes TOP immediately when saved state is present.
   The TOP body restores, optionally advances, and acquires the VP stop
   guard (`3862-3906`); it does not start the keeper. Later Worker/RPC
   resume and save-reset-restore operations are not callers of TOP.

This supports the unproved invariant: at the current TOP entry, the retained
keeper is the freshly constructed stopped-zero keeper, and an omitted
vmtime blob leaves it at zero until optional advancement. The formal proof
must connect this concrete history to `loaded_vm_representation` and
`PreparingRestore`. It cannot merely map every stopped VM to that phase:
stopped VMs can have nonzero time. Source inspection of the sole caller
does not replace a type/ownership invariant available to modular TOP proof,
nor does the sanctioned `InitializedVm::load` contract currently export the
needed keeper fact.

The accepted reset-installation and initial-source lifecycle evidence in
`../vmtime-reset-installation/README.md` and
`../vmtime-restore-source-lifecycle/README.md` is reused, not rerun.
`validation.log` confirms equality of 24 relevant source, manifest, and
built dependency-lock inputs with the accepted installation workspace.
Its conditional successful-secondary installation argument applies to the
initial source; arbitrary scheduler cancellation remains an API possibility,
not a demonstrated ordinary TOP lifecycle.

## Focused execution of unchanged production bodies

`probe.rs` calls the actual production `StateUnits`, `run_vmtime`,
`KeeperUnit`, blob parser, keeper, and primary/secondary tasks, without mock
clock, mock restore implementation, synthetic response, or custom scheduler.
It uses the normal `DefaultPool`, a retained spawned unit, and an initial
source. Four cases ran once, in two passing nextest tests:

| Initial ticks | Supplied vmtime blob | After successful restore | After 1234 ns advancement |
| --- | --- | --- | --- |
| 0 | Omitted | 0 | 12 |
| 37 | Omitted | 37 | 49 |
| 0 | Valid blob containing 91 | 91 | 103 |
| 37 | Same valid blob containing 91 | 91 | 103 |

Each case passes inventory validation with `["vmtime"]`. The test checks
both the initial source's observation and the actual keeper saved state
through `StateUnits::save`; after advancement it removes the unit and checks
the returned keeper directly. The supplied blob uses production
`SavedStateBlob::new(SavedState::from_vmtime(...))` and the real parse path.
No malformed-input decoding theorem is inferred from this successful sample.

This is **not** execution of the entire TOP, a hypervisor, or a complete
multi-unit snapshot. In particular, the adjusted harness cases have no
partition unit: the real x86 TOP separately requires a `"partition"` payload
when adjustment is requested (`dispatch.rs:3783-3797`). That restriction does
not require a vmtime payload. The harness establishes the lower-level
mechanism; the source path above supplies the narrower TOP applicability.

From the repository root, reproduce in a fresh diagnostic workspace:

```bash
bash research/vmtime-saved-state-binding/run.sh
"${ARGUS_SKILL_PYTHON:-python3}" research/vmtime-saved-state-binding/validate_inputs.py
```

`prepare.py` refuses to overwrite an existing workspace. It copies current
production inputs byte-for-byte and adds only an isolated integration-test
target to the copied `vmm_core` manifest. `validation.log` records comparison of 2,925 inputs, including dirty scalar
work. The isolated test-target suffix and the lockfile bookkeeping described
below are the only differences allowed by `validate_inputs.py`.
`runtime.log` contains the complete build/test output; 2 tests passed,
0 skipped, in 0.006 seconds (durable job including preparation/build: 21 s).
No prior completed diagnostics were rerun.

Both Cargo invocations added the already-declared dirty `vmcore` manifest's
`vstd` dependency to their lockfiles; no package version changed. The live
lockfile's one added line was removed after checking the exact diff and
timestamps, restoring its clean intake state. The isolated built lock is
retained and matches the accepted installation workspace. The comparison
script allows only this specific package-dependency bookkeeping difference;
it does not normalize any executable or proof source.

## Machine evidence and remaining obligations

The authoritative call-structure reader was attempted once and reported the
missing `.verus_agent/proof_state.json`. No graph rebuild/checker repair or
call-graph conclusion is claimed; callers and contracts above were checked
directly in source. Frozen manifests parse with the expected TOP/TCB entries.

All four required commands ran once via `checks.py`, from the live crate root
with the authoritative Python. Complete outputs are preserved in
`{make_verify,boundary,spec_drift,exec_drift}.complete.log`.

| Command module (`argus_verus.tools.checks.*`) | Result | Wrapper elapsed |
| --- | --- | --- |
| `make_verify --crate-root .` | Exit 0; **0 verified, 0 errors** on scaffolded TOP | 2.335 s |
| `boundary --crate-root . --baseline-dir .verus_agent check` | Exit 1; existing TCB provenance rejection and 7 temporary trust locations, 0 permanent violations, 0 assumptions | 1.014 s |
| `spec_drift --crate-root . --baseline-dir .verus_agent` | Exit 0; no frozen specification drift | 1.175 s |
| `exec_drift --crate-root . --baseline-dir .verus_agent` | Exit 0; no executable drift | 4.900 s |

Boundary specifically says `tcb_manifest.json was last modified by an
autonomous agent commit`. This existing setup/provenance problem is separate
from the omitted-blob semantics. A successful scaffolded verification command
is not proof of the TOP body. No unrelated repair was absorbed.

No temporary marker was added, removed, moved, or discharged. No contract,
View, invariant, or rlimit was changed. The outstanding facts are:

- **Decoding:** define exact name/type/byte interpretation for successful
  restores, including omission, and prove the real parser and selection
  path implement it. This diagnostic supports a zero-on-omission candidate
  only under the construction invariant, not a universal API zero default.
- **Lifecycle and representation:** prove zero at actual TOP entry and its
  preservation through registration, ownership transfer, other-unit restore,
  and omission. Establish the phase relation without excluding real behavior
  by fiat; relate concrete local keeper state to `LoadedVmView.virtual_time`.
- **Asynchronous installation:** admit and prove `run_op`, RPC completion,
  keeper/primary/secondary handlers and the accepted initial-source lifetime
  argument. Coverage of other or remote sources remains separate.
- **Time correspondence:** prove runtime `Duration` corresponds to decoded
  downtime, including truncation/wrapping and abstract elapsed time. The
  sample advancement does not discharge the pending Duration work.
- **All four bridges:** `decoded_restore_request_view`,
  `decoded_load_restore_request_view`, `initialized_vm_representation`, and
  `loaded_vm_representation` remain uninterpreted. The TOP
  `LoadedVm::restore_snapshot_state` remains `external_body`; existing
  `InitializedVm`/`LoadedVmInner` declaration cuts are unchanged.

### Durable runner receipts

Both final runs reported `state=submitted` with these IDs, then `state=done`.
Use the authoritative Python with
`-m argus_skill.tools.subagent status --task-id <task_id>` (`check_with`).

| task_id | run_id | Final exit |
| --- | --- | --- |
| omitted-vmtime-production-probe | omitted-vmtime-production-probe-1789906296045015940 | 0 |
| omitted-vmtime-required-checks | omitted-vmtime-required-checks-1789906323321730727 | 0 (driver completed; boundary itself is 1) |

The first checks submission,
`omitted-vmtime-required-checks-1789906296048073696`, failed preflight because
the launch executable was the literal unexpanded `${ARGUS_SKILL_PYTHON:-python3}`.
It ran no check. Resolving that executable to the current authoritative
Python produced the single actual validation run above.
