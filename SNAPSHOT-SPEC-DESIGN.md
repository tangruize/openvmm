# OpenVMM snapshot spec: design notes

Working design document for the rewritten snapshot save/restore
specification. This document records the new design as it is decided.

The earlier specification has been removed from the source: the model
(`openvmm_defs/src/snapshot.spec.rs`), the `.spec.rs`/`.proof.rs` glue files,
and all `verus_spec` contracts. It remains available in git at commit
`d326b4d6f` for reference (e.g. `git show d326b4d6f:openvmm/openvmm_defs/src/snapshot.spec.rs`).
It is not a design reference.

Source state relative to upstream `7f225e2`: the only code change is the
behavior-preserving extraction of `LoadedVm::restore_from_snapshot`
(`dispatch/restore.rs`). Verus wiring is kept: the `vstd` dependency (with the
`std` feature, required for async), `[package.metadata.verus]`, the
`verus_keep_ghost` cfg in `build.rs`, a Verus-only `vstd` import in each crate
root, and `scripts/verify-snapshot-spec.sh`.

Provenance (from the removed `SNAPSHOT-SPEC.md` reading guide):

- Upstream source baseline: `nanvix/openvmm` commit
  `7f225e2cd635bbd1e1839e587e659996433396c8`.
- Earlier customized spec and source (pre-Argus seed, October 15, 2026):
  commit `f127b8c455cabb04125eb582772c6eea326b5cc6`, also on branch
  `system-proof/snapshot-save-restore-seed`.
- `INPUT.md`, `goal.txt`, and `targets.txt` are byte-for-byte copies from task
  archive commit `031ac3c43c881409e191b32968fd4cd50d8ded29` on branch
  `system-proof/snapshot-save-restore-task-20261015`; its `production` gitlink
  points to `f127b8c`. In `INPUT.md`, `/workspace/task/production` is the
  experiment's container path; here the source is this repository.
- `human-howto-spec.txt` is an unchanged copy of an earlier natural-language
  brief for specifying `InitializedVm::load(Some(_))`.

These four files are historical inputs to the earlier specification and
describe its targets, not the current plan.

Design rule: no free spec functions. Abstractions of concrete types belong on
the type (e.g. a `View` impl, written `x@`); free lemmas are acceptable.

Configuration: microVM machine profile, Linux host, x86_64 guest.

## Current target

First step: specify and verify the state-unit save/restore pair on a stopped
`LoadedVm`.

| Function | Location | Role |
|---|---|---|
| `LoadedVm::save(&mut self) -> anyhow::Result<SavedState>` | `openvmm/openvmm_core/src/worker/dispatch.rs:4734` | `state_units.save()` + `state_units.inventory()` |
| `LoadedVm::restore(&mut self, SavedState) -> anyhow::Result<()>` | `openvmm/openvmm_core/src/worker/dispatch.rs:4757` | `validate_inventory` + `state_units.restore()` |

Both are async and require the VM to be stopped: `StateUnits::save` and
`StateUnits::restore` each `assert!(!self.running)`
(`vmm_core/state_unit/src/lib.rs:574, 607`). Stopping is the caller's
responsibility.

The interface between the two is the decoded `SavedState`
(`openvmm/openvmm_defs/src/worker.rs:46`): `units` (one saved blob per state
unit) and `inventory` (state-unit names).

Intended property: restoring the `SavedState` produced by `save` reproduces
the snapshot-relevant state of the VM at save time.

### Roadmap

`save` and `restore` are the first step, not the final goal. We eventually
want to verify the save side up to the controller, layer by layer.

1. **`LoadedVm::save` and `LoadedVm::restore`** (current target).
2. **Quiesce, after `save` and `restore` are verified.**
   `LoadedVm::quiesce_for_snapshot` (`dispatch/microvm.rs:353`) stops the
   state units, captures time, calls `save`, and then encodes the result into
   a `ProtobufMessage`. The plan is a behavior-preserving split: a new
   `quiesce_and_save` covering the steps up to and including `save` (and the
   time cut), returning the decoded `SavedState`, with `quiesce_for_snapshot`
   reduced to a thin wrapper that flushes RAM, reads the command line, and
   encodes. `quiesce_and_save` is then verified on top of `save`; it still
   starts and ends with a `LoadedVm` and needs no protobuf reasoning.
3. **Controller level: `VmController::handle_guest_snapshot_request`**
   (`openvmm/openvmm_entry/src/vm_controller/microvm.rs:150`). Moving up to
   the controller runs into the protobuf/encoding challenge: the worker
   returns the saved state encoded as a `ProtobufMessage` (a byte vector,
   `support/mesh/mesh_protobuf/src/message.rs:33`), the controller only ever
   handles the encoded form, and protobuf encode/decode will not be verified.
   This layer needs a design for relating the encoded message to the decoded
   `SavedState` (for example, a trusted contract stating that decoding returns
   what was encoded) before it can be specified.

