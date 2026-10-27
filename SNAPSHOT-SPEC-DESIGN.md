# OpenVMM snapshot spec: design notes

Working design document for the rewritten snapshot save/restore
specification. This document records the new design as it is decided.

The earlier specification has been removed from the source: the model
(`openvmm_defs/src/snapshot.spec.rs`), the `.spec.rs`/`.proof.rs` glue files,
and all `verus_spec` contracts. It remains available in git at commit
`d326b4d6f` for reference (e.g. `git show d326b4d6f:openvmm/openvmm_defs/src/snapshot.spec.rs`).
It is not a design reference.

Code changes to OpenVMM (relative to upstream `7f225e2`) and to Verus, made
and planned, are listed in `SNAPSHOT-SPEC-CODE-CHANGES.md`. Everything the
proof trusts (axioms, assumed specifications, trusted wrappers, assumptions)
is listed in `SNAPSHOT-SPEC-TRUSTED.md`.

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

## Conventions

Style reference: [microsoft/verified-storage](https://github.com/microsoft/verified-storage)
(capybaraKV), with the differences noted below.

- **No free spec functions or free lemmas**, unless there is no
  alternative. Spec functions and lemmas belong to the type they are about
  (an `impl` of that type).
- **A concrete type has two spec functions: `view` and `wf()`.** Its view
  type has an `inv()`; `wf()` includes `self@.inv()` plus the consistency of
  concrete fields the view does not represent. Preconditions rely on
  `wf()` of their parameters. State-dependent conditions that are not
  well-formedness are fields of the view (e.g. `LoadedVmView::recorded`) or
  spec functions of the view type (e.g. `StateUnitsView::all_recorded`).
  Spec functions used only by trusted axioms or lemmas are crate-private
  plumbing, not part of a type's spec.
- **Views are closed** unless they are a plain mapping of fields, so that
  their definitions do not flood every proof; lemmas unfold them where
  needed.
- **Annotation style.** Contracts are written as `verus_spec` / `verus_verify`
  attributes on the production functions. `verus! { ... }` blocks may be used
  where they are easier; mixing the two is fine. (verified-storage uses
  `verus!` throughout.)
- **File layout.** Keep production `.rs` files as clean as possible.
  - `<file>.spec.rs`: spec that a human must check (abstract types, `View`
    impls, spec functions on types that the top-level contracts use).
  - `<file>.proof.rs`: spec that need not be checked by a human, such as the
    contracts of functions called by `LoadedVm::save` or by a unit's save,
    and declarations of types to Verus; and lemmas.
  - Both are included from `<file>.rs` under `#[cfg(verus_keep_ghost)]`:
    `<file>.spec.rs` as `mod spec`, and `<file>.proof.rs` as `mod proof`
    *inside* `spec`, so that lemmas can unfold the spec's closed views (a
    closed body is visible in its module and the modules within it).
  - `<name>_t.rs`: trusted code (`external_body` wrappers and the
    assumptions they state), as in verified-storage. Each such file begins
    with what it assumes.

### Convention TODOs

Places that do not yet follow the conventions above:

- **`SavedStateUnit::map_of` and `SavedStateUnit::distinct_names`**
  (`vmm_core/state_unit/src/lib.spec.rs`) are spec functions on
  `SavedStateUnit` that take a sequence of units rather than `self`. They
  could become the view and `wf()` of a type for the list of units (used by
  `StateUnits::save`, `StateUnits::restore`, and `SavedState`'s view).
- **Trusted wrapper types** have spec functions besides a view: mesh
  `Sender`/`Receiver` (`constant()`, `attached()`), `Rpc` (`spec_input()`,
  `reply_ok()`), `PendingCall` (`reply_ok()`) in
  `support/mesh/mesh_rpc_verus/src/mesh_rpc_t.rs`, and the mutex wrapper
  (`vmm_core/state_unit/src/mutex_t.rs`). They describe what the trusted
  layer guarantees rather than an abstraction of the data; decide whether
  to keep them or recast them as views.

### Naming

Following verified-storage:

- The view of a concrete type `Foo` is always named `FooView`
  (`impl View for Foo { type V = FooView; }`), as in `KvStore` →
  `KvStoreView`. This holds even when the view type is reused elsewhere.
- Abstract states that are not the view of a concrete type are named for
  what they are, with no `View` suffix, as in `AtomicKvStore`,
  `KeyTableSnapshot`, and `JournalEntry`. If such a name would clash with a
  concrete type, the concrete type gets a `Concrete` prefix
  (`impl View for ConcreteJournalEntry { type V = JournalEntry; }`).
- A view is composed of named abstract parts, as in
  `KeyTableView { durable: KeyTableSnapshot, tentative: Option<KeyTableSnapshot>, ... }`.

Applied here:

```rust
pub struct SavedStateView { ... }  // view of the concrete SavedState:
                                   // unit name -> abstract unit state

pub struct LoadedVmView {          // view of the concrete LoadedVm
    pub snapshot: SavedStateView,  // the snapshotable part, in the same form
                                   // as a saved state
    pub runtime: ...,              // the non-snapshot part
}

impl View for SavedState { type V = SavedStateView; }
impl View for LoadedVm   { type V = LoadedVmView; }
```

The view has a non-snapshot part so that `restore` can state that it changes
only the snapshotable part (`final(self)@.runtime == old(self)@.runtime`) and
`save` can state that it changes nothing (`final(self)@ == old(self)@`).

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

### Contracts

Written in `dispatch.rs` (bodies `external_body` for now); the types are in
`vm/vmcore/src/save_restore.spec.rs`, `openvmm/openvmm_defs/src/worker.spec.rs`,
and `openvmm/openvmm_core/src/worker/dispatch.spec.rs`.

```rust
// save
requires
    old(self).wf(),
    !old(self)@.runtime.running,
    old(self)@.recorded,               // every unit's state is recorded (no `Unknown`)
ensures
    final(self).wf(),
    final(self)@ == old(self)@,        // changes nothing
    result matches Ok(saved) ==> saved.wf() && saved@ == old(self)@.snapshot,  // captures the VM's state

// restore
requires
    old(self).wf(),
    !old(self)@.runtime.running,
ensures
    final(self).wf(),
    final(self)@.runtime == old(self)@.runtime,
    final(self)@.snapshot.inventory == old(self)@.snapshot.inventory,
    result is Ok ==> {
        // units with saved state take it; the others keep theirs
        &&& final(self)@.snapshot.units
            == old(self)@.snapshot.units.union_prefer_right(state@.units)
        // the saved inventory, if any, matches the VM's
        &&& state@.inventory.len() == 0 || state@.inventory == old(self)@.snapshot.inventory
    },
```

- `SavedStateView { units: Map<Seq<char>, SavedStateBlobView>, inventory: Seq<Seq<char>> }`
  is the view of `SavedState`, and also `LoadedVmView::snapshot`.
- `SavedStateBlobView` is the abstract value a unit's blob encodes:
  `SavedStateBlobView::of(value)` for a blob encoding `value`. The trusted
  encode/decode contract (gap 4) is in `vm/vmcore/src/save_restore_t.rs`:
  `of::<T>` is injective, and the contract is attached to the real
  `SavedStateBlob::new` / `parse` (`new(data)@ == of(data)`; a successful
  `parse` returns a value whose `of` is the blob's view). Their
  `T: SavedStateRoot` bound needs the protobuf trait chain declared to
  Verus, which needs Verus patch 0003 (see "Next steps (agreed)", item 1).
- `restore`'s `union_prefer_right` reflects the code: units absent from the
  saved state get no `Restore` and keep their state; `validate_inventory`
  skips the check when the saved inventory is empty.
- `LoadedVmView::lemma_restore_gives_saved_state` (`dispatch.proof.rs`)
  states the round trip: restoring a saved state into a VM with the same
  stateful units and inventory gives back that state.
- `SavedState::wf()`: `self@.inv()` (the inventory lists each unit once and
  contains every unit with saved state) and distinct unit names.

`LoadedVm`'s view (`dispatch.spec.rs`) is computed from `StateUnits`' view
(`vmm_core/state_unit/src/lib.spec.rs`):

- `StateUnitsView { units: Map<Seq<char>, UnitSnapshot>, inventory, running }`
  (see "State-unit contracts").
- `LoadedVmView.snapshot.units` is `state_units@.saved_units()` (the units
  recorded `Saved(v)`, with value `v`); `snapshot.inventory` and
  `runtime.running` come from the `StateUnits` view; `recorded` is
  `state_units@.all_recorded()`.
- `LoadedVm::wf()`: `self@.inv()` (the snapshot's invariant) and
  `state_units.wf()`.
- `VmRuntime` has only `running`; more fields will come.

### State-unit contracts

On `StateUnits`' view (`vmm_core/state_unit/src/lib.spec.rs`): `units`
(each collected unit's recorded `UnitSnapshot`, by name), `inventory` (their
names, in id order, which is registration order), `running`. Collection
happens in `make_ids` before `LoadedVm` exists, so the view covers all
units (later registrations: see `SNAPSHOT-SPEC-TRUSTED.md`). Bodies are
`external_body`; every operation keeps `wf()`, and all but `stop` keep
`running`.

| Function | Requires | Ensures |
|---|---|---|
| `stop` | `wf`, running | stopped; same units and inventory; every unit recorded (`all_recorded`) |
| `reset` | `wf`, stopped | same units and inventory (states may change) |
| `save` | `wf`, stopped, `all_recorded` | `final@ == old@`; on `Ok(states)`: distinct names, and `SavedStateUnit::map_of(states) == old@.saved_units()` |
| `restore(states)` | `wf`, stopped | same units and inventory; on `Ok`: distinct, registered names, and `units == old.units.union_prefer_right(map_of(states) as Saved)` |
| `inventory()` | `wf` | the names, as strings, are `self@.inventory` |
| `validate_inventory(saved)` | `wf` | on `Ok`, `saved` is `self@.inventory` |

These compose into the `LoadedVm` contracts: `LoadedVm::save` builds its
`SavedState` from `StateUnits::save` (units) and `inventory()` (inventory);
`LoadedVm::restore` calls `validate_inventory` (in `dispatch/restore.rs`:
empty, or equal to the units' inventory) and `StateUnits::restore`. The
`LoadedVm` view's `snapshot.units` is `state_units@.saved_units()`, and
`LoadedVm::wf()` includes `state_units.wf()`.

`StateUnitsView::inv()`: the inventory has no duplicates and lists exactly
the recorded units. `StateUnits::wf()`: `self@.inv()`, and `StateUnits`
holds a half of each collected unit's FracGhost, belonging to its channel;
collected units have distinct names.

### `StateUnits`' unit states are the units' real states

Agreed 2026-10-26. **Property:** for each registered unit, the state that
`StateUnits`' registry records for it (`Unit.state`: `Running`, `Stopped`)
is the unit's real state. Units are registered `Stopped`, and `run_op`
sends `Stop` only to units recorded `Running` and `Start` only to units
recorded `Stopped`.

This is a correctness property of `StateUnits` in its own right: without
it, `StateUnits`' contracts speak only of what `StateUnits` believes, not of
the devices.

| Where | The spec says | It should mean |
|---|---|---|
| `StateUnits::stop` | ensures `!running` | every device has really stopped |
| `LoadedVm::save` | requires `!running` | no device changes while the units are saved one by one, so the snapshot is consistent |
| `LoadedVm::restore` | requires `!running` | no device runs while its state is replaced |
| the stop order | the partition stops first | the real devices stop in that order, so vCPUs cannot reach a stopped device |

The device-state FracGhost ("Design change: the FracGhost lives with the
device") relies on it too, at one point only: a unit answering `Stop` must
hold the whole FracGhost, that is, be running. Every other request that
reaches a unit either carries `StateUnits`' half (`Save`, `Restore`,
`Reset`, `AdvanceTime`), from which the unit proves it is stopped (holding
the whole as well would add up to more than 1), or gives it one (`Start`),
from which it proves it was stopped.

**How it is established: open.** Two options:

- **Assume it** (current choice): one trusted fact, stated where it is
  used, as a precondition of the unit's handling of `Stop` ("a unit
  receives `Stop` only while running"). `StateUnits`' side (the registry
  updates in `run_op`) can be verified.
- **Prove it** with a ghost tying each registry entry to its unit (for
  example a second FracGhost of the unit's running state, split between
  the registry and the unit). A later refinement.

### Quiesce and save

`LoadedVm::quiesce_for_snapshot` (`dispatch/microvm.rs`) is split,
behavior-preserving, into `quiesce_and_save` (reject unless microVM and
running; `StateUnits::quiesce_for_save`; time ABI capture; `save`; VM-time
cut) and a thin `quiesce_for_snapshot` that calls it, flushes guest RAM,
reads the effective command line, and encodes the saved state into a
`ProtobufMessage`. Contracts (bodies `external_body`):

| Function | Requires | Ensures |
|---|---|---|
| `quiesce_and_save` | `wf` | `wf`; inventory unchanged; on `Ok((saved, time))`: stopped, `recorded`, `saved.wf()`, `saved@ == final@.snapshot`; on `Err(Rejected)`: nothing changed |
| `StateUnits::quiesce_for_save` (`vmm_core/state_unit/src/quiesce.rs`) | `wf`, running | `wf`; same units and inventory; on `Ok`: stopped, `all_recorded`; on `Err`: still marked running |
| `capture_time_abi` | `wf` | `wf`; the view is unchanged |

These compose: after `quiesce_for_save` succeeds the units are stopped and
recorded, `capture_time_abi` changes nothing, so `save`'s precondition holds
and its result is the snapshot at the quiesce point.

Concerns:

- **The time capture is unconstrained.** `TimeCapture` (time contract, CPU
  profile) and `record_vm_time_cut` are not specified; the restore side's
  clock adjustment depends on them.
- **`quiesce_for_save` sends `Stop` through the unverified `state_change`
  path**, with its own dependency order, timeout, and cancellation; the
  contract assumes it records each unit's state as the request protocol's
  `Stop` does. Its `Err` cases (`RollbackSafe`, then
  `resume_after_failed_snapshot`; `Uncertain`, after which the VM must be
  terminated) are not specified beyond "still marked running".
- **`capture_time_abi` is trusted not to change the view**: it sends
  `CheckTimers` to every VP (`check_one_shot_timers`) and reads time
  records from the partition.
- **Two running flags**: `LoadedVm::running` (checked and cleared by
  `quiesce_and_save`) and `StateUnits::running` (the view's
  `runtime.running`). The contract speaks only of the latter.
- **After `quiesce_and_save` the VM stays stopped**; resuming or ending it
  is the caller's (controller's) business.

### Restore from snapshot

`LoadedVm::restore_from_snapshot` (`dispatch/restore.rs`) is
`begin_snapshot_restore`, then `restore`, then `finish_snapshot_restore`.
For a time ABI restore, `begin` has the partition drop the VPs' saved TSCs
(`omit_saved_tsc`), and `finish` runs the clock restore (`time_abi_restore`:
synchronized TSC set, LAPIC timer advance, `StateUnits::advance_time` for VM
time and the RTC) and holds the VPs stopped until the VM first starts.

So after a time ABI restore, some units do not hold exactly their saved
state: the partition and the units that advance time. The view names them:
`LoadedVmView::time_units` is `state_units@.advances_time` plus
`"partition"`, and `LoadedVmView::clock_units` is `time_units` while a time
ABI clock restore is pending (`SnapshotRestore`'s view,
`clock_restore_pending`), and empty otherwise. (`time_units` was added for
`InitializedVm::load`'s contract, which must name these units after the
clock restore has run and `clock_units` is empty; every restore contract
keeps it.)
`StateUnitsView::advances_time` is new (the units registered with
`advances_time()`; the ghost registry records the flag at collection).

Contracts (bodies `external_body`):

| Function | Requires | Ensures |
|---|---|---|
| `restore_from_snapshot(saved)` | `wf`, stopped | `wf`; runtime and inventory unchanged; on `Ok`: `clock_units` empty, units outside the old `clock_units` are as `restore` leaves them (`old.units.union_prefer_right(saved@.units)`), and the saved inventory is empty or the VM's |
| `begin_snapshot_restore` | `wf` | `wf`; view unchanged |
| `finish_snapshot_restore` | `wf`, stopped | `wf`; runtime and inventory unchanged; `clock_units` empty; units outside the old `clock_units` unchanged |
| `time_abi_restore` (`dispatch/time_abi.rs`) | `wf`, stopped | `wf`; runtime, inventory, `time_units`, `clock_units` unchanged; units outside `time_units` unchanged |
| `StateUnits::advance_time` (`vmm_core/state_unit/src/advance_time.rs`) | `wf`, stopped | `wf`; same units, inventory, `advances_time`; units outside `advances_time` unchanged |
| `LoadedVm::restore` (added) | | `time_units` and `clock_units` unchanged |
| `InitializedVm::load(saved, .., snapshot_restore)` (`dispatch.rs`) | | on `Ok(vm)`: `vm.wf()`; stopped; `clock_units` empty. With `saved` `None`: no unit recorded (`snapshot.units` empty), and no clock restore was pending. With `Some(state)`: outside the clock units (`time_units` if `snapshot_restore` had a clock restore pending, else none), the units have exactly `state@.units`; the saved inventory is empty or the VM's |

`restore_from_snapshot`, `restore`, and `finish_snapshot_restore` also keep
`time_units`.

`load`'s contract follows from the fresh VM it builds and
`restore_from_snapshot`'s: `LoadedVmView::lemma_load_restores`
(`dispatch.proof.rs`, verified) derives it from "no unit recorded, clock
units set if pending" and `restore_from_snapshot`'s `Ok` result (a fresh
VM records nothing, so the union with the saved units is the saved units).
Negative check: claiming the clock units also take their saved state
fails. The fresh-VM facts themselves are part of `load`'s unproved body:
`make_ids` collects every unit's half, recording `Unknown`.

They compose: `begin` changes nothing, `restore` gives the union and keeps
`clock_units`, and `finish` changes only `clock_units` and empties it. All
`StateUnits` operations now also keep `advances_time`.

Concerns and TODOs:

- **The clock units' restored state is unconstrained.** It is the saved
  state advanced by the downtime (TSC target, LAPIC timers, VM time, RTC).
  TODO: specify the clock restore; this needs typed views of those units'
  saved states (`SavedStateBlobView` is opaque) and a spec of the time
  contract (`RestoreTimeInput`, validated by the controller, is not
  specified).
- **The partition's view can go stale.** The TSC set and the LAPIC advance
  write the partition's VPs directly through `partition_unit`, outside the
  request protocol, so the partition's FracGhost half is not updated: after
  `finish`, the view still says the partition holds its saved blob, while
  its TSCs and timers differ. Likewise `omit_saved_tsc` means the partition's
  `restore` does not fully apply its blob, against the unit-side restore
  contract. The `restore_from_snapshot` contract is not affected (it says
  nothing about clock units), but the view after it is wrong for the
  partition. TODO: route these writes through a ghost-updating path, or mark
  the partition's recorded state as changed (e.g. `Unknown`) after the
  clock restore.
- **`SnapshotRestore`'s view is uninterpreted** (trusted to mean
  `time_abi.is_some()`), because `SnapshotRestore` has private fields.
- **`"partition"` is hard-coded** in `LoadedVm`'s view (the unit is added as
  `"partition"` in `dispatch.rs`). TODO: share a constant, or derive the
  set from the units the clock restore touches.
- **Error paths**: on `Err`, only `wf`, runtime, and inventory are
  promised. A failed `begin` (a time ABI restore without partition state)
  or `restore` leaves the clock restore pending; a failed `finish` may have
  partly advanced the clocks.
- **Not modelled**: the start guard (VPs held stopped until the first
  start), `restored_from_snapshot`, the restore packet's time fields, and
  profiling.
- **After `load`, units without saved state are not recorded.** They
  still record `Unknown` (only units in `state@.units` are restored), so
  `vm@.recorded` is false unless every unit had saved state, and
  `LoadedVm::save` (which requires `recorded`) cannot be called until the
  units are started and stopped again. Matches the code's use (a restored
  VM is resumed before it is saved again), but worth confirming.