The restore side has corresponding later layers:
`LoadedVm::restore_from_snapshot` (`dispatch/restore.rs:308`: time-ABI
preparation, `restore`, then guest-clock adjustment) and
`InitializedVm::load(Some(saved_state))` (`dispatch.rs:1824`, which builds the
`LoadedVm` and calls `restore_from_snapshot` at `dispatch.rs:3593`).

## Out of scope for now

- **Guest RAM.** It has no in-memory Rust structure (it is a file-backed
  mapping). RAM will be removed from the spec rather than left unverified.
- **Protobuf encoding/decoding** of `SavedState` (`ProtobufMessage`).
- **`SnapshotManifest`** and snapshot files (`manifest.bin`, `state.bin`,
  `memory.bin`), the controller (`VmController`), and publication.

## Concurrency among state units

Investigated for the microVM configuration on the **KVM** backend. Not
analyzed: MSHV and WHP backends.

### Scheduling rules (`vmm_core/state_unit/src/lib.rs`)

- Save and restore go through `run_op` (`:688`), which sends each unit a
  `StateRequest` RPC and awaits all replies (`join_all`).
- **Save** ignores dependencies (`|_| &[]`, `:586`): all units save
  concurrently.
- **Restore** follows dependencies (`:649`): a unit starts only after all its
  dependencies finish; units with no dependency path between them restore
  concurrently.
- A unit that saved `None` receives no Restore message (`:648`).
- Each chipset device's unit closes the device's `CloseableMutex` for the
  duration of its operation
  (`vmm_core/vmotherboard/src/chipset/backing/arc_mutex/state_unit.rs:186-220`).

### State units

`vmtime`, `time-abi`, `input`, `chipset`, `partition` (VPs, LAPIC),
`lines/irq`, `lines/bsp_lint`, `pic`, `ioapic`, `pit`, `rtc`,
`microvm-portb`, `microvm-shutdown`, `microvm-snapshot-request`, and one unit
per virtio-mmio device. VMBus, PCI/PCIe/VPCI, ISA DMA, i8042, watchdog, and
VTL2 are excluded for microVM (`openvmm/openvmm_defs/src/microvm.rs:1982-2045`).

Restore dependency order, first to last:
`vmtime` → `pic`, `ioapic` → `lines/irq` → `pit`, `rtc`, virtio-mmio →
`chipset` → `partition`. All devices depend on `vmtime`; `pic` depends on
`lines/bsp_lint`. `time-abi` and `input` have no edges.

Edge rules (`vmm_core/vmotherboard/src/chipset/backing/arc_mutex/services.rs`):
every device depends on `vmtime` (`:97`); MMIO/PIO devices are dependencies of
`chipset` (`:87-94`); a device driving a line set depends on it (`:113-119`); a
line-set target is a dependency of the line set (`:121-136`). `partition`
depends on `chipset` and `vmtime` (`dispatch.rs:3465-3467`).

### Findings

**Save:** every save handler is read-only toward other units' state.
Disjoint.

**Restore:**

- Disjoint or never called: `time-abi` (one atomic, `tsc_invariant_control`),
  `input`, `chipset`, `lines/*` (no-ops), `microvm-portb`, `microvm-shutdown`,
  `microvm-snapshot-request` (no-op), virtio-mmio devices with respect to each
  other (distinct IRQs, status words, doorbells, device tasks).
- Shared but ordered by dependencies:
  - `Arc<Halt>`: partition, chipset power/debug handlers, shutdown device.
  - `vmtime` `TimerState` (`Arc<RwLock>`): written by `vmtime`; device timers
    run after it. `rtc` writes only its own waiter slots.
  - vCPU state reached through interrupt lines: `partition` restores after
    `pic`.
- **Shared and concurrent: PIC and IOAPIC.** `pit`, `rtc`, and every
  virtio-mmio unit restore concurrently and each re-drives its interrupt line
  into the PIC and IOAPIC (e.g. PIT `sync_interrupt`,
  `vm/devices/chipset/src/pit/mod.rs:920, 195-198`). PIC and IOAPIC have
  already restored; their mutexes serialize the updates and each driver uses a
  distinct pin, so their final state is order-independent. The PIC may pulse
  LINT0 into the partition (`pic/mod.rs:89-95`; KVM
  `request_interrupt_window`), and that side effect is order-dependent.

Consequence for the spec: units can be treated as sequential, except for
concurrent per-pin writes into the PIC and IOAPIC.

### Not verified

- MSHV and WHP backends.
- Whether every virtio backend has fully quiesced holders of its transport
  interrupt before restore.
- How concurrent LINT0 pulses and IOAPIC routing updates interact inside KVM.
- That every microVM device reports PIO/MMIO at runtime (which places it
  before `partition`); checked from resolvers and constructors only.