- **Annotating `load` gives four Verus warnings** ("verus-related
  attribute has no effect") on constants declared inside its body;
  harmless.
- **Next layer up**: the controller
  (`VmController::handle_guest_snapshot_request` and the worker's restore
  entry), which calls `load`.

### Design change: the FracGhost lives with the device

Decided 2026-10-26. Implemented (refactoring steps 1 to 5, below): the
request protocol and `StateUnits`' view follow the rule; the PIT keeps its
share in a shared ghost field (`DeviceGhost`); and a verified unit
(`DeviceUnit`) drives it. Not yet: the PIT's code bodies proved, normal
builds, and wiring the PIT to `DeviceUnit` (see "Next steps (agreed)",
item 5).

**Problem with the current placement.** The unit's half and its invariant
("while stopped, my half records my current snapshot") live in the unit's
task. But vCPUs change a chipset device through its I/O handlers, which run
on vCPU threads, not in the unit's task, and never see the half. So "a
stopped device does not change" cannot even be stated as an obligation for
those handlers; it was a trusted assumption.

**What keeps vCPUs away today** (correcting the earlier account): not the
`CloseableMutex`. A chipset unit closes its device's mutex only for the
duration of each operation (`self.device.clone().close().save()` closes it,
and dropping the guard reopens it). Between `Stop` and `Save` it is open.
What keeps vCPUs away is that they are stopped: the partition unit depends
on the chipset devices, and `stop` / `quiesce_for_save` stop dependants
first, so the vCPUs stop before any device does and stay stopped until
`start`. (`CloseableMutex::lock` panics if the mutex is closed, so a vCPU
access during a unit operation would crash; the design relies on this
order.)

**Every change to a chipset device goes through its `CloseableMutex`**
(checked): the device object is stored only inside
`Arc<CloseableMutex<T>>` (`vmm_core/vmotherboard/src/chipset/backing/arc_mutex/device.rs:240`),
in an `UnsafeCell` reachable only through the mutex's guards; vCPU MMIO and
port I/O dispatch locks it (`chipset/mod.rs`, `lookup.dev.lock()`); a
line-set target locks it before `set_irq` (`chipset/line_sets.rs`); the
unit's operations `close()` or `lock()` it (`backing/arc_mutex/state_unit.rs`).

**The change.** The device's FracGhost (the receiver's half) is kept in the
device object, inside the `CloseableMutex`, next to the state it describes,
instead of in the unit's task. Then every mutation path (the unit's
`save` / `restore` and the vCPUs' I/O handlers) can be verified against the
device's `wf()`:

- While running, the device holds the whole FracGhost; its handlers may
  change the state, updating the ghost with each change (see "The rule").
- At `Stop`, the device splits the ghost and gives a half to `StateUnits`.
- While stopped, it holds one half, and `wf()` says "state equals the
  ghost". A handler that changed the state would break `wf()` and fail to
  verify, so a handler must either check a real "stopped" flag or require
  "running" as a precondition. In the latter case the remaining trusted
  fact is one explicit precondition on the unverified caller (the vCPU
  dispatch calls handlers only while the device runs), stated where it is
  used.
- The ghost is updated with every change (one proof step per mutating
  handler), so `wf()` is simply "state equals ghost", with no mention of
  halves. (An alternative, setting it only at `Stop`, needs a `wf()` that
  depends on which halves the device holds.)

**Scope: what the device owns.** A device's fields may refer to shared
state outside its mutex: the PIT's `VmTimeAccess` (its timer deadlines are
in vmtime's shared timer state), and interrupt line levels (in the line
sets). Changes there do not go through the device's mutex; they are the
cross-unit effects already listed (per-pin resources; vmtime as its own
unit). So a device's `saved_state()` should be defined over what it owns.

**The rule** (agreed 2026-10-26):

> While a device is running, the device holds the whole FracGhost. While it
> is stopped, the device and `LoadedVm` (through `StateUnits`) each hold one
> half. A freshly created VM counts as stopped.

| Moment | Device holds | `LoadedVm` holds |
|---|---|---|
| The VM is created (stopped) | one half | one half |
| `Start` | the whole (`LoadedVm` sends its half) | nothing |
| `Stop` | one half (records its state, sends one half back) | one half |
| `Save` | one half (unchanged) | one half (lends it, gets it back) |
| `Restore` | one half (`LoadedVm` lends its half; the device updates the ghost, sends a half back) | one half |

Whenever `LoadedVm` asks a stopped device to save or restore, it holds a
half, so save and restore work the same way on a fresh VM and on one
stopped earlier. (Letting a fresh VM's devices hold the whole instead would
give a device's `restore` two cases, "I hold the whole" and "I hold half
and one arrived", that its code cannot tell apart at runtime.) The device's
`wf()` is simply "my state's snapshot equals my ghost's value", kept by
updating the ghost with every change, which needs the whole; while it holds
half it cannot change its state.

Consequence: at creation `LoadedVm` must already have its half, so the
creation-time hand-off stays (the half attached to the `Sender`, kept by
the registry at registration, collected
by `StateUnits`), and so does the motivation for TODO 4 (registration
through `&mut StateUnits`). Open: whether any code changes a device after
it is registered and before its first `Start` (for example while loading
firmware); under this rule that would fail the proof.

**What it changes:** the receiver's half goes into the device object (for
chipset devices, inside the `CloseableMutex`; other units need their own
place); the unit-side contract moves down to the device: `SaveRestore`'s
contract and the device's `wf()` include the ghost, and the device's I/O
handlers get contracts; verifying a device means verifying all its mutation
paths, including its I/O handlers.

**Refactoring steps:**

1. *Done.* The request protocol follows the rule (`Start` gives
   `StateUnits`' half; `Stop` returns one), and so do `StateUnits::wf()`,
   its view, and `ExampleUnit` (see "Request protocol").
2. *Decided.* `unit_channel()` keeps creating the FracGhost with `Unknown`,
   so a device's `wf()` allows `Unknown` ("state equals ghost, unless the
   ghost is `Unknown`").
3. *Done.* A trusted spec of `CloseableMutex`
   (`support/closeable_mutex/src/closeable_mutex_t.rs`): a lock invariant
   given by the value's type (`LockInvariant`: `inv()`, and a `key()` that
   every holder keeps, for a device its FracGhost's id, so that verified
   code relates the values it sees on different acquisitions). Verified
   code closes with `CloseableMutex::close_verified`, gets the value
   through the guard's `borrow` / `borrow_mut` (a `&mut` with a prophecy
   spec), and reopens with `release`, which requires `inv()` and the key
   unchanged. A guard dropped without `release` leaves the mutex closed
   (later locks panic): safe, not unsound. The predicate comes from the
   type because `CloseableMutex<T>` has no parameter for one and is created
   by unverified, generic code. No verified `lock` yet: the vCPU paths that
   lock are unverified and trusted to keep the invariant. Example
   `ExampleCounter` (`support/closeable_mutex/src/lib.proof.rs`) verifies;
   three negative checks (breaking the invariant twice, changing the key)
   fail; a runtime test covers release and drop.
4. *Done (contracts; handler bodies not yet proved).* The PIT
   (`vm/devices/chipset/src/pit/`; spec in `mod.spec.rs`; declarations, its
   `StateUnitDevice` implementation, and examples in `mod.proof.rs`). See
   "The PIT" below.
5. *Done (Verus only; not yet wired or tested).* The unit loop. Decided 2026-10-26: the shared chipset unit
   (`ArcMutexChipsetDeviceUnit`, used by about 56 device types) stays as it
   is, trusted; a separate verified unit, `DeviceUnit<T>`
   (`vmm_core/state_unit/src/device_unit.rs`, Verus only for now), is for
   devices that implement `StateUnitDevice` (for now, the PIT): verified
   `handle` and `poll`, and the unverified run loop. See "The verified
   unit" below.

Alongside: a survey of cross-device changes (lines re-driven into the
PIC/IOAPIC at restore, vmtime, `Halt`, the time-ABI writes to the
partition), which need modelling.

#### The PIT

- **The shared ghost field** (`vm/vmcore/src/device_state.spec.rs`): a
  device keeps one Verus-only field of type `DeviceGhostState` (`frac`, its
  share of the FracGhost; `running`). `DeviceGhostState::wf(id, snapshot)`
  is the rule, written once for every device: the FracGhost with id `id`,
  whole or half, whole while running, and, unless `Unknown`, recording
  `snapshot`. The trait `DeviceGhost` lets generic code reach the field:
  spec functions `ghost()`, `id()`, `snapshot()`, `device_inv()` (the
  device's own invariant), and `ghost_mut()`, which hands out the field as
  a `&mut` and promises the device's state, id, and invariant unchanged.
  The state unit then combines, splits, and updates the FracGhost in its
  own proof blocks; devices write no ghost code of their own. (A probe
  showed this also verifies with the device behind `dyn` and inside a
  `CloseableMutex` of a `dyn` value; see step 5.)
- **The PIT's fields** (Verus only, zero-sized): `ghost:
  Tracked<DeviceGhostState>` and `id`. `UnitSnapshot` moved to
  `vm/vmcore/src/save_restore.spec.rs` so that devices can name it.
- **View** `PitDeviceView`: the three timers' states, `last`, `time_abi`.
  Not in it: `vmtime`, the interrupt line, `dram_refresh` (flips on each
  port-61 read; not saved), and the ghost field. `inv()`: three timers.
  `saved_state()` is defined on the view, field by field;
  `SaveRestoreSpecImpl::saved_state` is the view's; `snapshot()` is
  `Saved(of(saved_state()))`. `wf()`: the view's invariant, and the ghost
  field follows the rule for `snapshot()`. `LockInvariant`: key `id`,
  invariant `wf()`.
- **Entry points with contracts** (`external_body`: stated, not proved).
  The trait methods delegate to inherent methods: `start_device`, `poll`,
  `pio_read`, `pio_write` require `wf()` and running and keep both;
  `reset_device` and `restore_device` require `wf()`, whole, and stopped,
  and keep them; `restore_device` on `Ok` sets `saved_state()`;
  `save_device` changes neither the view nor the ghost and returns
  `saved_state()`. Proving a handler means updating the FracGhost after
  each change (it holds the whole while running).
- **Examples** (verified): `example_start`, `example_stop`,
  `example_save`, `example_restore`, the PIT's unit answering each request
  through `CloseableMutex::close_verified` / `release`, doing the ghost
  work in its own proof blocks through `ghost_mut`, and meeting the request
  protocol. `example_stop` assumes the device is running (the trusted fact
  of "`StateUnits`' unit states are the units' real states"). Negative
  checks, each failing: `Start` without the bound proof; `Stop` without
  recording, without clearing `running`, or without the trusted fact;
  `Save` without agreeing; `Restore` without taking the lent half.

Findings and open items:

- **The generic `SaveRestore` contract says nothing of `wf()` or the
  ghost**, so a caller cannot know that `save` / `restore` keep them. The
  PIT states this on `save_device` / `restore_device`; the generic unit
  (step 5) needs it generically.
- **Generic code and `dyn`.** The chipset unit holds its device as
  `Arc<CloseableMutex<dyn DynDevice>>`. The probe showed `DeviceGhost`
  works through `dyn`, with `LockInvariant` implemented for the `dyn` type
  (its invariant: the rule, from `ghost()`, `id()`, `snapshot()`). Needed
  for step 5: the `CloseableMutex` wrapper must accept unsized values
  (specs comparing `&T`, since spec equality needs sized values), and the
  device's own invariant must reach the `dyn` level (`device_inv()`).
- **Creation**: `PitDevice::new` (unverified) fills the ghost fields with
  placeholders (`assume_new`). The device should receive its unit's half
  at creation, and its mutex should be made with `new_verified`.
- **A recorded snapshot need not be saveable**: with the time ABI and
  channel 0 periodic, `save` fails (`E_PIT_ACTIVE`), yet `Stop` records
  `Saved(of(saved_state()))`, the state `save` would return if it
  succeeded.
- **`restore` on `Err`** leaves the timers and `last` changed; the unit
  still returns a half recording the new snapshot, which the protocol
  allows.
- **Cross-device effects**: `restore`, `reset`, and the handlers drive the
  interrupt line (`sync_interrupt`), and the handlers set vmtime timeouts;
  not modelled.

#### The verified unit

- **`StateUnitDevice`** (`vm/vmcore/src/device_state.spec.rs`): what a
  verified unit needs from its device, as synchronous methods with
  contracts (Verus does not support async trait methods):
  `start_device`, `stop_device` (require running), `reset_device`,
  `restore_blob` (require whole and stopped), and `save_blob` (changes
  neither state nor ghost; on `Ok`, the snapshot is `Saved(blob@)`). Each
  keeps `device_inv()`, the rule, and the id. Two lemmas each device
  proves: `lemma_lock_inv` (its `CloseableMutex` invariant is
  `device_inv()` plus the rule, and the key is its id) and, in
  `DeviceGhost`, `lemma_snapshot_known` (its snapshot is never `Unknown`,
  needed for `Stop`).
- **The PIT implements it** (`pit/mod.proof.rs`, verified), calling
  `SavedStateBlob::new` / `parse` directly for its blob. (Until 2026-10-27
  it used two trusted per-type wrappers, `pit/blob_t.rs`, now removed.)
- **`DeviceUnit<T>::handle(msg, gin)`** (verified, for any
  `T: StateUnitDevice`): requires the request protocol (`request_ok`) for
  `msg` and `gin`, and answers every request but `Inspect` by it, through
  `CloseableMutex::close_verified` / `release` and the mesh RPC wrapper's
  `split` / `complete`, doing the FracGhost work in its own proof blocks.
  `Stop` assumes the device runs (the trusted fact). `AdvanceTime` returns
  the lent half unchanged (`StateUnitDevice` devices keep no time).
  Negative checks, each failing: `Start` without the bound proof; `Stop`
  without recording or without the trusted fact; `Save` without agreeing,
  or replying `Ok(None)`; `Restore` without recording the restored state.
- `CloseableMutex::close_verified` now takes `&Arc` (vstd has no spec of
  `Arc::clone`; OpenVMM calls `self.device.clone().close()`).

- **The run loop** (`DeviceUnit::run`, unverified glue, following
  `ArcMutexChipsetDeviceUnit::run`): `futures::select!` between the next
  request (`recv_msg`, which gives the message with its ghost and
  `request_ok`) and the poll event while running. It answers `Inspect`
  itself (through the unverified `lock()`), passes every other message
  and its ghost unchanged to `handle`, sets its `running` flag after
  `Start` / `Stop`, and polls once after `Start` so the device arms its
  timers. Polling is verified (`DeviceUnit::poll`): it locks the mutex
  without closing it (vCPUs keep using the device; closing would make
  their accesses panic), through the new `CloseableMutex::lock_verified`
  / `VerifiedOpenGuard::release`, and calls `StateUnitDevice::poll_device`
  (new; requires running). `poll` assumes the device runs: the unit polls
  only while its `running` flag is set, the same trusted fact as for
  `Stop`. Negative check: `poll` without it fails.

Not yet done:

- **Testing the run loop**: it compiles only in Verus builds (see "Normal
  builds"), so it has not run; `cargo verus` type-checks it but runs no
  tests.
- **Wiring**: the chipset builder creates `ArcMutexChipsetDeviceUnit` for
  every device; the PIT would get a `DeviceUnit`, with its half handed
  over at creation (`PitDevice::new` still has placeholders), its mutex
  created by `new_verified`, and the channel's constant as the id.
- **Normal builds**: `DeviceUnit`, `DeviceGhost`, and the ghost types are
  Verus-only (`cfg(verus_keep_ghost)`); production use needs them in
  normal builds (as zero-sized ghost code).
- `StateUnitDevice` has no `advance_time`, `quiesce_input`, or
  `resume_input`; needed for devices that use them.

#### Verifying another device

Moved to its own section: "Recipe: verifying another device".

### Unit-side contract

What a state unit must do for the request protocol to hold, layer by layer,
from the device up.

1. **Device: `SaveRestore`** (`vm/vmcore`). *Attached.* Each implementation
   has a spec function `saved_state()`, its snapshotable state: what `save`
   returns. `save` does not change it and returns it; a successful
   `restore(state)` sets it to `state`. Given in
   `vm/vmcore/src/save_restore.spec.rs` with `external_trait_specification`
   and `external_trait_extension(SaveRestoreSpec via SaveRestoreSpecImpl)`
   (vstd's pattern for `Iterator`), so the trait and its many
   implementations are unchanged; a verified implementation defines
   `saved_state` by implementing `SaveRestoreSpecImpl`. Example:
   `ExampleDevice` (`save_restore.proof.rs`) verifies; injecting a `restore`
   that ignores its state, or a `save` that returns another value, each
   fails.
2. **Blob: `ProtobufSaveRestore`** (blanket impl for `SaveRestore` types).
   *Attached and verified* (2026-10-27): `external_trait_specification` +
   `external_trait_extension(ProtobufSaveRestoreSpec via
   ProtobufSaveRestoreSpecImpl)` in `vm/vmcore/src/save_restore.spec.rs`,
   with `saved_blob()` the encoded snapshotable state; the blanket impl
   (`save_restore.rs`, now `verus_verify`, its bodies rewritten without the
   closure and `?`) is verified against it, with `saved_blob() ==
   of(saved_state())` (`save_restore.proof.rs`). Negative check: a
   `restore` that ignores the decoded state fails. The other
   implementations (`ChipsetDeviceProxy`, `BoundVp`,
   `ErasedChipsetDevice`) are trusted to meet it. `save` returns a blob with view
   `of(old.saved_state())`; a successful `restore(blob)` leaves
   `of(saved_state()) == blob@`. Follows from layer 1 and the `SavedStateBlob`
   contract.
3. **State unit: `StateUnit`** (async trait, `vmm_core/state_unit`).
   *Stated, not attached.* Through the unit's `UnitSnapshot`: `save` returns
   `Ok(Some(blob))` with `Saved(blob@)`, or `Ok(None)` with `NoSavedState`,
   the unit's current snapshot; `restore(blob)` on success makes it
   `Saved(blob@)`; `stop` leaves the unit stopped with its state stable;
   `reset` and `advance_time` may change it. For a chipset device
   (`ArcMutexChipsetDeviceUnit`, `vmm_core/vmotherboard`), the snapshot is
   `NoSavedState` if `omit_saved_state`, else `Saved(of(saved_state()))` of
   its device.
4. **Request loop: `run_unit` / `StateRequest::apply`**. *Stated, not
   attached* for the production units. The unit answers each request by
   the request protocol. Under "Design change: the FracGhost lives with the
   device", the half lives in the device, not in the unit's task. *Done*
   for the verified unit: `DeviceUnit::handle` ("The verified unit").

Concerns:

- **`SavedStateRoot` trait chain**: resolved 2026-10-27 (Verus patch 0003;
  see "Next steps (agreed)", item 1), for derived structs (`TableEncoder`)
  and enums (`OneofEncoder`; checked on an example enum root,
  `ExampleEnumState` in `save_restore.proof.rs`, since all of OpenVMM's 85
  saved-state roots are structs). The derive's `transparent` and `prost`
  modes are not declared; no saved-state root uses them.
- **Other `ProtobufSaveRestore` implementations** need their own contracts:
  `ChipsetDeviceProxy` (`workers/chipset_device_worker`), `BoundVp`
  (`vmm_core/src/partition_unit/vp_set.rs`), and `ErasedChipsetDevice`
  (`vm/chipset_device_resources`).
- **`async fn` in traits** (Verus: "Opaque types in trait def") blocks
  layer 3 on `StateUnit` itself. Options: Verus support, or contracts on each
  unit's concrete implementation through wrapper functions.
- **Async closures** in `StateRequest::apply` (`rpc.handle_failable(async
  |..| ..)`) block layer 4; `apply` would be rewritten without them (a
  source change).
- **`run_async_unit`** handles concurrent `Inspect` requests while a state
  change runs, with the unit as `&T`: interior mutability, and the
  assumption that inspects do not change saved state.
- **`saved_state()` is a spec function on the trait**, alongside each
  implementation's view (if any). Open question: whether a device's view
  should be its saved state, or `saved_state()` derived from its view.
- **Deterministic `save`**: the contract makes `save`'s result a function
  of the state (`saved_state()`). A device whose saved state includes
  something else (time, counters that `save` itself advances) must model it
  in `saved_state`, or does not meet the contract.
- **`restore(s)` then `saved_state() == s`**: a device that normalizes or
  only partly restores a saved state does not meet the contract as stated.
- **`save` takes `&mut self`** (e.g. `WrappedVp::save` flushes async
  requests); the contract says `saved_state()` is unchanged.
- **Devices behind `CloseableMutex`**: while running, vCPUs change them
  outside the unit's task (the ghost goes stale, which is fine); while
  stopped, they are kept unchanged by the stop order (vCPUs stop first),
  not by the mutex (see "Design change: the FracGhost lives with the
  device", which makes this provable).
- **`reset` and `advance_time`** change unit state; the protocol only
  returns the half. `advance_time` runs after restore
  (`restore_from_snapshot`), so that layer must account for it.
- **Units not built on `SaveRestore`**: the partition unit (VPs through the
  hypervisor, KVM state; trusted), vmtime, time-abi, input, and the line
  sets need their own snapshot definitions and contracts.
- **Cross-unit effects at restore**: PIT, RTC, and virtio-mmio re-drive
  their interrupt lines into the PIC and IOAPIC; not yet modelled (see
  "Interrupt lines as resources").
- **Lossless protobuf encoding** (the `SavedStateBlob` injectivity axiom)
  underlies layers 2 and 3.

### Where the halves come from

Facts from the code:

- Loading a snapshot builds a new `LoadedVm` and calls `restore` on it
  (`make_ids`, `dispatch.rs:3505`) before any unit has started; units are
  registered `Stopped` (`UnitBuilder::build`). So the halves must exist from
  unit creation, not only after a `Stop`.
- Units are registered through a shared `&StateUnits` from several crates
  (the chipset builder keeps a `&'a StateUnits`; also `vmbus_unit`,
  `partition_unit`, vmtime, time-abi). The registry is behind
  `Arc<Mutex<Inner>>`, and dropping a `UnitHandle` removes its unit through
  a `Weak`.
- Not every unit is created by `spawn`: the chipset makes its own channel
  and calls `build(send)`. What every unit shares is the creation of its
  channel, then `build`.

Design:

1. **Created with the channel.** `unit_channel()` (verified,
   `vmm_core/state_unit/src/unit_channel.rs`) creates the unit's FracGhost
   and, through the trusted `channel(k, gs, gr)`, attaches one half to each
   end; the channel's constant is the ghost's id. `UnitBuilder::spawn` and
   the chipset builder (the one unit that makes its own channel) call it.
   The unit's run loop takes the receiver's half when it starts. A cloned
   sender carries none. Crates in between need no signature changes.
2. **Initial value "not yet known".** The ghost value is a `UnitSnapshot`
   (`vm/vmcore/src/save_restore.spec.rs`, so that devices can name it):
   `Unknown`, `NoSavedState`, or
   `Saved(SavedStateBlobView)`, starting `Unknown`. (`NoSavedState` is
   needed because a unit whose save returns `None` has a known state that
   is not a blob.) On `Stop`, the running unit, holding the whole, records
   what its save would return and sends a half back; `Restore` records `Saved(blob@)`
   (this covers restore on a fresh VM); `Save` only lends. `save` requires
   no half to be `Unknown`. The unit's run loop keeps a ghost-only
   running/stopped flag; its invariant is "while stopped, my half records
   what my save returns".
3. **Kept with the registry, then collected by `StateUnits`** (since
   2026-10-27, step 4 of "Next steps (agreed)"; before, the half stayed
   attached to the `Sender` and two registry axioms tied the registry to
   `StateUnits`). Registration is through `&StateUnits`
   (`UnitBuilder::build`) and removal through `UnitHandle`'s `Drop`; both
   now call small verified functions on the registry
   (`vmm_core/state_unit/src/registry.rs`): `Inner::insert_unit` takes the
   half attached to the unit's sender into the registry's ghost state, and
   `Inner::remove_unit` drops the unit's entry (and its half, if still
   there). The registry's ghost state (`RegistryGhost`, a Verus-only field
   of `Inner`, behind the mutex) holds each unit's entry (name, FracGhost
   id, `advances_time`), the halves of units registered since the last
   collection, and one half of a shared ghost record of the units
   `StateUnits` has collected; `StateUnits` holds the other half
   (`UnitHalves::collected`). The lock invariant (`InnerPred`,
   `lib.proof.rs`) says the entries match the units, names are distinct,
   and each registered unit is either collected (with its entry unchanged)
   or has its half in the registry. `StateUnits::collect_halves(&mut self)`
   (verified, Verus-only) locks the registry, takes the new halves, drops
   those of removed units, and updates both halves of the shared record:
   afterwards `StateUnits`' ghost state (`units_ghost`) covers exactly the
   registered units, which the view reads by name. The mutex wrapper's new
   `key()` (the shared record's id) ties the registry's half to this
   `StateUnits` (`StateUnits::wf`). No registry axiom remains. One trusted
   fact remains for operations that lock the registry more than once (for
   their proofs): no unit is registered while an operation holds
   `&mut StateUnits` (registration needs `&StateUnits`). Removal during an
   operation is possible; the specs of those operations will have to allow
   it (today the code ignores a removed unit's failure).
4. Done: `LoadedVm::halves` is removed; `LoadedVm`'s view reads
   `state_units@`.

No atomic invariant is needed: exact knowledge comes from holding halves;
the lock invariant only ties the uncollected halves to the registered
senders.

### Request protocol

`MeshProtocol::request_ok` for `StateRequest` (`vmm_core/state_unit/src/lib.proof.rs`).
`k` is the unit's ghost id. Following the rule ("Design change: the
FracGhost lives with the device"), a running unit holds the whole
FracGhost; while it is stopped, the caller (`StateUnits`) holds one half
and the unit the other.

| Request | Ghost sent | Reply must satisfy |
|---|---|---|
| `Start` | caller's half (given) | no ghost, also on `Err` (a unit that fails to start ends in a terminal state, `QuiesceUncertain`) |
| `Stop` | none | a half back recording a known state (not `Unknown`) |
| `Save` | caller's half (lent), not `Unknown` | the same half back; `Ok(Some(blob))` iff it records `Saved(blob@)`, `Ok(None)` iff `NoSavedState` |
| `Restore(blob)` | caller's half | a half back; on `Ok` it records `Saved(blob@)` |
| `Reset`, `AdvanceTime` | caller's half | a half back, any value |
| `QuiesceInput`, `ResumeInput`, `Inspect` | none | no ghost |

A reply to `Save`, `Restore`, `Reset`, or `AdvanceTime` returns a half also
on `Err`, so the caller keeps it (`save` promises to change nothing).

`StateUnits::wf()` follows: `StateUnits` holds halves only of collected
units, and while stopped (`!running`) one for every collected unit. While
running it may still hold some: those of units added while running, which
stay stopped until `start_stopped_units`. Its view records `Unknown` for a
unit whose half it does not hold.

Verified examples (Verus-only, in `lib.proof.rs`): `example_save_call`, a
caller that lends its half for `Save` and learns that the blob matches it;
and `ExampleUnit`, a unit following the rule: `handle` answers `Start`,
`Stop`, `Save`, and `Restore`, and `set` changes its state as an I/O
handler would, only while running (its `wf()`: it holds the whole of its
ghost while running and one half while stopped, and the ghost, unless
`Unknown`, records what its save returns). `handle` requires that `Stop`
arrives only while it runs: the trusted fact of "`StateUnits`' unit states
are the units' real states". For `Start` and `Restore` it proves from the
half it receives that it was stopped. Negative checks, each failing:
`Save` always `None`; `Restore` ignoring the blob; `Stop` not recording
(earlier version); dropping the `Stop`-while-running precondition; dropping
the proof that a unit receiving `Start` was stopped.

New trusted facts and assumptions:

- `SavedStateBlob::clone` preserves the encoded value
  (`assume_specification`, `vm/vmcore/src/save_restore.spec.rs`; part of the
  blob contract, gap 4).
- `Inspect` requests do not change a unit's saved state (they reach the unit
  through `InspectMut`, with no ghost).
- Channel failures (a unit's task gone) do not happen for a registered unit:
  OpenVMM's `run_op` panics on them, and the ghost would be lost.

Not yet done: the production unit side (each `StateUnit`) is blocked on
async trait functions (gap 1); `run_unit` and `StateRequest::apply` now use
the mesh RPC wrapper, unverified, with placeholder ghosts. The verified unit
`DeviceUnit` meets the protocol for devices implementing `StateUnitDevice`
("The verified unit").
`StateUnits::stop`/`save`/`restore` and `run_op` are not yet verified
against the protocol.

### Mutex wrapper

A standalone explanation (mental model, how to use it, design choices) is
in `SNAPSHOT-SPEC-MUTEX.md`.

`StateUnits` uses `parking_lot::Mutex`. vstd has `RwLock` but not `Mutex`;
vstd's `RwLock` is a spinlock that is not released on drop, so switching
would change runtime behavior. Instead, a trusted wrapper,
`vmm_core/state_unit/src/mutex_t.rs` (first drafted in the prototype; the
`state_unit` version names the invariant by a static `MutexPredicate`
instead of a ghost value):

- Runtime: `parking_lot::Mutex<Option<V>>`; the real lock is unchanged.
- Verified code: `acquire(&self) -> (V, MutexHandle)` gives the value, which
  satisfies the invariant;
  `MutexHandle::release(v)` requires the invariant. A handle dropped
  without `release` leaves `None`, so the next lock panics: safe, not
  unsound.
- Unverified code: `lock()` returns an ordinary (mapped) guard, so its call
  sites do not change. It is `verifier::external`. Assumption: unverified
  code keeps the invariant.

Example (`mutex_unit.rs`): units registered through `&self` leave their
half in the mutex; `collect(&mut self)` moves them into a ghost field.
Verified; a negative check (dropping `add`'s `frac == 0.5` precondition)
fails; runtime tests cover `acquire`/`release` and the unverified `lock`.

### Roadmap

`save` and `restore` are the first step, not the final goal. We eventually
want to verify the save side up to the controller, layer by layer.

1. **`LoadedVm::save` and `LoadedVm::restore`** (current target).
2. **Quiesce** (contracts written; see "Quiesce and save"), and on the
   restore side `restore_from_snapshot` (see "Restore from snapshot").
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
`InitializedVm::load(Some(saved_state))` (`dispatch.rs`, which builds the
`LoadedVm` and calls `restore_from_snapshot`). Both now have contracts
("Restore from snapshot").

## Recipe: verifying another device

Worked out on the PIT ("The PIT" and "The verified unit", under "Design
change: the FracGhost lives with the device"); the PIT's files are the
example to copy: `vm/devices/chipset/src/pit/mod.rs` (ghost fields, entry
points), `mod.spec.rs` (view, `saved_state()`, `wf()`, the trait impls),
`mod.proof.rs` (declarations, `StateUnitDevice`, examples).

What the PIT needed, as a checklist for each further chipset device. Items
1 to 7 are spec work; 8 and 9 come with wiring and proofs.

1. **Crate setup** (once per crate): `[package.metadata.verus] verify =
   true`, `vstd` and `closeable_mutex` dependencies, `build.rs` declaring
   the `verus_keep_ghost` cfg, a Verus-only `use vstd::prelude::*`.
2. **Declare the device's types to Verus** (`external_type_specification`):
   its state types with their fields visible (for the view), and its
   runtime glue as opaque types (`VmTimeAccess`, `LineInterrupt`, ...); its
   saved-state types (the root and the types inside it) with their fields
   visible, so that `saved_state()` can be written. Types already declared
   elsewhere (for example `VmTime` in the PIT's proof file) should move to
   their own crate's spec once a second device needs them.
3. **View and `saved_state()`**: a view of the snapshotable state (and the
   static configuration that affects `save` / `restore`), its `inv()`, and
   `saved_state()` on the view, field by field;
   `SaveRestoreSpecImpl::saved_state` is the view's. The view leaves out
   state that is not saved (like the PIT's `dram_refresh`); say so.
4. **Ghost fields and `wf()`**: Verus-only fields `ghost:
   Tracked<DeviceGhostState>` and `id: Ghost<Loc>` (placeholders in the
   constructor until the creation hand-off), and `wf()` = the view's
   invariant and `ghost.wf(id, snapshot)`.
5. **`DeviceGhost`, `LockInvariant`, `StateUnitDevice`**: `snapshot()` is
   `Saved(of(saved_state()))`; `lemma_snapshot_known` and `lemma_lock_inv`
   are usually empty proofs. `save_blob` / `restore_blob` call
   `SavedStateBlob::new` / `parse` directly (works for any derived struct
   or enum saved-state root; no per-device blob code).
6. **Entry points with contracts**: move the bodies of the device's trait
   methods (I/O handlers such as `PortIoIntercept` / `MmioIntercept`,
   `PollDevice`, `ChangeDeviceState`, `SaveRestore`) into inherent methods
   with contracts, and let the trait methods call them (Verus cannot add
   contracts to a foreign trait's impl methods). Handlers and `poll`
   require `wf()` and running; `reset` / `restore` require `wf()`, whole,
   and stopped; `save` keeps the view and the ghost.
7. **Device-specific gaps** to check for each device:
   - activity of its own to stop (`stop_device` doing work), `advance_time`
     (`advances_time()` true, like the RTC), or `quiesce_input` /
     `resume_input`: `StateUnitDevice` has no `advance_time` or input
     methods yet, and `DeviceUnit` treats `AdvanceTime` as a no-op;
   - cross-device effects: interrupt lines it drives (into the PIC /
     IOAPIC), vmtime timers, `Halt`: not modelled;
   - `restore` that changes state before failing, or normalizes the saved
     state: the contracts allow the first; the second breaks `restore`'s
     "sets `saved_state()`" contract;
   - a saved-state root using the derive's `transparent` or `prost` mode:
     needs those encoders declared.
8. **Wiring** (not yet done for the PIT): the device receives its unit's
   FracGhost half at creation, its mutex is created with `new_verified`,
   and it is driven by `DeviceUnit` instead of the shared chipset unit;
   needs the ghost types in normal builds.
9. **Proofs** (proof phase): remove `external_body` from the entry points;
   each change must update the FracGhost (it holds the whole while
   running); the external functions they call (vmtime, interrupt lines,
   logging) need specs or rewrites.

Not needed per device: the blob contract (`SavedStateBlob::new` / `parse`)
and `ProtobufSaveRestore`'s blanket impl, which are generic.

Units that are not chipset devices (the partition, vmtime, time-abi, vmbus
devices) have their own unit types and saved state, not built on
`SaveRestore` for some; each needs its own snapshot definition and unit
contract ("Unit-side contract").

## Verification approach: fractional ghost ownership

### Problem

The snapshotable state is not reachable from `LoadedVm`'s fields.
`LoadedVm.state_units` is a `StateUnits` (`vmm_core/state_unit/src/lib.rs:253`),
which holds only a channel to each unit. Each unit's real state lives in a
device object owned by that unit's own task. `StateUnits::save`/`restore` reach
the units only by RPC: `run_op` (`lib.rs:688`) sends each unit a
`StateRequest` through `state_change` (`lib.rs:843`, `send.call(...)`), and the
unit's task dispatches it to its `StateUnit::save`/`restore` (`run_unit` and
`StateRequest::apply`, `lib.rs:151, 224`).

An RPC can be reasoned about like a local call (plus a failure case), but the
caller never holds the callee's `self`: the callee's postcondition (e.g. "the
blob describes the PIT's state") talks about an object the caller has no way to
name. Treating the units as opaque and trusting their save/restore would prove
only the thin `LoadedVm` layer, which is the weakness of the earlier spec
(everything abstracted away, little proved, no bugs found).

### Approach

Use `vstd::resource::frac::FracGhost` (fractional ghost ownership). A
`FracGhost` is a proof-only value with an id; it can be split into halves and
recombined; halves with the same id agree on the value; and only the holder of
the whole (fraction 1) can change the value. See
`verus-concurrency_tutorial/my_fracghost_basics.rs` and `my_count_to_2.rs`.

- **One `FracGhost` per stateful unit**, whose value is the unit's abstract
  state (e.g. `pit@`). The unit's task holds it with the device, under the
  invariant "the ghost's value equals the device's view".
- **While running**, the unit holds the whole permission, so it may change its
  device state, updating the ghost in step. `LoadedVm` holds nothing and does
  not know the units' states.
- **On save (and later quiesce)**, the unit proves its save correct, splits
  its permission, and returns one half in its reply, keeping the other half
  tied to its device. From its half, `LoadedVm` knows:
  - the unit's state (the half's value);
  - that the state is frozen: no one holds the whole, so the value cannot
    change, and the unit's invariant ties the device to its half;
  - that the snapshot is correct: the returned blob's view equals the half's
    value.
- **On restore**, `LoadedVm` sends its half back with the blob; the unit
  recombines, restores, updates the value to the blob's view, and may split
  again.

The proof burden moves into each unit (its own save/restore code, where bugs
are plausible). What remains trusted is RPC delivery.

### Views

A `View` contains values, and a `FracGhost` is a tracked resource, so the
halves are held in a tracked field and the view is computed from their values:

```rust
// in StateUnits (or LoadedVm)
units_ghost: Tracked<Map<Seq<char>, FracGhost<UnitState>>>   // unit name -> half

// in the view
snapshot.units == units_ghost@.map_values(|g| g@)            // unit name -> state
```

The map is keyed by unit name; `SavedState.inventory` gives the order. The
snapshot part of the view is meaningful only while the VM is stopped and
`LoadedVm` holds the halves.

### Interrupt lines as resources

Units are not fully independent: during restore, `pit`, `rtc`, and each
virtio-mmio unit re-drive their interrupt lines into the PIC and IOAPIC (see
"Concurrency among state units"), changing PIC/IOAPIC state after those units
have restored. Per-unit ownership alone cannot express this.

- Model **each interrupt pin as its own `FracGhost`** (its line level), shared
  between the driving unit and the target. Concurrent drivers use distinct
  pins, so each has its own resource, matching the concurrency analysis.
- The PIC's and IOAPIC's abstract states exclude the line levels; their
  invariants relate their internal state to the current values of their pin
  resources.
- The PIC's output into the partition (LINT0, KVM `request_interrupt_window`)
  is another cross-unit effect and needs a resource of its own.

### The dependency graph as coordinator

`run_op` starts a unit's restore only after all its dependencies have
finished. That is when the resources a unit needs can be handed to it. For
example: the PIC restores and returns its half; then the PIT receives its pin's
resource, restores, drives the pin, and returns the resource with the new
value. The dependency graph is the schedule for moving ghost ownership between
units.

### Changes needed on the Verus side

Verus and `vstd` provide no channel or RPC support; the only specified
cross-task primitive is `vstd::thread::spawn` (`vstd/thread.rs:107`), whose
trusted spec relates the child's postcondition to what `join` returns. Tracked
values travel through it as ordinary captured arguments and results.

We need the same for RPC: a trusted spec on mesh RPC in which the request and
the reply each carry a `Tracked<...>`, so that the caller obtains what the
unit promises. See "Mesh RPC trusted spec" below for the design.

Tracked values are zero-sized at runtime, so nothing crosses the channel; the
trusted spec is what makes "passing" them sound. It must guarantee:

- exactly-once delivery: a ghost half is never duplicated;
- matching: the reply's ghost belongs to this request;
- on failure, the ghost may be lost, after which `LoadedVm` can prove nothing
  about that unit (acceptable on an error path).

This is our own trusted design (it could later be proposed to `vstd`). It
needs careful soundness review.

Also expected on the Verus side:

- **Async with tracked values.** Verus's async support is recent; moving
  `Tracked` values through `async` functions and closures may hit verifier
  limits or bugs (Verus already panicked on `exec_await` without vstd's `std`
  feature). Such issues may need focused Verus fixes. One has been fixed so
  far: an `async fn` without a named return lost its postconditions at
  `.await` (`verus-patches/0001-async-fn-unnamed-return-keeps-ensures.patch`),
  and annotation-style ghost parameters on an `async fn` hit an internal
  error (`0002-async-fn-ghost-params.patch`); both are described in
  `verus-patches/README.md`. Still unsupported: `async` blocks (rewritten as
  `async fn`s, code change 16), ghost outputs of an `async fn` (tracked
  `&mut` inputs are used instead), `async fn` in traits, async closures.
- **Production signatures.** The unit side must return a ghost half, but
  `StateUnit` is a trait with many implementations and the request types
  derive `MeshPayload`. The mesh RPC spec avoids changing those types by
  using wrapper functions, at the cost of small source changes at the call
  sites (listed in `SNAPSHOT-SPEC-CODE-CHANGES.md`).

### Mesh RPC trusted spec

A standalone explanation, starting from the mental model (`thread::spawn`
for RPC) and how to use it, is in `SNAPSHOT-SPEC-MESH-RPC.md`.

Now in crate `mesh_rpc_verus` (`support/mesh/mesh_rpc_verus/src/mesh_rpc_t.rs`;
first a module of `state_unit`; first drafted in
`verus-patches/verus-rpc-prototype/src/mesh_rpc_t.rs`, written against the
real `mesh` crate; the prototype copy is kept for its example and lacks the
attached ghosts).

Mesh RPC in brief: `send.call(f, input)` creates a one-shot reply channel,
builds an `Rpc(input, reply_sender)`, wraps it in a message with `f` (e.g.
`StateRequest::Save`), sends it, and returns a `PendingRpc` that the caller
awaits. The receiver reads messages from a `Receiver` and answers with
`rpc.complete(r)` (or `handle_failable`).

Tracked arguments change a function's signature, which `assume_specification`
cannot do, so the spec is a set of `external_body` wrappers that call mesh:

| Wrapper | Wraps | Contract |
|---|---|---|
| `channel(Ghost(k), Tracked(gs), Tracked(gr))` | `mesh::channel()` | both ends get the ghost constant `k`; `gs` is attached to the sender, `gr` to the receiver |
| `take_sender_ghost(&mut send)`, `take_receiver_ghost(&mut recv)` | (none) | return the attached ghost, once |
| `clone_sender(&send)` | `Sender::clone` | same constant, no attached ghost |
| `call(&send, f, input, Tracked(gin), Ghost(post))` | `RpcSend::call` | requires `request_ok` for the message `f` builds; the result carries `post` |
| `PendingCall::recv().await` | awaiting `PendingRpc` | on `Ok`, the reply and its ghost satisfy `post` |
| `recv_msg(&mut receiver).await` | `Receiver::recv` | the message and its ghost satisfy `request_ok` |
| `split(rpc)` | `Rpc::split` | returns the input; keeps the reply contract |
| `complete(rpc, r, Tracked(gout))` | `Rpc::complete` | requires the `Rpc`'s reply contract |

The rules for what crosses a channel are supplied by its message type, through
a trait we define:

```rust
pub trait MeshProtocol {     // implemented by the message type, e.g. StateRequest
    type Const;              // per-channel ghost constant, e.g. the unit's ghost ids
    type GhostIn;            // sent with a request (e.g. the caller's half on restore)
    type GhostOut;           // returned with a reply (e.g. a half on save)
    type SenderGhost;        // attached to the sender at creation
    type ReceiverGhost;      // attached to the receiver at creation
    spec fn request_ok(k: Self::Const, msg: Self, gin: Self::GhostIn) -> bool;
}
```

- The request ghost travels with the message; the reply ghost travels with
  the `Rpc`'s reply.
- `post` is the caller's expectation of the reply, chosen at `call`.
  `request_ok` for a message holding an `Rpc` states which replies satisfy
  that expectation (`rpc.reply_ok(r, gout)`), so the receiver knows what it
  must answer.
- Specs on mesh's own types (`constant()`, `spec_input()`, `reply_ok()`) come
  from traits with uninterpreted spec functions implemented for those types,
  as vstd does for foreign types (`FutureAdditionalSpecFns`).
- Verus rejects a datatype constructor used as a function value, so `f` is a
  closure: `|rpc| -> (m: StateRequest) ensures m == StateRequest::Save(rpc) { ... }`.

Example: `src/mesh_unit.rs` serves a stopped unit over a real mesh channel.
On `Save`, the caller lends its half and gets it back with a blob equal to its
value; on `Restore(blob)`, it sends its half and gets back a half whose value
is `blob`. `round_trip` proves that after save, restore of another value, and
restore of the saved blob, the caller's half equals the saved value. Three
injected bugs (wrong final claim, wrong blob on `Save`, wrong half on
`Restore`) each fail verification; `cargo test` runs it over mesh.

Attached ghosts carry each unit's initial halves from its channel's
creation to `build` (sender) and to the unit's run loop (receiver); see
"Where the halves come from". Assumption: an end whose ghost is taken was
created by `channel`, not by `mesh::channel` directly.

Open points:

- **Soundness review.** The reply ghost's type is a type parameter of
  `complete`, and `reply_ok` is uninterpreted for every type; the argument
  that a mismatched type cannot be used is in the file header.
- Not yet covered: `FailableRpc` replies (`Result<_, RemoteError>`), the
  `StateRequest` variants, the hand-off at `Stop`, and the concurrent
  `run_op`.

### Plan

1. **Stage 1, standalone prototype** (outside OpenVMM, tutorial style): a
   channel RPC with the trusted `call` spec; a unit task owning a device and a
   `FracGhost` that handles Save and Restore; a caller that receives the half
   on save, returns it on restore, and proves the round trip. Include a
   two-unit case with a shared interrupt pin, to test cross-unit effects
   early.
2. **Stage 2, OpenVMM, one unit at a time**, starting with the PIT: the trusted
   spec on `state_change` / `send.call` in `vmm_core/state_unit`, and the unit
   side on its `StateUnit` save/restore, connected to the PIT's own
   `SaveRestore`.

### Stage 1 status

Done: `verus-patches/verus-rpc-prototype` verifies (45 items, 0 errors) with
the patched Verus. It has a trusted RPC channel designed for the experiment
(sync `rpc_t.rs` and async `rpc_async_t.rs`), a single unit, and a two-unit
case where the source re-drives a shared pin into the target on restore. It
shows that FracGhost halves can be passed through a trusted RPC spec and held
across `.await`. Its requests there (`Save`, `Restore`, `Resume`, `Run`) are
test requests, not OpenVMM's `StateRequest`.

It also has the first draft of the trusted spec for the real mesh RPC
(`mesh_rpc_t.rs`) with an example (`mesh_unit.rs`); see "Mesh RPC trusted
spec" above.

### Gaps between the prototype and OpenVMM

What Stage 2 still needs, found by comparing the prototype with
`vmm_core/state_unit` and the mesh RPC code.

1. **Unsupported Verus features.** Both were checked against the patched
   Verus with small examples:
   - `async fn` in a trait: "does not yet support Opaque types in trait def".
     `StateUnit` (`vmm_core/state_unit/src/lib.rs:110`) is such a trait.
   - Async closures: "does not yet support coroutine closure types". The unit
     side answers each request with
     `rpc.handle_failable(async |()| unit.save().await)` (`lib.rs:224-245`).

   Each needs a Verus fix or a wrapper on the unit side.
2. **Trusted spec for mesh RPC.** The caller sends with
   `send.call(StateRequest::Save, input)` on a `mesh::Sender<StateRequest>`,
   which places an `Rpc<I, R>` (input plus a oneshot reply sender) inside the
   request (`lib.rs:843`). The unit reads plain `StateRequest`s from a
   `mesh::Receiver` and answers through `Rpc::handle_failable` or `complete`
   (`support/mesh/mesh_channel/src/rpc.rs`). The trusted spec, or a wrapper,
   must cover these types and say where the ghost travels. `StateRequest`
   derives `MeshPayload`, so it cannot simply gain a `Tracked` field.

   *Status:* drafted as wrappers in `mesh_rpc_t.rs` (see "Mesh RPC trusted
   spec"); not yet applied to `StateRequest` or moved into OpenVMM.
3. **One ghost type and per-unit contracts.** The prototype has one protocol
   type per channel. OpenVMM uses one `StateRequest` type for every unit, while
   the contract differs per unit. `StateUnits` holds all units' halves, so
   they need a single abstract-state type covering every kind of unit, and the
   per-unit contract must be carried by the channel (e.g. in its constant).
4. **Unit saved state is encoded inside `save`.** `LoadedVm::save` itself does
   not encode: it returns a decoded `SavedState`, and encoding the whole
   `SavedState` into a `ProtobufMessage` happens later in
   `quiesce_for_snapshot` (out of scope). But each `SavedStateUnit.state` is a
   `SavedStateBlob(ProtobufAny)` (`vm/vmcore/src/save_restore.rs:120`) that
   the unit has already encoded: `ProtobufSaveRestore` wraps the typed
   `SaveRestore::save` result with `SavedStateBlob::new` and decodes with
   `parse()` on restore (`save_restore.rs:151-160`). Relating a unit's half
   to its blob therefore needs a trusted contract on `SavedStateBlob`: an
   abstract view (the typed value it encodes), `new(x)` gives a blob whose
   view is `x`, and a successful `parse()` returns the view. Protobuf
   encoding itself is not verified. (Alternative: state unit contracts at the
   typed `SaveRestore` level and trust the blanket `ProtobufSaveRestore`
   impl.)
5. **Hand-off points must follow OpenVMM's requests.** The prototype's `Save`
   stops the unit and splits its ghost once. In OpenVMM, `Stop` is a separate
   request, and `Save` may be sent repeatedly while stopped, so a second
   `Save` would have to split a half the unit no longer holds whole. The
   hand-off probably moves to `Stop`/`Start`. The protocol must also cover
   `Reset`, `AdvanceTime`, `QuiesceInput`/`ResumeInput`, and concurrent
   `Inspect`; and a unit that saved `None` receives no `Restore`
   (`lib.rs:648`).
6. **Concurrent caller.** The prototype's caller calls units one after
   another. `run_op` (`lib.rs:688`) starts all calls concurrently, collects
   them with `join_all`, and orders them through `ReadySet` signals
   (`Arc<Ready>`). This needs a spec for `join_all` and a way to pass ghost
   resources between concurrent futures (e.g. a pin's resource from the PIC's
   restore to the PIT's) through `Ready` or a shared invariant. That is new
   trusted code.
7. **Device state behind a lock.** Chipset devices are
   `Arc<CloseableMutex<...>>`. While the VM runs, vCPU threads change them
   through MMIO and port I/O, not through the unit's task, so "while running,
   the unit holds the whole permission and updates the ghost in step" does not
   hold. Likely fix: let the ghost be stale while running; at `Stop`, the
   unit, holding the whole permission, sets the ghost to the device's current
   state. "Frozen" then relies on vCPUs being stopped (stop order), a
   trusted assumption; see "Design change: the FracGhost lives with the
   device" for how to prove it.
8. **Task spawning.** Units are spawned on `pal_async` executors, not with
   `vstd::thread::spawn`; spawning needs a small trusted spec.

## Out of scope for now

- **Guest RAM.** It has no in-memory Rust structure (it is a file-backed
  mapping). RAM will be removed from the spec rather than left unverified.
- **Protobuf encoding/decoding** of `SavedState` (`ProtobufMessage`). Each
  unit's `SavedStateBlob` is also protobuf-encoded; that is covered by a
  trusted contract, not verified (see gap 4 above).
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

## Open TODOs

### Next steps (agreed)

1. **`SavedStateBlob::new` / `parse`**: attach their contract
   (`vm/vmcore/src/save_restore_t.rs`). First probe whether Verus accepts the
   protobuf trait chain (`SavedStateRoot`, `DescribedProtobuf`, `Protobuf`,
   and `Protobuf`'s codec traits with `for<'a>` bounds), so the contract can
   go on the real functions; otherwise use per-type trusted wrappers next to
   each verified device.
   *Status:* done (2026-10-27). The contract is attached to the real
   functions (`assume_specification` in `vm/vmcore/src/save_restore_t.rs`),
   and the PIT calls them directly (its per-type wrappers are removed).
   Two Verus bugs had to be fixed (patch 0003, `verus-patches/`): a trait
   whose supertrait bound names its own associated type (`Protobuf`'s
   `DefaultEncoding<Encoding = <Self as Protobuf>::Encoding>`) could not be
   declared, and an impl whose associated type has a `for<'a>` bound
   panicked Verus. Declared to Verus, without methods: `SavedStateRoot`,
   `DescribedProtobuf`, `Protobuf`, `DefaultEncoding`, the four codec
   traits, `DescribeMessage`, `NoResources`, and, for a derived struct to
   count as a `SavedStateRoot`, the struct encoder `TableEncoder` and the
   traits the derive implements for it (`StructMetadata`,
   `StructEncodeMetadata`, `StructDecodeMetadata`, `DescribeTable`). Also
   `RestoreError`'s `From<payload::Error>` (no contract). Negative checks:
   dropping `new`'s or `parse`'s postcondition makes the PIT's `save_blob`
   / `restore_blob` fail. Follow-ups, also done: enum saved-state roots
   (`OneofEncoder` and its traits declared; example enum root verified);
   layer 2, `ProtobufSaveRestore`, attached and its blanket impl verified
   ("Unit-side contract"); and a third Verus fix in patch 0003 (an
   undeclared type in a declared trait's bound is an error, not a panic).
   Each further device needs similar per-device work; see "Recipe:
   verifying another device".
2. **`InitializedVm::load(Some(_))`** (`dispatch.rs`): its contract. It
   builds the `LoadedVm` (collecting the halves in `make_ids`) and calls
   `restore_from_snapshot`.
   *Status:* done: contract stated (`external_body`), with the `None` case
   too, and checked against `restore_from_snapshot`'s by
   `lemma_load_restores` (see "Restore from snapshot"). Added
   `LoadedVmView::time_units` for it.
3. **Replace the RPC calls with the mesh RPC wrapper**
   (`SNAPSHOT-SPEC-MESH-RPC.md`): `state_change` / `run_op` on the caller
   side (in `verus!` style, as on branch `system-proof/run-op-proof-attempt`)
   and `StateRequest::apply` on the unit side (without async closures).
   Needed for the proofs; the current direct mesh calls are correct but give
   Verus no contract and carry no ghosts.
   *Status:* done (2026-10-27). Every state-unit request and reply goes
   through the wrapper: the caller side (`call_unit`, used by `run_op`,
   `start_with_rollback`, and `quiesce_for_save`) sends with `call` and
   waits with `PendingCall::recv`; the unit side receives with `recv_msg`
   (`run_unit`, `run_async_unit`) and replies with `split` / `complete`
   (`StateRequest::apply`, rewritten without async closures; it behaves as
   `Rpc::handle` / `handle_failable` did). Units with their own receive
   loops (the chipset unit, among others) still receive with
   `recv.next()` but reply through `apply`. In this unverified code the
   ghosts are placeholders (`Tracked::assume_new()`), erased at runtime;
   passing `StateUnits`' real halves comes with verifying `run_op`. The
   verified unit (`DeviceUnit`) uses the wrapper with real ghosts.
   Also done (generic change, 2026-10-27): the wrapper moved into its own
   crate, `mesh_rpc_verus` (`support/mesh/mesh_rpc_verus`), with a
   `call_failable` counterpart (`PendingCall::recv_failable`); the other
   two message types on the snapshot and restore path use it for their
   snapshot requests, both ends: `VmRpc` (controller to worker:
   `QuiesceForSnapshot`, `ResumeAfterFailedSnapshot`,
   `ReleaseSnapshotBoundary`) and `PartitionRequest` (`StopVps`;
   `Snapshot`'s `StopVpsAtIoBoundary`, `CheckTimers`, `AdvanceLapic`,
   `OmitSavedTsc`); `vmm_core` is now Verus-enabled for this. Their
   protocols are placeholders (`request_ok` uninterpreted: verified code
   can neither send them nor learn from them).
   TODO, for whoever verifies those layers (quiesce and restore for
   `PartitionRequest`, the controller for `VmRpc`): design their
   protocols, that is, what each reply promises. This needs views that do
   not exist yet (e.g. the partition's timer state for `CheckTimers`; the
   worker's snapshot, seen from the controller, for `QuiesceForSnapshot`).
4. **Consider a small source change to unit registration**: have
   registration hand each unit's `StateUnits` half back explicitly (through
   `&mut StateUnits`, or returned by the unit builder) instead of attaching
   it to the `Sender` and registering through `&StateUnits`. This would
   remove the registry axioms (`registry_t.rs`) and most of the registry's
   lock invariant, at the cost of changing the registration API and its
   callers (the chipset builder, `vmbus_unit`, `partition_unit`, vmtime,
   time-abi).
   *Status:* done (2026-10-27), differently: registration stays through
   `&StateUnits` (no API change); instead the halves are kept with the
   registry, behind its mutex, and registration and removal go through
   verified functions, so a lock invariant ties the registry to
   `StateUnits`' ghost state and both registry axioms are gone. Remaining
   trusted fact: no registration during a `&mut StateUnits` operation. See
   "Where the halves come from", item 3.
5. **Validate on one unit first: the PIT, end to end**, with the FracGhost
   living with the device ("Design change: the FracGhost lives with the
   device"): tie its `saved_state()` to its real fields, keep its half in
   the device object inside its `CloseableMutex`, verify its `SaveRestore`
   `save` / `restore` and its I/O handlers against its `wf()`, and its
   request loop against the request protocol. This tests whether the
   unit-side obligations are tractable before more layers are added.
   *Status:* done: the PIT's spec and contracts, the shared ghost field
   (`DeviceGhost`), its `StateUnitDevice` implementation, and the verified
   unit (`DeviceUnit::handle`, `poll`) with its run loop ("The PIT" and
   "The verified unit", under "Design change: the FracGhost lives with
   the device"). This answers the step's purpose for the spec design: the
   design verifies at the unit level, and checking it changed the spec
   (the frame missing from the generic `SaveRestore` contract,
   `lemma_snapshot_known`, `DeviceGhost` for generic and `dyn` code, the
   separate verified unit, locking without closing for polling).
   TODO, kept here because a proof may still require a spec change:
   - **Prove the PIT's code bodies** (handlers, `poll`, `start`, `reset`,
     `save` / `restore`; now `external_body`). Possible spec changes:
     `restore` changes the timers before it can fail; the handlers depend
     on `vmtime`, whose state is not in the PIT's view; Verus's overflow
     checks in the timer arithmetic.
   - Normal-build support, so `DeviceUnit` can run and be tested.
   - Wiring the PIT to `DeviceUnit` in the chipset builder, with its half
     handed over at creation.
   For further devices, the steps are in "Recipe: verifying another
   device".

### Other open items

A summary of the other open items recorded in the sections above, most
important first.

1. **Prove the contracts.** Every contract above has an unverified body
   (`external_body`); they are listed in `SNAPSHOT-SPEC-TRUSTED.md` as
   assumed until verified.
2. **Next layer**: the controller
   (`VmController::handle_guest_snapshot_request`), which needs a decision
   on the encoded `ProtobufMessage` (roadmap item 3). (`InitializedVm::load`
   now has a contract.)
3. **Clock restore**: specify the clock units' restored state, and fix the
   partition's stale view after the TSC set and LAPIC advance ("Restore from
   snapshot").
4. **Unit side**: for the production units, attach the `StateUnit` and
   request-loop contracts (blocked on async trait functions;
   `StateRequest::apply` no longer uses async closures; the blob layers,
   `SavedStateBlob` and `ProtobufSaveRestore`, are attached); the units not built on
   `SaveRestore`; cross-unit interrupt effects ("Unit-side contract"). For
   the PIT, see "Next steps (agreed)", item 5.
5. **Other `ProtobufSaveRestore` implementations**: `ChipsetDeviceProxy`,
   `BoundVp`, `ErasedChipsetDevice` are trusted to meet its contract (the
   blanket impl for `SaveRestore` types is verified).
6. **Quiesce path**: the time capture, and `quiesce_for_save`'s error cases
   ("Quiesce and save").
7. **Pending proof**: `StateUnits::lemma_view_inv`.
8. **Convention TODOs**: `SavedStateUnit`'s sequence helpers, and the
   trusted wrapper types' spec functions ("Convention TODOs").
9. **Soundness review** of the mesh RPC wrapper (`mesh_rpc_verus`,
   `src/mesh_rpc_t.rs`; the reply-ghost type argument). (The registry
   axioms are gone.)
10. **Hard-coded `"partition"`** in `LoadedVm`'s view.
11. **Unit removal during an operation**: a unit can be removed
    (`UnitHandle`'s `Drop`) while a `StateUnits` operation runs; the specs
    of `stop`, `save`, `restore` (which say "same units") must allow it
    ("Where the halves come from", item 3).

