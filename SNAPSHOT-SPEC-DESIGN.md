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
- **Trait-level well-formedness** follows the same convention: a trait
  that generic code uses to reach a type's `wf()` names it `<role>_wf()`
  (e.g. `device_wf()` on `SaveRestore`'s spec extension), or `wf()` where
  no clash with the type's inherent `wf()` is possible (`StateUnitSpec::wf`
  on units). A predicate that takes arguments is not called `wf` (e.g.
  `DeviceGhostState::follows_rule(id, snapshot)`).
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
| `quiesce_and_save` | `wf` | `wf`; inventory and boundary unchanged; on `Ok((saved, time))`: stopped, `recorded`, `saved.wf()`, `saved@ == final@.snapshot`; on `Err(Rejected)`: nothing changed; on `Err(RollbackSafe)`: stopped and `recorded` (the quiesce succeeded) |
| `quiesce_for_snapshot` (the `QuiesceForSnapshot` handler) | `wf` | as `quiesce_and_save`, and on `Ok(response)`: `response.saved_state` encodes a `SavedState` whose view is the snapshot (`ProtobufMessageView::of`), and `response.state_unit_names` is the inventory |
| `StateUnits::quiesce_for_save` (`vmm_core/state_unit/src/quiesce.rs`) | `wf`, running | `wf`; same units and inventory; on `Ok`: stopped, `all_recorded`; on `Err(e)`: still marked running, and `e@.uncertain` |
| `capture_time_abi` | `wf` | `wf`; the view is unchanged |
| `StateUnits::resume_after_failed_save` | `wf` | `wf`; same units and inventory; on `Ok`: it was stopped and is running |
| `resume_after_failed_snapshot` (rollback) | `wf` | `wf`; inventory unchanged; on `Ok`: it was stopped, and is running with no boundary active |
| `establish_snapshot_boundary` | `wf` | `wf`; only the boundary may change: an active boundary stays active, a new one only when it returns `true` |
| `release_snapshot_boundary` | `wf` | `wf`; only the boundary changes; on `Ok`, none is active |
| `handle_snapshot_boundary` | `wf` | `wf`; inventory unchanged; on `false` (the worker must stop), the VM is stopped |

**The error paths.** Every `quiesce_for_save` failure is *uncertain*: a unit
fails only when its `Stop` fails or times out (uncertain), or when a unit
depending on it did not stop, a chain that starts at an uncertain unit. So
`quiesce_and_save` returns `RollbackSafe` only after a successful quiesce
(from the time capture or `save`; `quiesce_for_snapshot` adds the RAM flush
and the command line), when every unit is stopped and recorded, which is
exactly what rollback (`resume_after_failed_save`) checks. After
`Uncertain`, nothing beyond `wf` is promised; the VM must be terminated.
(`QuiesceForSaveError`'s view, `uncertain`, is uninterpreted and tied to its
field by `has_uncertain_state`'s trusted contract.)

**The two running flags** are one: `LoadedVm::wf()` requires
`self.running == self.state_units@.running` (every function that changes one
changes the other before it returns).

**The snapshot boundary** is in the view's runtime part,
`runtime.snapshot_boundary` (from `SnapshotBoundary`'s view, uninterpreted:
trusted to mean its vCPU stop guard is held). Establishing and releasing it
change nothing else in the view (`QuiesceInput` / `ResumeInput` carry no
ghost); `quiesce_and_save` leaves it as it is.

**The encoded response.** `ProtobufMessageView::of(value)` is the abstract
value of a `ProtobufMessage` encoding `value` (`encoded()`, a spec function
of `ProtobufMessageAdditionalSpecFns`, since `View` cannot be implemented
for mesh's type outside mesh); the trusted contract
(`openvmm/openvmm_defs/src/rpc_t.rs`) attaches `ProtobufMessage::new`'s
encoding and states `of` injective, as for blobs.

These compose: after `quiesce_for_save` succeeds the units are stopped and
recorded, `capture_time_abi` changes nothing, so `save`'s precondition holds
and its result is the snapshot at the quiesce point.

Concerns:

- **The time capture is unconstrained**, by decision: guest time is out of
  scope for now ("Out of scope for now"). `TimeCapture` (time contract, CPU
  profile) and `record_vm_time_cut` are not specified.
- **`quiesce_for_save` sends `Stop` through the unverified `state_change`
  path**, with its own dependency order, timeout, and cancellation; the
  contract assumes it records each unit's state as the request protocol's
  `Stop` does. (Its error cases are now specified; see "The error paths".)
- **`capture_time_abi` is trusted not to change the view**: it sends
  `CheckTimers` to every VP (`check_one_shot_timers`) and reads time
  records from the partition.
- **After `quiesce_and_save` the VM stays stopped**; resuming or ending it
  is the caller's (controller's) business.

### Restore from snapshot

`LoadedVm::restore_from_snapshot` (`dispatch/restore.rs`) is
`begin_snapshot_restore`, then `restore`, then `finish_snapshot_restore`.
For a time ABI restore, `begin` has the partition drop the VPs' saved TSCs
(`omit_saved_tsc`), and `finish` runs the clock restore (`time_abi_restore`:
synchronized TSC set, LAPIC timer advance, `StateUnits::advance_time` for VM
time and the RTC) and holds the VPs stopped until the VM first starts.

The clock restore changes only guest time (the partition's TSCs and LAPIC
timers, VM time, the RTC's clock), which is out of the spec's scope ("Out of
scope for now", guest time): the units' views leave it out (the
time-keeping units' saved-state views have no time fields, and a blob is
determined by its saved state's view, `of_view`). So in the view a restore gives back
exactly the saved state, for every unit, also after a time ABI restore; the
clock restore changes nothing in the view. (Until 2026-10-27 the view named
the units the clock restore changes, `time_units` and `clock_units`, and the
contracts excluded them; both are gone.)
`StateUnitsView::advances_time` records the units registered with
`advances_time()` (the ghost registry records the flag at collection).

Contracts (bodies `external_body`):

| Function | Requires | Ensures |
|---|---|---|
| `restore_from_snapshot(saved)` | `wf`, stopped | `wf`; runtime and inventory unchanged; on `Ok`: the units are as `restore` leaves them (`old.units.union_prefer_right(saved@.units)`), and the saved inventory is empty or the VM's |
| `begin_snapshot_restore` | `wf` | `wf`; view unchanged |
| `finish_snapshot_restore` | `wf`, stopped | `wf`; runtime unchanged; snapshot unchanged (the clock restore changes only time) |
| `time_abi_restore` (`dispatch/time_abi.rs`) | `wf`, stopped | `wf`; runtime unchanged; snapshot unchanged |
| `StateUnits::advance_time` (`vmm_core/state_unit/src/advance_time.rs`) | `wf`, stopped | `wf`; same `advances_time`, inventory; units unchanged |
| `InitializedVm::load(saved, .., snapshot_restore)` (`dispatch.rs`) | | on `Ok(vm)`: `vm.wf()`; stopped. With `saved` `None`: no unit recorded (`snapshot.units` empty), and no clock restore was pending. With `Some(state)`: the units have exactly `state@.units`; the saved inventory is empty or the VM's |

`load`'s contract follows from the fresh VM it builds and
`restore_from_snapshot`'s: `LoadedVmView::lemma_load_restores`
(`dispatch.proof.rs`, verified) derives it from "no unit recorded" and
`restore_from_snapshot`'s `Ok` result (a fresh VM records nothing, so the
union with the saved units is the saved units). Negative check: claiming
the restored VM records nothing fails. The fresh-VM facts themselves are
part of `load`'s unproved body: `make_ids` collects every unit's half,
recording `Unknown`.

They compose: `begin` changes nothing, `restore` gives the union, and
`finish` changes nothing in the view. All `StateUnits` operations also keep
`advances_time`.

Concerns and TODOs:

- **The clock units' restored time is not modelled**, by decision: guest
  time is out of scope for now ("Out of scope for now"). The TSC set and
  the LAPIC advance write the partition's VPs directly, outside the request
  protocol, and `omit_saved_tsc` makes the partition's `restore` skip its
  saved TSC; both change only time, which the view leaves out, so the
  partition's record stays true. (This resolves the earlier "partition's
  stale view" concern.)
- **`SnapshotRestore`'s view is uninterpreted** (trusted to mean
  `time_abi.is_some()`), because `SnapshotRestore` has private fields.
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
unit). So a device's view, and its `saved_view(view)`, should be defined over
what it owns.

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
   *Extended 2026-10-27 (user):* for state created before its unit's
   channel (the time-ABI MSRs, chipset devices), the state's creator
   creates the FracGhost (still `Unknown`), keeps one half as the state's
   share, and passes the other to `state_unit::unit_channel_for`, which
   attaches it to the sender (the receiver carries none); the unit is
   spawned with `UnitBuilder::spawn_with_channel`. (Choosing
   `unit_channel()` earlier was for convenience: the device-created
   FracGhost works for restore and registration too, the half travelling
   through registration.) Units created with their channel keep the
   `unit_channel()` path.
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
5. *Done (Verus only; not yet wired or tested).* The unit loop. Decided 2026-10-26, as a first step (the PIT as the
   first device worked end to end): the shared chipset unit
   (`ArcMutexChipsetDeviceUnit`, used by about 56 device types) stayed as
   it was, trusted, for now. (That interim step has ended, 2026-10-27:
   every unit's share lives with its state, under its lock; see
   "Production units: the contract on `StateUnit`".) a separate verified unit, `DeviceUnit<T>`
   (`vmm_core/state_unit/src/device_unit.rs`, Verus only for now), is for
   devices that implement `StateUnitDevice` (for now, the PIT): verified
   `handle` and `poll`, and the unverified run loop. See "The verified
   unit" below.

Alongside: a survey of cross-device changes (lines re-driven into the
PIC/IOAPIC at restore, vmtime, `Halt`, the time-ABI writes to the
partition), which need modelling.

#### The PIT

*Superseded 2026-10-27* ("Chipset devices: the share in the device's
lock"): the PIT's ghost field, `DeviceGhost`, and its lock invariant are
retired; its view, `saved_state()`, and entry-point contracts stay. Kept
below as the record of the first device worked through. (Later, 2026-10-27,
"`SaveRestore`'s contract on views": the trait's `saved_state()` became
`saved_view(view)`, the view of the saved state, at first named
`blob_of(view)`; the PIT's `saved_view` is `view.saved_state()`.)

- **The shared ghost field** (`vm/vmcore/src/device_state.spec.rs`): a
  device keeps one Verus-only field of type `DeviceGhostState` (`frac`, its
  share of the FracGhost; `running`). `DeviceGhostState::follows_rule(id, snapshot)`
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

*Superseded 2026-10-27*: `StateUnitDevice` and `DeviceUnit` are retired;
the PIT runs behind `ArcMutexChipsetDeviceUnit`, whose share is kept in the
device's lock ("Chipset devices: the share in the device's lock"). Kept
below as the record.

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

1. **Device: `SaveRestore`** (`vm/vmcore`). *Attached.* Stated on views,
   with no blobs (see "`SaveRestore`'s contract on views"): `save` keeps
   the device's view (`final(self)@ == old(self)@`) and returns a state
   whose view is `saved_view(old(self)@)`; a successful `restore(state)`
   makes `saved_view(final(self)@)` equal to `state@`. `saved_view(view)` is
   a spec function each specified implementation defines. Given in
   `vm/vmcore/src/save_restore.spec.rs` with `external_trait_specification`
   and `external_trait_extension(SaveRestoreSpec via SaveRestoreSpecImpl)`
   (vstd's pattern for `Iterator`); the views come from the trait's
   Verus-only supertrait `SaveRestoreView: View` and `SavedState`'s bound.
   Example: `ExampleDevice` (`save_restore.proof.rs`) verifies.
2. **Blob: `ProtobufSaveRestore`** (blanket impl for `SaveRestore` types).
   *Attached and verified* (2026-10-27): `external_trait_specification` +
   `external_trait_extension(ProtobufSaveRestoreSpec via
   ProtobufSaveRestoreSpecImpl)` in `vm/vmcore/src/save_restore.spec.rs`,
   with `saved_blob()` the encoded snapshotable state; the blanket impl
   (`save_restore.rs`, now `verus_verify`, its bodies rewritten without the
   closure and `?`) is verified against it, with `saved_blob() ==
   of_view(saved_view(self@))` (`save_restore.proof.rs`): the only place a
   device's saved state meets its blob. Negative check: a
   `restore` that ignores the decoded state fails. The other
   implementations (`ChipsetDeviceProxy`, `BoundVp`,
   `ErasedChipsetDevice`) are trusted to meet it. `save` returns a blob with view
   `of_view(saved_view(old@))`; a successful `restore(blob)` leaves
   `of_view(saved_view(self@)) == blob@`. Follows from layer 1 and the `SavedStateBlob`
   contract.
3. **State unit: `StateUnit`** (async trait, `vmm_core/state_unit`).
   *Stated, not attached.* Through the unit's `UnitSnapshot`: `save` returns
   `Ok(Some(blob))` with `Saved(blob@)`, or `Ok(None)` with `NoSavedState`,
   the unit's current snapshot; `restore(blob)` on success makes it
   `Saved(blob@)`; `stop` leaves the unit stopped with its state stable;
   `reset` and `advance_time` may change it. For a chipset device
   (`ArcMutexChipsetDeviceUnit`, `vmm_core/vmotherboard`), the snapshot is
   `NoSavedState` if `omit_saved_state`, else `Saved(device.saved_blob())` of
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
- **`async fn` in traits** (Verus: "Opaque types in trait def"): resolved
  2026-10-27 by Verus patch 0004 (`verus-patches/README.md`). Checked with a
  probe on `StateUnit` (not committed): declared with
  `external_trait_specification` and an `external_trait_extension` spec
  function, a contract on `save` holds for a generic caller and is enforced
  on an impl inside `verus!`. An impl outside `verus!` is not checked, so
  each production unit's impl must move into `verus!` (or be listed as
  trusted).
- **Async closures** in `StateRequest::apply` (`rpc.handle_failable(async
  |..| ..)`) block layer 4; `apply` would be rewritten without them (a
  source change).
- **`run_async_unit`** handles concurrent `Inspect` requests while a state
  change runs, with the unit as `&T`: interior mutability, and the
  assumption that inspects do not change saved state.
- **The device's saved state is a function of its view**
  (`saved_view(view)`), resolved 2026-10-27.
- **Deterministic `save`**: the contract makes `save`'s result (its view)
  a function of the device's view. A device whose saved state includes
  something else (counters that `save` itself advances) must model it in
  its view, or does not meet the contract. Time is not such a thing: it is
  not in saved-state views.
- **`restore(s)` then `saved_view(self@) == s@`**: a device that
  normalizes or only partly restores a saved state does not meet the
  contract as stated, unless the normalized part is outside the saved
  state's view (the RTC's `time_valid`: time).
- **`save` takes `&mut self`** (e.g. `WrappedVp::save` flushes async
  requests); the contract says the view is unchanged.
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

### Production units: the contract on `StateUnit`

Decided 2026-10-27 (user): every production unit holds its own FracGhost
share, following the rule ("Design change: the FracGhost lives with the
device"), and every path that changes a unit's state gets proof
obligations; nothing about the units is trusted to save spec work. The
contract is written once, on the `StateUnit` trait, so that every unit is
specified by it. (Rejected: keeping the ghost in `run_unit`'s task and
trusting each unit's methods and that nothing changes a stopped unit.)

**Shape** (decided 2026-10-27; Verus patch 0005):

- The contract is the request protocol, on `StateUnit`'s methods: each
  method takes the FracGhost halves as ghost inputs (`#[verus_spec(with
  Tracked(..))]`) and its contract says what it does with them, as the
  table in "Request protocol" does for the replies. It names nothing about
  where the unit keeps its own share, so it fits every unit: a field (the
  partition, vmtime, input), inside the device's `CloseableMutex` (the
  chipset units, the PIT), or behind `&X` (the vmbus units).
- Each unit's impl proves it meets the contract, keeping its own share
  where its state is, with an invariant tying the two; so does every other
  path that changes its state.
- `StateRequest::apply` / `run_unit`, verified once for every unit, pass
  the halves from the request to the method and back to the reply.
  `DeviceUnit` becomes one instance of the general scheme.
- Written in `lib.rs`, on the trait itself (annotation style, as `lib.rs`
  already does for `StateUnits`' methods): `with` works on a trait's own
  methods, not through `external_trait_specification`.

**Status** (2026-10-27): the contract is written (`StateUnit` in
`lib.rs`, `StateUnitSpec` in `lib.spec.rs`); `state_unit` verifies (14
verified, including the three default bodies, against it); negative
check: a default `advance_time` that drops the half fails. The ten
production impls are converted (code change 39): each takes the ghost
input, its bodies are `external_body`, and its `StateUnitSpec` is
uninterpreted (listed in `SNAPSHOT-SPEC-TRUSTED.md`). `StateRequest::apply`,
`run_unit`, and `run_unit_step` are verified (code change 40): `apply`
passes each request's half to the unit's method and replies with what
comes back; `Stop` assumes the unit runs (the trusted fact, as in
`DeviceUnit::handle`). Negative checks, each failing: `Save` replying
`Ok(None)`; `Stop` without the assumption; `Restore` replying `Ok`
without restoring; `run_unit` without requiring its own channel.
Request loops (code change 41): `KeeperUnit`, `TimeAbiUnit`, the chipset
unit, and the line sets run through `run_unit`;
`ArcMutexChipsetDeviceUnit::run` is verified, with its `select!` and
its poll in two trusted helpers (negative check: without requiring its
own channel, its loop invariant fails). All loops go through the new
verified `handle_request`. Next: the vmbus units' `run_async_unit` (with
concurrent inspects), the input distributor's loop (state requests merged
with input events, which change its state: other mutation paths), and
each unit's `StateUnitSpec` definition (where its share lives, its `wf`).

Per-unit `StateUnitSpec` (code changes 42, 43): defined for the line sets,
the chipset unit, the input distributor (no saved state), and the time
keeper; each keeps its share in a ghost field of the unit, which only the
unit's task changes. The time keeper's saved state is all time, so its
view is only its running flag and its snapshot one constant,
`Saved(VmTimeKeeperView::saved_blob())` (vmtime's saved state has the
view `()`, so all its blobs are one, `of_view(())`).

Decided 2026-10-27 (user): a unit whose state is shared with other code
that changes it keeps its share with that state, under the state's own
synchronization; not in the unit, with a trusted "nobody changes a stopped
unit". The time-ABI unit is the first (code change 43): its saved state is
one atomic MSR value (`TimeAbiMsrs::tsc_invariant_control`, a control
setting, not time), which VPs write; in Verus builds it is vstd's atomic
with ghost (`vstd::atomic_ghost::AtomicU64`), whose invariant
(`TscControlPred`) ties the share to the value. The unit's id is the
atomic's key (as `DeviceUnit`'s is its mutex's key). The VP write path
must change the value and the share together, which needs the whole,
that is, the unit running (as for the PIT's handlers). Still
uninterpreted: the vmbus units, `PartitionUnitRunner`.
(`ArcMutexChipsetDeviceUnit`: see "Chipset devices: the share in the
device's lock".)

Proof hint (time-ABI): `TimeAbiMsrs::load_control` / `store_control` now
reach the inner `PAtomicU64` (`.patomic`) with a made-up permission
(`Tracked::assume_new()`), bypassing the atomic's invariant; they are
`external_body` placeholders. Rewrite them with vstd's `atomic_with_ghost!`,
which opens the invariant, performs the load or store, and updates the
share in the same step (the store needs the whole, so the unit running).

Verus bug (not patched): unverified code that returns a verified `async
fn`'s future without awaiting it (`spawn(.., |recv| unit.run(recv))`)
makes rustc panic in Verus builds ("internal compiler error:
OpaqueTypeStorage ... ProvisionalHiddenType ... ?0t", reported at the
enclosing `async fn`, `add_dyn_device`). Workaround: await it inside an
`async` block.
`InspectMut` is declared without a contract: `apply`'s `Inspect` path
(and `run_async_unit`'s concurrent inspects) can change a unit outside the
contract, so `inspect_mut` needs one too (keeps `wf()`, `id()`,
`running()`). Open. Verus panicked ("compute_dyn_compatibility: missing
trait") instead of reporting the undeclared supertrait; a Verus bug, not
yet patched. Verus patch 0005 gained a third fix on the way: a
`cfg_attr`'d `verus_spec` in a `verus_verify` trait impl got a second,
empty `verus_spec`.

**Why not a ghost field reached through a supertrait** (the first
proposal, probed and dropped 2026-10-27): its contract was stated on the
unit's ghost field, `self.ghost()`, so it fit only units whose state is in
a field of the unit, not those whose state is in a `CloseableMutex` or
behind `&X`.

**Verus patch 0005** (`verus-patches/README.md`): ghost inputs on trait
methods (the macro made both the original and the verified copy only for
functions) and async default methods (0004 covered only methods without a
default body). Tested in Verus's own tests, sync and async, with an impl,
a generic caller, and negative checks.

**Consequences and open questions:**

- **All implementors at once.** In the Verus build, every impl of
  `StateUnit` must provide the methods' verified copies (patch 0005: an
  impl without `with` does not compile there), so every crate with a
  production unit becomes Verus-enabled together: vmm_core (already),
  vmotherboard (not yet), openvmm_core (`TimeAbiUnit`). OpenHCL's
  `VmbusRelayUnit` (underhill_core) is outside the build of the verified
  crates; `state_unit`'s test units are built only for tests.
- **Units implemented for `&X`**: resolved 2026-10-27 (user): they now
  implement `StateUnit` for `X`; see "The vmbus units" below.
- **Default method bodies** (`quiesce_input`, `resume_input`,
  `advance_time` return `Ok(())`): verified with the trait (patch 0005
  supports async default methods, with ghost inputs).
- **Per unit**: where it keeps its share (Verus-only), its snapshot
  definition, its invariant, its `StateUnit` impl verified, and contracts
  on every other path that changes its state (for the partition: the VPs; for vmbus: the
  channels; for vmtime: the timer updates; for the chipset units: the
  devices' I/O handlers, as for the PIT).

### The vmbus units

**What they are.** The vmbus server's unit (`VmbusServerUnit`), the vmbus
channels' units (`ChannelUnit<T>`, `SimpleChannelUnit<T>`), and OpenHCL's
vmbus relay unit (`VmbusRelayUnit`) are proxies: their state lives in
another task (the vmbus server's task; the channel's task, which owns the
device `T`; the relay's task), which guest vmbus traffic also changes. Each
unit's methods send `Start` / `Stop` / `Save` / `Restore` / `Reset` over
that task's own internal channel (`VmbusRequest`,
`vmbus_channel::channel::StateRequest`, the relay's `TaskRequest`), and
inspecting the unit sends an `Inspect` request over the same channel.

**The problem.** They implemented `StateUnit` for `&X`, so that
`run_async_unit` could serve inspects while a state change was in flight
(the state change held `&X`, the inspect another `&X`). But
`StateUnit`'s contract has `running()` change (`start` ensures it; `stop`
requires it and ensures its negation), and nothing in `X` can change
through `&X`, so the contract could not be met.

**Decided 2026-10-27 (user): option A.** They implement `StateUnit` for `X`
(the state change holds `&mut X`), and `run_async_unit` serves concurrent
inspects through a separate *inspector*. Options considered:

| | How | Cost |
|---|---|---|
| A (chosen) | `StateUnit` for `X`; concurrent inspects through an inspector that does not borrow the unit | an executable change in `run_async_unit` and the four units, and an `inspector()` on four handle types; inspect behavior unchanged |
| B | keep `&X`, with the running flag behind interior mutability | a `running()` that is a function of a lock's contents cannot be stated as a spec function; would need tokens |
| C | drop `running()` from the `StateUnit` contract | the one trusted fact ("`Stop` arrives only while running") would be copied into every unit and owning task |

**The executable change** (code change 46; also listed in "Executable code
changes (normal builds)"):

- `state_unit`: new trait `InspectConcurrently` (`type Inspector:
  Inspect; fn inspector(&self)`). `run_async_unit`'s bound changes from
  `for<'a> &'a T: StateUnit` to `T: StateUnit + InspectConcurrently`; it
  takes the inspector once, before its loop, and passes the unit as `&mut
  T`. `StateRequest::apply_with_concurrent_inspects` takes `(unit: &mut T,
  inspector: &impl Inspect, recv)` instead of `(unit: &mut &T, recv)`: an
  `Inspect` request with no state change in flight goes to the unit, as
  before; one that arrives while a state change is in flight goes to the
  inspector (before: to the unit through another `&T`).
- The four handle types gain `inspector()`, returning a new type that holds
  a clone of the handle's internal sender and inspects with the same
  attribute as the handle (`#[inspect(flatten, send = "...::Inspect")]`):
  `VmbusServer::inspector` (`VmbusServerInspector`, `vmbus_server`),
  `ChannelHandle::inspector` and `SimpleDeviceHandle::inspector`
  (`ChannelInspector`, `vmbus_channel`), `HostVmbusTransport::inspector`
  (`HostVmbusTransportInspector`, `vmbus_relay`).
- The four units implement `StateUnit` for `X` (same method bodies; they
  call the same `&self` methods of their handles), `InspectMut` (forwarding
  to their existing `Inspect`), and `InspectConcurrently` (their handle's
  inspector).

**Why behavior is unchanged.** Before and after, an inspect of these units
sends the same `Inspect` request to the same task, which answers it; only
the sender it goes through is a clone, and a mesh sender's clones share one
queue (`mesh_channel_core`'s `SenderCore` is an `Arc<Queue>`), so the order
of requests on the task's channel is the same. Inspects that arrive during
a state change are still served during it, not after. New test
`state_unit::tests::concurrent_inspect::test_inspect_during_state_change`:
while a unit's `start` is blocked, an inspect is answered, by the
inspector; after `start` completes, an inspect goes to the unit.

**Spec.** `run_async_unit` and `apply_with_concurrent_inspects` have the
contracts of `run_unit` and `apply` (`external_body`).

**Where the vmbus units' shares live** (plan agreed 2026-10-27): with their
state, in the owning tasks; the proxies hold no share and pass the halves
over their internal channels, whose protocols mirror the outer request
protocol. One FracGhost per unit; the outer channel (`StateUnits` to the
proxy, from `unit_channel_for`) and the inner one (the proxy to the owning
task) both have its id as their constant. The owning task's creator
creates the FracGhost (creator-made). Inside the owning tasks, only the
state-request handler and the task's invariant are specified; the
guest-traffic handlers are proof-phase work. OpenHCL's relay
(`vmbus_relay`) is out of scope (OpenHCL is not on the verified microVM
path). Done for the channels (`vmbus_channel`, code change 47); the server
(`vmbus_server`) is next.

**The channels (`vmbus_channel`, code change 47).**

- *The device's spec*: `VmbusDevice`'s spec extension (`channel.spec.rs`)
  declares `saved_blob()` (`None` if the device does not support
  save/restore) and `device_wf()`, defined per device when it is specified
  (none is yet); `UnitSnapshot::from_saved` (vmcore) turns `saved_blob()`
  into the device's snapshot.
- *The internal protocol*: `MeshProtocol for StateRequest` (the channel's
  private request type), the outer protocol request by request (`Start`
  gives the half, no reply; `Stop` returns a known half; `Save` lends;
  `Restore` / `Reset` give one and get one back; `Inspect` none).
  `request_ok` is closed (`StateRequest` is private).
- *The task*: `Device` (the task's state) is opaque to Verus (private fields
  of undeclared types), with an uninterpreted view `DeviceView { running }`
  (trusted: `state is DeviceState::Running`, the task handles the guest's
  requests only then, and queues them otherwise). The task's share is a
  tracked value it owns, threaded through its loop and handler (with its
  state: the one task owns both `Device` and the device).
  `Device::keeps_share(share, channel)`: the share follows the rule for
  `from_saved(channel.saved_blob())`, the device is `device_wf()`, and
  while running the task holds the whole share.
  `Device::handle_state_request` takes the request's half and the share as
  ghost inputs, answers by the internal protocol, and keeps `keeps_share`
  (`external_body`; its `Stop` relies on the trusted fact that `Stop`
  arrives only while running, as `apply`'s does).
- *The handles*: `GenericChannelHandle`, `ChannelHandle<T>`,
  `SimpleDeviceHandle<T>` have `id()` (the internal channel's constant) and
  contracts on `start`, `stop`, `reset`, `save`, `restore`, with the half
  as a ghost input, mirroring the outer protocol (`external_body`).
  `GenericChannelHandle::id` and `ChannelHandle::id` are uninterpreted,
  trusted to be the internal sender's constant (the types are opaque:
  `GenericChannelHandle` holds a `pal_async` `Task`, an alias of an
  `async_task` type; `ChannelHandle`'s `PhantomData<fn() -> Box<T>>` is a
  function-pointer type, which Verus does not support);
  `SimpleDeviceHandle::id` is `self.0.id()`.
- *Creation*: `offer_channel`, `offer_generic_channel`, `offer_simple_device`
  (and the private `offer_generic`) create the FracGhost (`Unknown`) and give
  back the other half (ghost out-parameter), with the handle's id
  (`external_body`).
- *The units* (`vmm_core`): `ChannelUnit<T>` and `SimpleChannelUnit<T>` gain
  a Verus-only `running` field; `StateUnitSpec`: `id()` = the handle's,
  `running()` = the field, `wf()` = `true` (a proxy has no state of its own
  to relate); spawned with `spawn_with_channel(.., unit_channel_for(), ..)`.

**`VmbusDevice`'s `Any` supertrait** (decided 2026-10-27, user: option A).
Verus refuses `dyn` of a trait whose supertrait has an unsized blanket impl
(rust-lang/rust#57893), as `Any` does (`impl<T: 'static + ?Sized> Any for
T`); the channel task holds its device as `&mut dyn VmbusDevice`, so the
task's spec could not name its device. Options:

| | How | Cost |
|---|---|---|
| A (chosen) | replace the `Any` supertrait with `IntoAny` (`fn into_any(self: Box<Self>) -> Box<dyn Any>`), implemented by a blanket impl for every **sized** `T: Any`; `revoke` calls `.into_any()` | an executable change in `vmbus_channel` only; implementors unchanged (the blanket covers them); same runtime behavior |
| B | patch Verus to exempt `Any` from the check (#57893's unsoundness needs associated types, which `Any` has none of) | a change to a verifier soundness check, its argument trusted |

Why behavior is unchanged: before, `revoke` upcast `Box<dyn VmbusDevice>`
to `Box<dyn Any>` (trait upcasting); now it calls `into_any`, which, through
the device's vtable, runs `IntoAny::into_any` for the device's concrete
type and returns the same `Box` as `Box<dyn Any>`; in both, the `dyn Any`'s
type is the device's concrete type, so the downcast gives the same result.
The `'static` bound that `Any` implied is written out on `VmbusDevice`.
Every implementor (`netvsp`'s `Nic`, `storvsp`'s `StorageDevice`,
`SimpleDeviceWrapper`, two `vmbus_server` test devices) is a sized
`'static` type, so the blanket impl covers it. Covered by `netvsp`'s tests,
which revoke a `ChannelHandle<Nic>` and unwrap the downcast.

**Open (channels; parked: VMBus is out of scope for the microVM, see
"Out of scope for now"):**

- **A channel's task starts running.** `Device::new` sets `state:
  DeviceState::Running`, so a new channel's task handles the guest's
  requests before its unit's first `Start`, while, by the rule, the unit is
  stopped (holding one half). `keeps_share`'s "while running, the whole
  share" fails from creation until the first `Start`. (In practice the
  vmbus server, a dependency, starts first, so the guest can reach the
  channel in that window.) Either the task starts stopped (an executable
  change: the guest's requests are queued until the first `Start`, then
  replayed, as after every `Stop`), or this is a trusted fact. To decide.
  (`offer_simple_device` with `InitialDeviceState::Stopped` leaves the
  device stopped, but its task still starts `Running`.)
- **`Device::run_channel`'s contract** (done): Verus does not support a
  `mut self` parameter, so the signature takes `self` and the body rebinds
  it (`let mut this = self;`, an executable change, code change 47). With
  the task's share as a ghost input: requires `self.keeps_share(share,
  channel)` and `state_req_recv.constant() == share.frac.id()`; the body is
  `external_body`.
- **No vmbus device's `saved_blob()` / `device_wf()` is defined yet**
  (`netvsp`, `storvsp`, the simple devices): device-level spec, per device.

### Chipset devices: the share in the device's lock

Decided 2026-10-27 (user): option B below. Every chipset device's unit
keeps its share of its FracGhost *in the device's lock, next to the
device*, in a generic wrapper. The PIT is folded into this: it is one of
the devices behind the shared chipset unit (`ArcMutexChipsetDeviceUnit`),
and its own unit machinery is retired.

**The problem.** About 56 device types sit behind one shared unit,
`ArcMutexChipsetDeviceUnit`, each in an `Arc<CloseableMutex<..>>` that its
unit and the vCPUs (port and MMIO I/O), the line sets, vPCI and PCIe all
lock. Under the rule ("Design change: the FracGhost lives with the
device"), the unit's share must live where every one of those changes can
update it, that is, inside the lock.

**The two options considered:**

| | A: a ghost field in every device (the PIT's earlier way) | B: the share in a wrapper inside the lock (chosen) |
|---|---|---|
| What the lock holds | the device, which carries its share | `DeviceSlot<T> { share, omit, device: T }` (`share`, `omit`: Verus only) |
| Where each device's snapshot is defined | in each of the ~56 device types | once, from `ProtobufSaveRestore`'s contract, which every device has: `omit ? NoSavedState : Saved(device.saved_blob())` |
| What a device must prove for snapshots | its `SaveRestore` contract, plus ghost updates in every handler that changes it | only its `SaveRestore` contract (`save` returns `saved_blob()`, `restore` sets it) |
| Where "a stopped device does not change" is enforced | in every device's handlers | once, in the code that calls into devices under the lock: it can change a device only while holding the whole share, that is, while the unit runs |
| Code changes | ~56 device files in many crates, each crate Verus-enabled, plus the object-safe `DynDevice` | a type change in about 15 files (see below), mechanical; devices untouched |
| Runtime | unchanged | unchanged (normal builds: the wrapper holds just the device) |

Why B: (1) it states exactly the high-level property and no more: the
snapshot proof needs that `save` returns the device's saved state, that
`restore` sets it, and that nothing changes a stopped device; the first two
are the existing `SaveRestore` contract, the third is enforced at the one
place that calls into devices. A adds per-device ghost bookkeeping that
proves nothing more. (2) It verifies the same code: every device's `save` /
`restore` must still meet its contract, and every call into a device must
still happen while its unit runs; neither is trusted. (3) It is the smaller
change, in one layer instead of 56 devices. Its cost is the type change
visible in four crates.

**The spec** (`vm/vmcore/src/device_state.rs`, `device_state.spec.rs`):

- `DeviceSlot<T: ?Sized>`: the device (last field, so the slot coerces to
  `DeviceSlot<dyn ChipsetDevice>` and the like), and, Verus only, `share:
  Tracked<DeviceGhostState>` and `omit: Ghost<bool>`. Dereferences to the
  device (`Deref` / `DerefMut`), so calls through a guard are unchanged.
  `DeviceMutex<T>` = `CloseableMutex<DeviceSlot<T>>`.
- `DeviceSnapshot` (Verus only; an empty stand-in in normal builds): `spec
  fn saved_blob(&self)`, for every `ProtobufSaveRestore` type (blanket),
  and through `DynDevice` (a supertrait) for devices seen as `dyn`.
- The lock invariant (`LockInvariant for DeviceSlot<T>`): key =
  (the share's id, `omit`); invariant: the device is well formed
  (`device.device_wf()`), and the share follows the rule
  (`DeviceGhostState::follows_rule`) for `snapshot()` = `omit ?
  NoSavedState : Saved(device.saved_blob())`. (`CloseableMutex::key` is now declared for
  unsized values too.)
- `DeviceSlot::new` (`external_body`): the creator-made FracGhost: it
  creates the unit's FracGhost (`Unknown`), keeps one half as the share,
  and gives the other back (ghost out-parameter) for
  `state_unit::unit_channel_for`; `omit` is a ghost input.
- `ArcMutexChipsetDeviceUnit`'s `StateUnitSpec`
  (`backing/arc_mutex/state_unit.spec.rs`): `id()` = the lock's key's id,
  `wf()` = the key's `omit` part equals the unit's `omit_saved_state` (so
  the lock's snapshot and the unit's `save` agree), `running()` = its
  `running` field. `ArcMutexChipsetDeviceUnit::new` (`external_body`):
  requires the key's `omit` part to match. Its two spawn sites use
  `spawn_with_channel(.., unit_channel_for(), ..)`.

**The type change** (code change 44): every place that names a device's
lock: vmotherboard (builder, MMIO/PIO/PCI dispatch, services, line sets,
base chipset, the unit), `vmm_core/src/device_builder.rs`, `vpci` (the
devices behind a vPCI bus, and the bus itself), `chipset_arc_mutex_device`
(the test chipset, whose devices also go to vPCI), the fuzzer,
`openvmm_core`'s `dispatch.rs`, and OpenHCL's fallback MMIO device.
Constructors become `CloseableMutex::new(DeviceSlot::new(dev))`.

**The PIT, folded in.** Retired: `DeviceGhost` and `StateUnitDevice`
(`vm/vmcore/src/device_state.spec.rs`), `DeviceUnit`
(`vmm_core/state_unit/src/device_unit.rs`, removed), the PIT's Verus-only
`ghost` and `id` fields and its constructor's placeholders, its
`DeviceGhost` and `LockInvariant` impls, `PitDeviceView::snapshot`, its
`StateUnitDevice` impl and the four `example_*` protocol walk-throughs
(`pit/mod.proof.rs`). Kept: its view, `PitDeviceView::saved_state`, `SaveRestoreSpecImpl`,
`wf()` (now the view's invariant only), and its entry points' contracts
(without the ghost clauses). What the PIT work found stays: the Verus
patches (0001 to 0005), the `SavedStateBlob` contract, and the
`CloseableMutex` spec. `StateUnitDevice` was a per-device copy of what the
`StateUnit` trait contract now says once for every unit.

**Open:**

- **A device's own well-formedness**: resolved 2026-10-27 (code change
  45). `SaveRestore`'s spec extension declares a trait-level `device_wf()`
  (the device's `wf()`, by the view / `wf()` convention: its view's
  `inv()` and the properties of fields outside the view), which
  `ProtobufSaveRestore` and `DeviceSnapshot` pass on; the slot's lock
  invariant is `device.device_wf()` and the share rule, so every lock
  holder gets it and must keep it; `save` and `restore` require and keep
  it. Each device defines it when it is specified (the PIT: its `wf()`);
  for the others it is unspecified, as their `saved_view` is.
- **The dispatch layer's obligation**: code that changes a device under
  its lock (the vCPU I/O dispatch, line sets, the unit's poll) must hold
  the whole share and update it (`Saved(device.saved_blob())` after the
  call); it is unverified today ("Unverified `lock`, `lock_if_open`,
  `close`" in `SNAPSHOT-SPEC-TRUSTED.md`).
- **`Deref` / `DerefMut`** give unverified code the device without the
  share; covered by the same trusted assumption.
- **The half reaching `unit_channel_for`**: the builder (unverified) is
  trusted to pass `DeviceSlot::new`'s other half to the unit's sender, and
  the right `omit`.

### `SaveRestore`'s contract on views

Decided 2026-10-27 (user), in two steps: first state the contract on the
device's view (option A below, code change 49), then compare saved states
by their views too, with no blobs at the device level (code change 50).
Replaces the contract over `saved_state()`, the typed saved state, which
compared saved states with `==`: a saved state holding a `Vec` (portb, the
IOAPIC, virtio-mmio) cannot be built in spec, and two `Vec`s with equal
views are not provably `==`, so `save` could never meet it. Comparisons
are between views.

**The contract** (`save_restore.spec.rs`):

```
spec fn saved_view(view: <Self as View>::V) -> <Self::SavedState as View>::V;  // per device
save:    final(self)@ == old(self)@
         result matches Ok(s) ==> s@ == Self::saved_view(old(self)@)
restore: result is Ok ==> Self::saved_view(final(self)@) == state@
(both require and keep device_wf())
```

`saved_view` projects the device's view onto what it saves: some of a
device's state is not saved (the PIC's `primary`, the PIT's `time_abi`:
static configuration), and the saved form may use other types or names.
A device's own contracts can say more (the PIT's `restore_device` keeps
`time_abi`).

**Blobs only in generic code.** A device's spec never mentions blobs.
The blob appears in `ProtobufSaveRestore`'s blanket impl, whose
`saved_blob()` is `of_view::<T::SavedState>(T::saved_view(self@))`
(verified with the link fact below), and above it in the units and
`LoadedVm`, all generic.

**Where the views come from.** `SaveRestore` gets the supertrait
`SaveRestoreView` (code change 49) and `SavedState` the bound
`SaveRestoreView` (code change 50): in Verus builds `View`; in normal
builds an empty trait implemented for every type. The spec cannot add
these bounds itself: an `external_trait_specification` cannot add bounds
the real trait lacks, nor give implementations an associated type (both
tried).

**A saved state's view: its meaning** (decided 2026-10-27, user). A
saved-state type's view is what the saved state means: what `restore`
makes of it. Two encodings that `restore` reads the same way have the same
view; two it reads differently never do. Guest time (out of scope) is not
in it: the RTC's saved-state view has no `clock_time_millis` or
`time_valid` (which `restore` reads as `true` when absent); vmtime's is
`()` (all time). Legacy encodings read the same as current ones: the
IOAPIC's empty `line_levels` (snapshots from before commit `4e396f9df`)
is every line low, as `restore` treats it; and an IOAPIC entry with an
interrupt pending reads as delivered (`restore` delivers it,
`IrqEntry::evaluate`; the IOAPIC itself never keeps one). So `restore(s)`
then `save` gives back `s`'s meaning, and the generic contract needs no
per-device normalization.

**Which time fields to leave out** (decided 2026-10-28, user). Leave a
time field out of a saved-state view only when one of these holds; keep
every other time value (an ordinary stored field, which `save` copies and
`restore` sets: keeping it is provable and makes the spec more precise):

- (a) **the view cannot determine it**: `save` reads it from a clock at
  save time;
- (b) **the time-ABI restore changes it after `restore`**: its unit's
  `advance_time` moves it forward by the downtime
  (`LoadedVm::finish_snapshot_restore` → `time_abi_restore`,
  `state_units.advance_time`). Keeping it would make
  `restore_from_snapshot`'s contract ("the units are as `restore` leaves
  them") false. Only units whose devices keep guest-visible time of their
  own get the advance (`ChangeDeviceState::advances_time`).

| Unit | Time field | Where `save` gets it; changed after `restore`? | In the view? |
|---|---|---|---|
| RTC | `clock_time_millis` | read from the UTC clock at save (`saved_clock_time`); advanced by the RTC's `advance_time` | **no**: (a) and (b) |
| RTC | `time_valid` | stored; part of the clock's state (`restore` reads an absent value as `true`) | no: with the clock |
| PIT | `last` (the VM time up to which the timers were evaluated) | stored field, copied; not changed (the PIT has no `advance_time`: its deadlines follow VM time) | **yes** |
| vmtime | `vmtime` (the VM time at stop), `stop_host_time` | stored at `stop`, copied by `save`; **advanced** by the keeper's `advance_time` after a time-ABI restore | **no**: (b). Its saved-state view is `()` |
| partition (VPs) | the TSCs, the LAPIC timers | read from the hypervisor at save; the LAPIC timers advanced after a time-ABI restore (`advance_lapic_timers`) | no: (a) and (b) (when the VP layer is specified) | The
trusted encode/decode contract (`save_restore_t.rs`) says a blob is
determined by its saved state's view (`of(v) == of_view::<T>(v@)`) and
that `of_view` is injective, i.e. a blob's view is the meaning of the saved
state it encodes. So "time is out of scope" is "time is not in the
saved-state views", reviewed per saved-state type in its `.spec.rs`;
this replaces `equal_except_time` and its per-type axioms (vmtime's
`axiom_all_time`, removed).

**Options considered:**

| Option | What | Outcome |
|---|---|---|
| A (chosen) | `View` as `SaveRestore`'s (Verus-only) supertrait; the contract on `self@` | every `SaveRestore` type compiled in Verus runs needs a view; the ~10 microVM devices get real views, the others placeholders |
| B | no supertrait; the generic contract blob-only, views only in each device's own contracts | rejected: "save keeps the view" said per device instead of once |
| `blob_of(view)` (code change 49) | the device's blob as a function of its view | superseded by `saved_view`: needed a per-type time axiom (the RTC) and could not handle `Vec`s |
| predicate `is_saved_state(&self, state)` | judge a saved state on views | rejected: needs a `forall` to say `save` keeps it, and `choose` to define the blob |
| `SavedStateBlobView` as the typed saved state | | rejected: a blob is not interpretable |
| trusted "`Vec`s with equal views are `==`" | | rejected: compare views instead |

**Placeholder views.** Every `impl SaveRestore for X` compiled when Verus
runs (the 12 verified crates and their dependencies) needs `X: View` and
`X::SavedState: View`: 63 impls in 27 crates, found by the compiler. The
microVM's devices get real views; the others placeholders from vmcore's
`placeholder_view!` macro, a line before the impl:

- `placeholder_view!(pub ExX, X)` (or `ExX<T>, X<T>` for a generic type)
  declares the device `X` to Verus as an opaque type and gives it the
  view `()`.
- `placeholder_view!(external[FDC: Bound] X<FDC>)` gives `X` the view `()`
  in a plain Rust impl outside `verus!` (so external to Verus), without
  declaring `X` to Verus; used where declaring `X` would need its bounds'
  traits declared too (`chipset_legacy`'s `Winbond83977FloppySioDevice`).
- `placeholder_view!(saved T)` gives the saved-state type `T` its identity
  view (`V = T`), outside `verus!`. Verus does not see these devices'
  `SaveRestore` impls (outside `verus!`), so neither needs a view there; an
  identity view is complete, as a saved-state view must be. The partition's
  `VmSavedState` and `VpSavedState` have theirs in `virt` (orphan rule).
  `NoSavedState`'s view is itself; `SavedStateNotSupported` (no values)
  has the view `()`, outside `verus!`.

A device placeholder makes `final(self)@ == old(self)@` trivially true:
the contract says nothing about that device, as before (its `saved_view`
is unspecified). A placeholder is replaced by a real view when the device
is specified. Not in the 12 crates' dependency graph, so untouched: the
other 16 crates with `SaveRestore` impls (e.g. `underhill_core`, `nvme`,
`tpm_device`).

Verus-enabled for this (code change 49), 22 crates: `amd_iommu`,
`chipset_legacy`, `cxl_spec`, `firmware_pcat`, `floppy`,
`floppy_pcat_stub`, `framebuffer`, `generation_id`,
`hcl_compat_uefi_nvram_storage`, `ide`, `intel_vtd`, `missing_dev`,
`pci_bus`, `pci_core`, `pcie`, `smmu`, `uefi_nvram_storage`,
`vfio_assigned_device`, `vga`, `virtio`, `vpci`, `watchdog_core`. Their
vstd import is `use vstd::prelude::verus as _;`, not the glob: the glob's
`int` type turned a closure parameter named `int` into a pattern
(`vfio_assigned_device`; the PIC's top-level import is narrow for the
same reason).

**Open:**

- **The partition's VP saved state** (`virt`'s `VpSavedState`) has the
  placeholder identity view, so its TSC and LAPIC-timer fields are not yet
  left out. Its unit (`PartitionUnitRunner`) is uninterpreted, so nothing
  depends on it yet; its real view (without time) comes with the
  partition's spec.
- **The PIT's `last`**: resolved 2026-10-28 (user): kept in the view (a
  stored field; see "Which time fields to leave out" above).

### The partition runner's construction order

Decided 2026-10-28 (user), code change 56. **Why:** the partition unit's
runner keeps its share of its FracGhost in a Verus-only field. The share
comes from the unit's channel, which `spawn` creates. The runner used to
be built **before** `spawn`, so its share started as a placeholder
(`Tracked::assume_new()`, a ghost made from nothing), replaced in the
unit's task before `run`. That left two things unverified: the
placeholder, and the order "replace the placeholder, then run", which was
in an unverified async closure.

**The change: only the order of construction.** It is not a concurrency
change; everything still happens on the same task, one step after the
other.

| Step | Before | After |
|---|---|---|
| the checks, `VpSet` setup and the VPs, each of the runner's field values (with `gdb`, the debugger state, which can fail with `MissingGuestMemory`) | before `spawn`, in `PartitionUnit::new` | **unchanged**: before `spawn`, in `PartitionUnit::new`, same order, same error; collected in `RunnerParts` |
| assemble `PartitionUnitRunner { .. }` | before `spawn` | inside the unit's task (`PartitionUnitRunner::new`), from `RunnerParts`, taking the share from the channel |
| run the runner | in the task (an async closure) | in the task (`run_partition_unit`, a named async fn) |

Assembling a struct from computed values only moves them (no effects, no
failures), so the order of effects, the errors `PartitionUnit::new`
returns, and the runner's behavior are unchanged.

**What is verified now:** `run_partition_unit` (build the runner, taking
the share from the channel, then run it) is verified: the order is
checked. `PartitionUnitRunner::new`'s contract says it takes the channel's
half; its body is `external_body` until proved (it still makes the share
with `Tracked::assume_new()`, to be replaced by taking the half from the
channel, as in `KeeperUnit::new`). No placeholder is left outside it.

**Verus note:** the spawn closure awaits `run_partition_unit` inside an
`async` block (`|recv| async move { run_partition_unit(parts, recv).await }`);
returning the verified async fn's future directly from the unverified
closure crashes Verus ("`VerusErasureCtxt` has not been initialized"), as
for the chipset device unit's spawn sites (code change 41).

### The input distributor's initial share

Decided 2026-10-28 (user), code change 57. The input distributor's unit
keeps its share of its FracGhost in `Inner`'s Verus-only field `ghost`
(`Tracked<Option<DeviceGhostState>>`): `None` until `InputDistributor::run`
takes the unit's half from the unit's channel.

**Before:** `InputDistributor::new` (unverified) built `Inner` with
`ghost: Tracked::assume_new()`, a ghost value made from nothing (any
value, not necessarily `None`), relying on `run` to overwrite it.

**Why not the partition's fix** ("The partition runner's construction
order": build the runner inside its task): the distributor must exist
before its unit is spawned, because its users need it then: `dispatch.rs`
hands `input_distributor.client()` to the
device resolvers before `spawn`, and takes the input channel back after the
unit is removed (`into_inner`). And unlike the partition, the order "take
the share, then handle requests" is already inside a function in Verus
(`InputDistributor::run`, contract stated, body to prove), not in an
unverified closure. Only the made-up ghost was left.

**The change:**

| | Before | After |
|---|---|---|
| who builds `Inner` | a struct literal in `InputDistributor::new` | `Inner::new()` (new), called from `InputDistributor::new` at the same point |
| `Inner`'s fields | `running: false`, two empty forwarders, and (Verus) `ghost: Tracked::assume_new()` | the same `running` and forwarders, and (Verus) `ghost: Inner::no_share()`, which is `Tracked(None)` |
| verified | nothing (`new` is outside Verus) | `Inner::new()` is verified: ensures `!running` and `ghost@ is None`; `Inner::no_share()` is verified (a Verus-only helper in a `verus!` block, since `Tracked(None)` must be built in `verus!` syntax) |

`Forwarder::new` is declared to Verus (`verus_verify(external_body)`, no
contract needed). Normal builds: `Inner::new()` builds the same `Inner`
as the old literal, at the same point, so behavior is unchanged.

**What is left to prove:** `InputDistributor::run` takes the half from the
channel into `ghost` before any `StateRequest::apply` (its body,
`external_body` for now).

### virtio-MMIO's interrupt status: the guest-memory boundary

Decided 2026-10-28 (user), code change 58.

**Where the status lives.** A virtio-MMIO transport is created in one of
three interrupt modes (`VirtioMmioInterruptMode`, chosen by its creator;
the accelerated acknowledgements are automatic on Linux when available):

| Mode | The status is | Its writers |
|---|---|---|
| `Legacy` (the default) | `InterruptState.status`, a field in OpenVMM's memory behind a lock | the transport (guest acknowledgement writes, `restore`, reset), the device task's interrupt closures |
| `Legacy` with accelerated acknowledgements | the same | the same, and the acknowledgement worker task |
| `SharedStatus { status_gpa }` | a 32-bit word **in guest memory** at `status_gpa` (`SharedInterruptStatus`), which the device sets and the guest's driver reads and clears directly, without trapping into OpenVMM | the device (through `SharedInterruptStatus::update`, a compare-and-swap loop on the word), and the guest |

**The microVM always uses `SharedStatus`** (`dispatch/microvm.rs`,
`microvm_virtio_status_gpa`). So for the microVM the interrupt status is a
word of guest memory.

**The boundary.** Rather than ignore that word (guest RAM is otherwise
out of scope), the spec treats it as one abstract value,
`MmioTransport::interrupt_status()` (uninterpreted), with trusted contracts
on the two transport functions that decide what `save` returns and what
`restore` leaves:

| Function | Trusted contract | Used by |
|---|---|---|
| `read_interrupt_status(&self)` | returns `interrupt_status()` | `save`, guest reads of `INTERRUPT_STATUS` |
| `restore_interrupt_status(&mut self, s)` (new) | on `Ok`, `interrupt_status() == s` | `restore` |

(`reset_interrupt_state`, which `restore` calls first, needs no contract:
`restore_interrupt_status` sets the final status after it.)

Also trusted: otherwise only the guest (while its VPs run) and the device
task's interrupts (while the device runs) change it, so it is stable while
the VM is stopped. With these, virtio-MMIO's `save` and `restore` can be
proved against the view (which holds `interrupt_status()`); what is trusted
is only the guest-memory access (`GuestMemory`, in
`SharedInterruptStatus::update`) and that the guest does not run while
stopped. The same contracts hold in the legacy modes (the value is then
the locked field), but there they are a stand-in for the lock protocol
(TODO in "Other open items").

**Code change (58):** `restore`'s inline block (lock the interrupt state,
`restore_status`) moved into the new `MmioTransport::restore_interrupt_status`
(same calls, same lock, same order), so that it can carry a contract: a
contract can only be attached to a function. Behavior unchanged.

### The hypervisor boundary

Decided 2026-10-28 (user): the lowest trust boundary of the VP layer is
the per-element getters and setters that talk to the hypervisor; every
OpenVMM layer above them (`save_all` / `restore_all`, `WrappedVp`,
`BoundVp`, the VP threads, `VpSet`) is to be verified. (Code change 59,
Verus-only.)

**What it is.** A VP's (and the partition's) hypervisor state is reached
through a trait generated by `state_trait!` (`vmm_core/virt/src/state.rs`):
`AccessVpState` per VP and `AccessVmState` per partition, for x86 and
aarch64. Each backend (KVM, WHP, mshv, hvf) implements it. The trait has,
per state element (x86 VP: registers, activity, xsave, APIC, XCR0, XSS,
MTRRs, PAT, virtual MSRs, debug registers, TSC, CET, CET SS, TSC_AUX, TSC
deadline, synic MSRs and pages, message queues, timers, nested state), a
getter `$get(&mut self) -> Result<$ty, Error>` and a setter
`$set(&mut self, &$ty) -> Result<(), Error>`, and `commit()`.

**The abstract state.** `hv_state()` (uninterpreted, TRUSTED: the
hypervisor's) is the hypervisor's state as a value of the trait's
saved-state type (`VpSavedState`, `VmSavedState`): for each element, its
current value. Closed spec accessors on the saved-state type name an
element (`state.registers()`) and set one (`state.set_registers(v)`).

**The trusted contracts** (`state_spec::StateAccessSpec`, generated per
trait):

| Method | Contract |
|---|---|
| getter `$get` | changes nothing (`hv_state()` unchanged); on `Ok(v)`, the element's value is `v` |
| setter `$set(value)` | on `Ok`, `hv_state()` becomes the old one with that element set to `value` (and only it); on `Err`, unspecified |
| `commit` | changes nothing (every host backend applies a set at once: their `commit` returns `Ok(())`) |

That is all that is trusted at this level: the hypervisor reads and
writes each element faithfully. Checked with a throwaway probe (not
kept): after a getter the state is provably unchanged; claiming the same
after a setter fails.

**Not yet here** (later layers, the proof agent's, guided by the VP
protocol): which elements are present (`StateElement::is_present`, from
the partition's capabilities); `save_all` / `restore_all`, which loop
over the elements; and the VP saved state's view, which leaves out the
TSC and the LAPIC timers (time rule). Note also that the synic message
and event-flag pages are guest memory (out of scope).

**How (Verus only).** The macro emits, per invocation, a module
`state_spec`: one small module per element declaring its type to Verus
as opaque (`macro_rules!` cannot make new identifiers), the spec
accessors on the saved-state type, and the trait's
`external_trait_specification` with an extension (so the backends, which
are not Verus crates, need no changes). The saved-state struct gets
`cfg_attr(verus_keep_ghost, verus_verify)` and
`verifier::external_derive` (its derives stay outside Verus).
`std::error::Error` is declared once in `virt` (`state.rs`), for the
trait's `Error` bound.

### The trust audit

Run 2026-10-28 (user), after a gap was found (the PIT's `SaveRestore`
impl was plain Rust, so the trait's contract was assumed for it, not
checked). **Why gaps hide:** Verus is silent about code outside `verus!`:
a plain-Rust impl of a trait with a contract compiles without a warning,
and its contract is assumed. So the audit follows the microVM's
save/restore path at runtime (`LoadedVm::save` / `restore`,
`restore_from_snapshot`, `quiesce_for_snapshot`) down to the trust
boundaries, and classifies what it reaches.

**What is a silent gap.** A plain function called inside an
`external_body` body is not one: its caller is explicitly unproved, so the
proof must bring it into Verus anyway. A silent gap is where Verus
**assumes** something nobody decided: (1) a plain-Rust impl of a trait
that has a contract, reached through generics or `dyn`; (2) an `assume` or
a made-up ghost (`assume_new`) in verified code; (3) a dispatch point
(`dyn` trait) on the path whose methods have no contract, so the link it
makes is unstated.

**Method.** A classifier
(`scripts/verus-trust-classify.py`: per function or impl, VERIFIED /
CONTRACT (`external_body`) / PLAIN; a line-based heuristic) over the
path's files; the
dispatch points traced by hand (how each unit reaches its device:
resolvers, `dyn` traits, channels); a sweep for `assume(`, `admit()`,
`assume_new()`, and `assume_specification` in the spec crates.

**Contract-bearing traits and their impls on the path:**

| Trait | Impls on the microVM path | Status |
|---|---|---|
| `StateUnit` | the 9 production units (vmtime, time-abi, input, chipset, line sets, partition, chipset device units, and the vmbus units, out of scope) | all `verus_verify` (bodies mostly `external_body`); only test units are plain |
| `SaveRestore` | the PIT, PIC, RTC, portb, IOAPIC, shutdown, snapshot-request, virtio-MMIO | all `verus_verify` (the PIT since code change 60) |
| `SaveRestore` | `WrappedPartition`, `WrappedVp` (`openvmm_core/src/partition.rs`) | **PLAIN: gaps G3, G4** |
| `ProtobufSaveRestore` | the blanket impl for `SaveRestore` types | verified |
| `ProtobufSaveRestore` | `ErasedChipsetDevice` (`chipset_device_resources`), `BoundVp` (`vp_set.rs`) | **PLAIN: gaps G1, G4** |
| `ProtobufSaveRestore` | `ChipsetDeviceProxy` (`workers/`) | plain, **not on the path** (used only for the TPM, which the microVM excludes) |
| the hypervisor state traits | the backends (KVM, WHP, mshv, hvf) | plain, **trusted on purpose** ("The hypervisor boundary") |

**Dispatch points on the path:**

| Dispatch point | Reaches | Status |
|---|---|---|
| a chipset device unit → its device: `dyn DynDevice` (`vmotherboard`, `backing/arc_mutex/state_unit.rs`) | the device's `ProtobufSaveRestore` | **fixed (G2, code change 61)**: `save` / `restore` moved into `DynDeviceState`, `DynDevice`'s supertrait, with contracts (the device's blob, from `DeviceSnapshot`); its blanket impl is verified from the device's `ProtobufSaveRestore` (negative check: a `restore` that skips restoring fails) |
| resource-built devices: `ErasedChipsetDevice(Box<dyn DynChipsetDevice>)` (`base_chipset.rs:295`) | the PIT, PIC, IOAPIC, portb, shutdown, snapshot-request | **fixed (G1, code change 62)**, with one trusted fact: the wrapper's blob is its `dyn` device's, and its `save` / `restore` are verified; at the one erasure point (`From<T> for ResolvedChipsetDevice`) a trusted contract says the erased device's blob and `device_wf` are `t`'s (Verus cannot prove it while `DynChipsetDevice` is `async_trait`, G5) |
| typed devices: `arc_mutex_device(..).add(..)` | the RTC (`base_chipset.rs:447`), virtio-MMIO (`dispatch/microvm.rs`) | connected (no erasure) |
| the partition unit → `Box<dyn VmPartition>` | `WrappedPartition`'s `SaveRestore` | **gap G3** |
| the partition unit → `VpSet` → VP threads → `BoundVp` → `WrappedVp` | the hypervisor getters | **gap G4**: the VP layer (the VPs' saved state is now defined from `VpSet`'s halves, "The VP protocol"; the layers below `VpSet` are to prove) |
| virtio-MMIO → its device task → the inner device (net, console, fs, block) | the captured private state | recorded TODO (the inner devices' layer): the blob is opaque in the view |
| `StateUnits` → each unit: `mesh` channels through the RPC wrapper | the unit's task | trusted boundary (`mesh_rpc_t.rs`), listed |

**In-code assumptions** (outside the `_t.rs` trusted files): one `assume`
in verified code, in `StateRequest::apply`'s `Stop` ("a unit receives
`Stop` only while it runs"), recorded as trusted. Every `assume_new()` is
either inside an `external_body` body (a placeholder the proof replaces:
`DeviceSlot::new`, the units' constructors, `TimeAbiMsrs`, the partition
runner's `new`) or a recorded placeholder ghost at an RPC call site in
plain code (`VmRpc`'s and `PartitionRequest`'s protocols, uninterpreted:
`dispatch.rs`, `partition_unit/snapshot.rs`, `partition_unit.rs`, the
controller's `vm_controller/microvm.rs`).

**Silent gaps found** (the plan: fix G1 and G2 first, they disconnect the
chipset devices' specs from their units; G3 and G4 with the VP /
partition layer; G5 needs a Verus patch):

- **G1** `ErasedChipsetDevice`'s `ProtobufSaveRestore` (and `dyn
  DynChipsetDevice`): plain. **Fixed** (code change 62), except one
  trusted fact at the erasure point, tied to G5. Two probes (scratch
  files, not kept) showed why: Verus's `dyn` axiom (`dyn_spec_fn_axiom`,
  `vir/src/traits.rs`) connects a boxed value's spec function to the
  concrete type's only for spec functions **of the `dyn` trait itself**,
  with an impl Verus sees (a supertrait's spec function: the assertion
  fails; the `dyn` trait's own: it verifies). The `dyn` trait here,
  `DynChipsetDevice`, is `async_trait`, so neither holds; once G5's patch
  lands, giving `DynChipsetDevice` spec functions with a verified
  blanket impl makes the fact provable.
- **G2** `DynDevice`'s `save` / `restore`: no contract; blanket impl plain. **Fixed** (code change 61).
- **G3** `WrappedPartition`'s `SaveRestore`: plain (placeholder view).
- **G4** `WrappedVp`'s `SaveRestore`, `BoundVp`'s `ProtobufSaveRestore`:
  plain (the VP layer).
- **G5** **the `async_trait` glue** (found 2026-10-28). The async methods
  through `dyn DynDevice` (`vmotherboard`) and `dyn DynChipsetDevice`
  (`chipset_device_resources`): `start`, `stop`, `reset`, `advance_time`,
  `quiesce_input`, `resume_input`; also `ControlVp` (the VP thread's
  control of its VP, `vp_set.rs`), `VmPartition`'s async methods, and
  `HvliteVp` (`openvmm_core/src/partition.rs`). `#[async_trait]` rewrites
  each `async fn` into a fn returning `Pin<Box<dyn Future<Output = T> +
  Send + '_>>`, which Verus cannot express, so these methods can carry no
  contract and their blanket impls stay outside Verus. Each end can be
  specified (the unit's `StateUnit` methods; the device's
  `ChangeDeviceState`, native `async fn`, supported by patch 0004), but
  **they cannot be connected**: the unit's proof cannot use the device's
  contract through the glue. Not affected: `save` / `restore`, which are
  synchronous everywhere on the path (through `DynDeviceState`,
  `ProtobufSaveRestore`). **The `stop` contract is needed** (user,
  2026-10-28): a snapshot is consistent only if every device has really
  stopped before `save`. The G2 split does not apply (the methods are
  async, and Rust does not allow native `async fn` in a trait used as
  `dyn`). **To fix it, a Verus patch** (investigated 2026-10-28), in five
  pieces: (1) accept auto traits (`+ Send`) on a `dyn`, as markers (easy;
  `rust_verify/src/rust_to_vir_base.rs`, "dyn with auto-traits" / "dyn
  with more than one trait"); (2) accept associated-type bindings on a
  `dyn` (`dyn Future<Output = T>`, "dyn with projections": VIR's
  `TypX::Dyn` has no slot for them; moderate to hard); (3) vstd specs for
  `Pin<Box<F>>` as a future with `F`'s result; (4) the generated bodies,
  `Box::pin(async move { .. })` (depends on Verus's `async` block
  support); (5) a way to write contracts on `async_trait` methods (the
  macro turns `async fn` into a plain `fn` returning a future, so patch
  0001's handling does not apply). Verus's async model itself is general
  (`vstd/future.rs`: `awaited()`, `view()` for any `Future`). Deferred;
  to revisit at the latest with the VP layer (`ControlVp`).

**What the audit does not cover:** whether each contract says the right
thing (that is review of the `.spec.rs` files), and the contents of the
trusted items (`SNAPSHOT-SPEC-TRUSTED.md`); paths other than the microVM's
save/restore (e.g. the VM's normal running, which no contract here
covers).

### The VP protocol

Proposed and reviewed 2026-10-28; **specified** (code changes 63 to 68;
see "Status" at the end of this section): the interface between `VpSet`
(in the partition unit's task) and the VP threads (`VpRunner`), the
`VpEvent` channel, one per VP (`vmm_core/src/partition_unit/vp_set.rs`).
It is the state-unit protocol ("Request protocol") one level down. With
it, `VpSet::saved_vps()` (trusted before) becomes defined, and the layers
in between (`save_all` / `restore_all`, `WrappedVp`, `BoundVp`, the VP
thread's loop, `VpSet`'s loops) are left to the proof agent, with this
protocol and the hypervisor boundary ("The hypervisor boundary") as their
spec.

**Facts the protocol must fit** (from `VpRunner::run_inner`):

| Fact | Where |
|---|---|
| a VP is `Stopped`, `Running`, or `Halted` (it stops by itself when the VM halts, and is `Stopped` only once `Stop` arrives) | `VpState`; `Stop` is accepted from `Halted` |
| state events (`Save`, `Restore`, `Reset`, `Inspect`, `SetInitialRegs`, `CheckTimers`, `AdvanceLapic`, ...) can arrive while the VP runs: the thread pauses the VP, handles them, and resumes | `Event::Vp(VpEvent::State(..))` in the run loop |
| `VpSet::save` and `restore` require stopped VPs | `assert!(!self.started)` |
| a cancelled `run` can be reissued with a **different `vp` object** for the same VP | `VpRunner::run`'s doc |

**The ghost.** One FracGhost per VP, a `UnitSnapshot` (the units' type;
`NoSavedState` is never recorded for a VP): `Unknown` or `Saved(blob)` (the VP's saved state, as a blob view, without guest time:
the TSC and the LAPIC timers, time rule). Its halves: one in `VpSet` (per
VP), one in the VP thread (`RunnerInner`, which survives a reissue of
`run`, so the share is per VP index, not per `vp` object).

| | `VpSet` holds | the VP thread holds |
|---|---|---|
| VP stopped | one half | one half |
| VP running or halted | nothing | both halves (it may change the VP) |

**The rule** (as for units): while `VpSet` holds its half, the VP's state
is the recorded one: `Saved(b)` means the VP's `save` returns `b`.

**Events and replies** (each `VpEvent` sent through the mesh RPC wrapper,
with the ghost each row says):

| Event | Sent with | Reply carries | Promise |
|---|---|---|---|
| `Start` (one-way) | `VpSet`'s half | (none) | the thread then holds both halves |
| `Stop` (to become an `Rpc<(), ()>`; today a bare oneshot) | (nothing) | a half, recording `Saved(the VP's blob)` | the VP has stopped; the recorded blob is the one its `save` returns |
| `State(Save)` | `VpSet`'s half, lent (the VP is stopped) | the same half back | `Ok(blob)`: `blob@` is the recorded blob; the VP is unchanged |
| `State(Restore(blob))` | `VpSet`'s half | the half back, recording `Saved(blob@)` on `Ok` | on `Ok`, the VP's blob is `blob@` |
| `State(Reset)`, `State(Scrub(vtl))`, `State(SetInitialRegs(..))` while stopped | `VpSet`'s half | the half back, also on `Err` (its value per the share rule: `Unknown` or the VP's current blob) | they change the VP |
| `State(AdvanceLapic(..))` | (nothing) | (nothing) | changes only the LAPIC timers, which the blob's view leaves out (time rule (b)) |
| `State(Inspect)`, `CheckTimers`, `GetDumpVpState` | (nothing) | (nothing) | change nothing |
| `State(Debug(..))` (`gdb` only) | out of scope | | debugging is not part of the snapshot spec |

While the VP runs, the thread holds both halves, so a state event that
arrives then needs no ghost from `VpSet`.

**What it gives.** `VpSetView::saved_vps()` = for each VP, its index and
the blob recorded in `VpSet`'s half (defined, while the VPs are stopped;
no longer trusted). The partition unit's snapshot then follows from the VP
protocol and the hypervisor boundary.

**Code changes it needs** (executable, after review): `VpEvent::Stop`
becomes an `Rpc<(), ()>` (today `mesh::OneshotSender<()>`); `VpSet` sends
and the VP thread answers through the RPC wrapper (`call`, `recv_msg`,
`split`, `complete`), as for state requests (code change 31); a one-way
`send_msg` with a ghost added to the wrapper (trusted, like `call`) for
`Start`; Verus-only ghost fields in `VpSet` (per VP) and `RunnerInner`.

**Open questions:**

- `Halted`: **decided 2026-10-28 (user): treated like `Running`.** A VP
  halts by itself when the VM halts (the shared `Halt`): its thread stops
  running guest code and sets `Halted`, while `VpSet` is still `started`
  and holds no half. The partition unit then sends `Stop` (its `Halt`
  event: `vp_set.stop()`), which `Halted` accepts like `Running`: the
  reply hands `VpSet` its half, recording the VP's blob. `VpSet` cannot
  send `Save` or `Restore` meanwhile (`!started` is required).
- The `gdb` debug events (`SetVpState`, `WriteVirtualMemory`, ...):
  **decided 2026-10-28 (user): out of scope.** With a debugger attached,
  the protocol promises nothing.
- What `Reset` records: **decided 2026-10-28 (user): as for units.** The
  unit-level `reset` takes the caller's half and gives back a half, also
  on `Err`, with no promise about its value beyond the share rule
  (`Unknown` or the current state); the VP's `Reset`, `Scrub`, and
  `SetInitialRegs` do the same with `VpSet`'s half. As at the unit level,
  every request that changes the recorded value (`Restore`, `Reset`, ...)
  runs with the whole FracGhost: the caller's half travels with the
  request, so the receiver holds both halves while it changes the state
  (checked 2026-10-28: the unit-level `restore` contract already does
  this, recording `Saved(buffer@)` on `Ok`).
- Not covered by the protocol, left to the proof agent: the VP thread's
  run loop itself, `save_all` / `restore_all`, `WrappedVp`, `BoundVp`
  (gap G4); and G3 (`WrappedPartition`), the partition-wide state,
  similarly specified through `AccessVmState`'s boundary.

**Status** (2026-10-28, code changes 63 to 68):

| Piece | State | Where |
|---|---|---|
| the protocol (`MeshProtocol for VpEvent`, `StateEvent::request_ok`) | written, as the tables above | `vmm_core/src/partition_unit/vp_set.spec.rs` |
| `VpSet`'s view (`VpSetView`: `started`, each VP's recorded `UnitSnapshot`; `saved_vps()`, `all_saved()`) and `wf()` (`VpSet` holds a valid half per VP exactly while stopped) | written | same file |
| `VpSet::saved_vps()` | **no longer trusted**: `VpSetView::saved_vps()`, from `VpSet`'s halves; the partition unit's saved view uses it | `partition_unit.spec.rs` |
| the VP's FracGhost: created with its channel, one half attached to each end | **verified** (`vp_channel`) | `vp_set.rs` |
| `VpEvent::Stop` as an `Rpc<(), ()>` | done (executable, CC 63) | `vp_set.rs`, `vp_set/boundary.rs` |
| contracts on `new`, `add`, `start`, `halt`, `clear_halt`, `stop`, `reset`, `scrub`, `save`, `restore`, `restore_halt`, `set_initial_regs`, `stop_at_io_boundary`, `check_one_shot_timers`, `advance_lapic_timers` | written, bodies `external_body` (to prove) | `vp_set.rs`, `vp_set/boundary.rs`, `vp_set/time_abi.rs` |

Done since (code changes 66 to 68):

- **`VpSet::new`** has its contract (`wf()`, not started, no VPs). Its
  parameter type `GuestMemory` was declared to Verus by virtio, and a
  second declaration in `vmm_core` crashed Verus (a panic in Verus's
  erasure; a duplicate of an ordinary type is reported as an error). Now
  `guestmem` is Verus-enabled and declares it once (code change 67).
- **`VpIndex::new`** has its contract (verified): it moved into a `verus!`
  block, since the attribute form does not take a `const fn` (code change
  66).
- **`stop_at_io_boundary`** (as `stop`), **`check_one_shot_timers`** and
  **`advance_lapic_timers`** (view unchanged), **`get_dump_vp_state`**
  (`&self`) have contracts (code change 68).

Open (to do, not yet specified):

- **The calls through the RPC wrapper.** `VpSet`'s methods still call
  `mesh` directly; their proofs need `mesh_rpc_verus::call` (and, for the
  one-way `Start`, a new trusted `send_msg` with a ghost, to add). These
  are executable changes, like code change 31.
- **The VP thread's side.** Its half stays attached to the runner's
  receiver (`VpRunner::recv`); the run loop (`run_inner`) goes through
  `dyn ControlVp` (`async_trait`, gap G5), so its contract waits for G5.
- **Not specified**: the `gdb` methods (out of scope).

Concerns found while specifying (for review):

- **A VP whose runner is gone.** `stop` ignores a VP whose runner was
  dropped (`.ok()`), and a call whose channel fails returns no ghost, so
  `VpSet` would lose that VP's half and `wf()` would break. `stop`'s
  contract promises every VP's half back; proving it needs a fact like
  the units' ("channel failures do not happen for a registered unit"):
  VP runners are not dropped while their `VpSet` is in use. To be added to
  `SNAPSHOT-SPEC-TRUSTED.md` when a proof uses it, unless the proof can
  avoid it.
- **Restore with the VPs in another order.** `restore` accepts the saved
  VPs in any order (it checks that each index appears once), and `save`
  returns them in index order. The partition unit's restore contract says
  that after `Ok` the unit's recorded blob is the given one, so for a
  snapshot whose VPs are not in index order it would be false (`save`
  then returns a different encoding). OpenVMM's own snapshots are in
  index order. Options: require sorted VPs in the partition's restore
  contract, or compare saved states by their meaning (as sets) rather
  than as blobs.

### The microVM's units: what is specified

Decided 2026-10-27 (user): after VMBus went out of scope, specify the
microVM's units. The units are fixed: `openvmm_defs/src/microvm.rs` checks
the microVM's chipset-device inventory exactly (`ioapic`, `microvm-portb`,
`microvm-shutdown`, `microvm-snapshot-request`, `pic`, `pit`), the RTC
comes from the fixed chipset (`with_generic_cmos_rtc: true`, line 1991), and the tests' machine contracts list the
saved inventory (`openvmm_helpers/src/snapshot/microvm.rs`, e.g. line
2650). One unit per virtio-mmio device (net, console, fs, sandbox blocks),
each a chipset device `VirtioMmioDevice` wrapping the virtio device.

A chipset device's unit is `ArcMutexChipsetDeviceUnit` (its spec is
generic, through `DeviceSlot`); what remains per device is its view, its
saved state's view, and its `SaveRestoreSpecImpl` (`saved_view(view)`,
`device_wf()`) and, so that the
device's `save` / `restore` are checked rather than trusted, its
`SaveRestore` impl inside Verus.

| Unit | Unit's spec | Device's view, `saved_view` / `device_wf` | Device's `save` / `restore` |
|---|---|---|---|
| `vmtime` | done (`KeeperUnit`) | n/a | n/a |
| `time-abi` | done (`TimeAbiUnit`) | n/a | n/a |
| `input` | done | n/a (no state) | n/a |
| `chipset` | done (`ChipsetUnit`) | n/a (no state) | n/a |
| `lines/*` | done (`LineSetUnit`) | n/a (no state) | n/a |
| `partition` | done (`PartitionUnitRunner`: its share in a ghost field, taken from its channel when the runner is built inside its task, `run_partition_unit` (code change 56); its snapshot is its saved state's blob: the partition's (`VmPartition`'s `ProtobufSaveRestore`), the VPs' (`VpSetView::saved_vps`, from `VpSet`'s halves; "The VP protocol"), the encoded halt reason; running while started; `run` has a contract) | n/a (not a `SaveRestore` device; the hypervisor state is trusted) | `run`, `PartitionUnitRunner::new`: contracts, `external_body`; `run_partition_unit` **verified** |
| `pit` | done (generic) | done (`PitDeviceView`; `view.saved_state()`) | contract on the trait impl (`verus_verify`, bodies `external_body`: they call `save_device` / `restore_device`, which have the PIT's own contracts; code change 60) |
| `microvm-shutdown` | done (generic) | done: view `()`, `NoSavedState`, `true` | **verified** (impl is `verus_verify`) |
| `microvm-snapshot-request` | done (generic) | done: view `()`, `NoSavedState`, `true` | **verified** (impl is `verus_verify`) |
| `microvm-portb` | done (generic) | done (`MicrovmPortbView`: `rx`, `tx` as `Seq<u8>`; also its saved state's view; `saved_view` the identity; `wf()`: each buffer at most `BUFFER_MAX`) | contract on the trait impl (`verus_verify`, bodies `external_body`) |
| `pic` | done (generic) | done (`DualPicView`: two PICs' registers and roles; `view.saved_state()`, without `primary`; `wf()`: ICW2's low bits zero, the first PIC primary) | contract on the trait impl (`verus_verify`, bodies `external_body`) |
| `ioapic` | done (generic) | done (`IoApicView`: ID, index, entries' bits, line levels; its saved state's view is the state's meaning: empty `line_levels` as all low, pending interrupts delivered; `saved_view` delivers pending interrupts; `wf()`: one level per entry) | contract on the trait impl (`verus_verify`, bodies `external_body`) |
| `rtc` | done (generic) | done (`RtcView`: `addr`, CMOS bytes, the transaction mask; its saved state's view is the same, without the time fields `clock_time_millis` and `time_valid`; `wf()`: no transaction) | contract on the trait impl (`verus_verify`, bodies `external_body`) |
| virtio-mmio | done (generic) | done (`VirtioMmioView`: the transport's saved registers, feature banks, queues' configuration and progress, the inner device's captured private state, the interrupt status; also its saved state's view; `saved_view` keeps all, with the private state to save; `wf()`: two feature banks). **Trusted boundary**: the interrupt status (a guest-memory word in the microVM; see "virtio-MMIO's interrupt status: the guest-memory boundary") | contract on the trait impl (`verus_verify`, bodies `external_body`); needs code changes 52 and 53; the interrupt status through the guest-memory boundary (code change 58) |

**Saved states that hold a `Vec`**: resolved 2026-10-27 by comparing saved
states by their views ("`SaveRestore`'s contract on views"): a saved
state's view holds `Seq`s for its `Vec`s.

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
   `&mut StateUnits` (registration needs `&StateUnits`).

   **No unit is removed during an operation either** (checked 2026-10-27),
   so the operations' contracts ("the same units") stand. The type allows
   it: a `UnitHandle` removes its unit when dropped, through a weak pointer
   to the registry, from any task. But in OpenVMM every unit handle (or
   `SpawnedUnit`) is owned by `LoadedVm`'s state (`LoadedVmInner`: the
   chipset devices and line sets, the partition unit, the vmbus server and
   devices, vmtime, the time-ABI unit, the input distributor, and the
   hot-plugged devices in `pcie_hotplug_devices` / `dynamic_vpci_devices`).
   An operation holds `LoadedVm` exclusively (`&mut self`), so no handle can
   be dropped while it runs; removals happen in `LoadedVm`'s own request
   handlers (hot-unplug) or when `LoadedVm` is dropped, between operations.
   (Had a removal happened during `LoadedVm::save`, the snapshot could hold
   a removed unit's state while its inventory, read after the units' save,
   omits it; restore would then reject it.) openhcl keeps its handles in
   its own structures and was not checked.

   *Proof hint:* the argument is ownership and borrowing, which a proof of
   `StateUnits`' operations alone cannot see (removal is in `UnitHandle`'s
   `Drop`, outside Verus' view). It may need removal (and registration) made
   visible, for example through `StateUnits` or a ghost token held by each
   handle and shared with `StateUnits`, or the argument made at the
   `LoadedVm` level.
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

Not yet done: the production unit side (each `StateUnit`; async trait
functions, gap 1, are supported since Verus patch 0004); `run_unit` and `StateRequest::apply` now use
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

## Executable code changes (normal builds)

Every change to code that a normal (non-Verus) build compiles: code,
data types, signatures, and dependencies. Changes that exist only in Verus
builds (`cfg(verus_keep_ghost)` items, spec and proof files, ghost fields,
`verus_spec` attributes) are not listed. Each row names its entry in
`SNAPSHOT-SPEC-CODE-CHANGES.md` ("CC n"). Unless a row says otherwise, the
change preserves behavior: same results, same order of effects.

**Data types and public signatures**

| CC | Where | Change |
|---|---|---|
| 9 | `vmm_core/state_unit` | `StateUnits`' registry lock: `Arc<parking_lot::Mutex<Inner>>` becomes `Arc<InnerMutex>` (`mutex_t::Mutex`, wrapping `parking_lot::Mutex<Option<Inner>>`); also `UnitHandle::inner`, `StateUnitsInspector::inner` |
| 27 | `vm/devices/chipset` (PIT) | `save_restore::state` becomes `pub(super)` |
| 38 | `vmm_core/state_unit` | `StateUnit` gains the supertrait `StateUnitSpec` (in normal builds an empty trait implemented for every type, so no impl changes) |
| 43 | `vmm_core/state_unit` | new `unit_channel_for()` (normal builds: `mesh::channel()`) and `UnitBuilder::spawn_with_channel` (`spawn` now calls it with `unit_channel()`) |
| 43 | `vmm_core/virt` | `TimeAbiMsrs`: `derive(Debug, Default)` becomes `cfg_attr(not(verus_keep_ghost), derive(..))` (same in normal builds) |
| 43 | `openvmm_core` | `TimeAbiUnit::new` (new constructor) |
| 42 | `vmm_core/vmotherboard`, `vmm_core` | new constructors `LineSetUnit::new`, `ChipsetUnit::new`, `KeeperUnit::new` (each also takes the unit's receiver, unused in normal builds) |
| 44 | vmcore and every device lock | new `vmcore::device_state::DeviceSlot<T>` (normal builds: holds just the device; `Deref` / `DerefMut` to it), `DeviceMutex<T>` = `CloseableMutex<DeviceSlot<T>>`, `DeviceSnapshot` (empty trait for every type). **Every chipset device's lock changes type** from `CloseableMutex<T>` to `CloseableMutex<DeviceSlot<T>>`, including in public APIs: vmotherboard's device builder (`add`, `add_async`, `try_add`, `try_add_async` return `Arc<DeviceMutex<T>>`), `ArcMutexChipsetServicesFinalize::finalize`, `add_dyn_device`, the PCI/PCIe registration traits, `ArcMutexChipsetDeviceUnit::new`; `vpci`'s bus and device APIs (`VpciBus::new_unoffered`, `offer_registered`, ...); `chipset_arc_mutex_device`'s builder and test chipset; `vmm_core::device_builder`; `openvmm_core`'s `dispatch.rs` (PCIe root complexes, erased devices, hotplug) |
| 44 | `vmm_core/vmotherboard` | the private `DynDevice` trait gains the supertrait `DeviceSnapshot` |
| 46 | `vmm_core/state_unit` | new public trait `InspectConcurrently`; **`run_async_unit`'s bound changes** from `for<'a> &'a T: StateUnit` to `T: StateUnit + InspectConcurrently`; **`StateRequest::apply_with_concurrent_inspects`'s signature changes** from `(self, unit: &mut &T, recv)` to `(self, unit: &mut T, inspector: &impl Inspect, recv)` |
| 46 | `vmbus_server`, `vmbus_channel`, `vmbus_relay` | new public types `VmbusServerInspector`, `ChannelInspector`, `HostVmbusTransportInspector`, and methods `VmbusServer::inspector`, `ChannelHandle::inspector`, `SimpleDeviceHandle::inspector`, `HostVmbusTransport::inspector` |
| 46 | `vmm_core` (`vmbus_unit.rs`), `underhill_core` (`vmbus_relay_unit.rs`) | `VmbusServerUnit`, `ChannelUnit<T>`, `SimpleChannelUnit<T>`, `VmbusRelayUnit` implement `StateUnit` for `X` instead of `&X`, and newly `InspectMut` and `InspectConcurrently` |
| 47 | `vmbus_channel` | **`VmbusDevice`'s supertraits change** from `Send + Any + InspectMut` to `Send + 'static + IntoAny + InspectMut`; new public trait `IntoAny` (`fn into_any(self: Box<Self>) -> Box<dyn Any>`), with a blanket impl for every sized `T: Any` (returns `self`) |
| 47 | `vmm_core` (`vmbus_unit.rs`) | `ChannelUnit<T>`, `SimpleChannelUnit<T>`: a Verus-only tuple field (normal builds: unchanged); `derive(Debug)` becomes `cfg_attr(not(verus_keep_ghost), derive(Debug))` (normal builds: unchanged; Verus builds: a hand-written `Debug` with the derived output and bounds); new private constructors `ChannelUnit::new`, `SimpleChannelUnit::new` |

**Code**

| CC | Where | Change |
|---|---|---|
| 1 | `openvmm_core` | `LoadedVm::restore_from_snapshot` extracted from `make_ids` |
| 11 | `vmm_core/state_unit` | `UnitBuilder::spawn` creates the channel with `unit_channel()` (normal builds: `mesh::channel()`) |
| 16 | `vmm_core/state_unit` | `run_op` and helpers restructured: `state_change` an `async fn`, new `call_unit`, `wait_then_change`, `join_all_timed`; loops over keys with `get_mut` instead of `iter_mut().filter`; `get` instead of `get_key_value` in `restore` |
| 23 | `openvmm_core` | `LoadedVm::quiesce_for_snapshot` split (new `quiesce_and_save`) |
| 27 | `vm/devices/chipset` (PIT) | the bodies of `start`, `reset`, `poll_device`, `io_read`, `io_write`, `save`, `restore` moved into inherent methods, which the trait methods call |
| 31 | `vmm_core/state_unit` | state requests sent and answered through the mesh RPC wrapper (`call`, `recv`, `recv_msg`, `split`, `complete`) instead of `mesh`'s own calls; `StateRequest::apply` without async closures |
| 32 | `mesh_rpc_verus` (new crate), `openvmm_entry`, `openvmm_core`, `vmm_core` | the wrapper moved into its own crate; `VmRpc`'s snapshot requests and `PartitionRequest`'s `StopVps` / `Snapshot` use it (new `PendingCall::recv_failable`) |
| 34 | `vm/vmcore` | `ProtobufSaveRestore`'s blanket impl: `match` instead of `map` and `?` |
| 35 | `vmm_core/state_unit` | new `registry.rs` (`Inner::insert_unit`, `remove_unit`), called by `UnitBuilder::build` and `UnitHandle::remove_if` instead of the map's `insert` / `remove` |
| 40 | `vmm_core/state_unit` | `StateRequest::apply`: errors converted with `match` and `anyhow_error` / `save_error` / `restore_error` instead of `map_err(RemoteError::new)`; replies through new `reply`; `Inspect` through new `inspect_unit`; `run_unit` is a `loop` calling new `run_unit_step`; `no_reply_ghost` removed |
| 41 | `vmm_core/state_unit`, `vmm_core/vmotherboard` | new `handle_request`; the chipset unit's and each line set's task call `run_unit` instead of their own `recv` / `apply` loop (same requests, same order); `ArcMutexChipsetDeviceUnit::run` restructured: its `select!` in new `next_event`, which receives with `recv_msg` instead of `state_change.next()`, its poll in new `poll_now`; its two spawn sites await `run` inside an `async` block |
| 42 | `vmm_core/vmotherboard`, `vmm_core` | the line sets', chipset unit's, and time keeper's tasks construct their units with the new constructors |
| 43 | `vmm_core/virt` | `TimeAbiMsrs`: the MSR value's loads and stores go through `load_control` / `store_control` (normal builds: the same `Relaxed` load and store) |
| 43 | `openvmm_core` | the time-ABI unit is spawned with `spawn_with_channel(.., unit_channel_for(), ..)`, awaiting `run_unit` inside an `async` block |
| 44 | about 15 files (see CC 44) | every device lock created as `CloseableMutex::new(DeviceSlot::new(dev))`; the fuzzer passes `&mut **locked_dev` (one more dereference); OpenHCL's fallback MMIO device wrapped; the chipset device unit's two spawn sites use `spawn_with_channel(.., unit_channel_for(), ..)` |
| 46 | `vmm_core/state_unit` | `run_async_unit` takes the unit's inspector once and passes the unit as `&mut`; `apply_with_concurrent_inspects` sends inspects that arrive during a state change to the inspector instead of to the unit (for every production user, the inspector sends the same `Inspect` request to the same task, through a clone of the same sender; same queue, same order; see "The vmbus units") |
| 47 | `vmbus_channel` | `ChannelHandle::revoke` gets the device back with `.into_any()` instead of the `as Box<dyn Any>` upcast (same `Box`, same concrete type, same downcast; see "The vmbus units") |
| 47 | `vmbus_channel` | `Device::run_channel` and `Device::handle_state_request` moved into their own `impl Device` block (bodies unchanged); `handle_state_request`'s `#[instrument]` applied only in normal builds (`cfg_attr(not(verus_keep_ghost), instrument(..))`; normal builds: unchanged) |
| 47 | `vmbus_channel` | `Device::run_channel` takes `self` instead of `mut self`; the body starts with `let mut this = self;` and uses `this` (Verus does not support a `mut self` parameter; private method, same behavior) |
| **52** | `vm/devices/virtio/virtio` (`transport/core/restore.rs`, `mmio.rs`, `pci.rs`) | **Behavior change, small. Scope: every virtio device (net, console, fs, block, ...) on both the MMIO and the PCI transport**, since the change is in their shared transport code. The virtio transports' `save` **copies** the inner device's captured private state instead of **taking** it: `take_device_state` (`captured.take()`) becomes `captured_device_state`, which clones the `SavedStateBlob` and keeps the capture until the next `start` / reset (which clear it, as before). Effect: the first `save` after a `stop` is unchanged; **a second `save` before the next `start` now returns the same state instead of failing** ("device state was not captured"); `save` without a `stop` still fails; a failed capture is still returned (and dropped) by the first `save`. Cost: one blob copy per save. Why: `SaveRestore`'s contract (`save` keeps the device's view and returns what the view determines). The same change against `origin/main`, for OpenVMM developers: `verus-patches/openvmm-0001-virtio-save-keeps-captured-device-state.patch` |
| **53** | `vm/devices/virtio/virtio` (`transport/core/restore.rs`) | **Behavior change, small. Scope: every virtio device (net, console, fs, block, ...) on both the MMIO and the PCI transport.** `restore(D)` puts the device's private state `D` in the **captured** slot too, not only the **staged** one (`TransportRestore::stage`, one added line). Before, `save` (which reads the captured slot) after a `restore(D)` and before the next `start` did not see `D`: after a fresh VM or a reset (OpenVMM's flows) it **failed** ("device state was not captured"); after a `stop` with no reset (no OpenVMM flow today) it returned the state from **before** the restore. Now it returns `D`. `start` and reset are unchanged (they clear the captured slot; `start` hands `D` to the device task). Cost: one blob copy per restore. Why: `SaveRestore`'s contract (after `restore(state)`, the device saves `state`). For OpenVMM developers: `verus-patches/openvmm-0002-virtio-restore-sets-captured-device-state.patch` (apply after `openvmm-0001`) |
| 47 | `vmm_core` (`vmbus_unit.rs`) | `offer_channel_unit`, `offer_simple_device_unit`, `offer_vmbus_device_handle_unit` spawn with `spawn_with_channel(.., unit_channel_for(), ..)` instead of `spawn(..)`, and build their units with `new` (normal builds: `unit_channel_for()` and `unit_channel()` are both `mesh::channel()`, so the same channel) |
| 54 | `vm/devices/virtio/virtio` (`transport/core/restore.rs`) | `TransportRestore`'s validator (a `Box<dyn Fn>`) is wrapped in a new private struct `Validator(DeviceStateValidator)`: built with `Validator(device.device_state_validator())`, called as `(self.restore.validator.0)(..)`. Same validator, same calls (Verus cannot declare a `Box<dyn Fn>`, so as a direct field it would hide `TransportRestore`'s other fields) |
| 55 | `vmm_core` (`partition_unit.rs`) | `PartitionUnitRunner`'s `topology` field is wrapped in a new private struct `Topology(ProcessorTopology)`: built with `Topology(..)` (also in a test), read as `self.topology.0` (its `vp_count` and its inspect field). Same topology, same uses (Verus cannot declare `ProcessorTopology`: its generic bound needs `AsRef`, which uses an unstable marker). (The two Verus-only lines this change added to the unit's task closure are gone with code change 56.) |
| **56** | `vmm_core` (`partition_unit.rs`) | **The partition unit's runner is built inside the unit's task, after its channel exists, instead of before `spawn`** (see "The partition runner's construction order" below). `PartitionUnit::new` now computes the runner's parts (`RunnerParts`: the partition, the VP set, the halt and request receivers, the client notifier, the topology, and with `gdb` the debugger state) where it built the runner, in the same order and with the same error (`MissingGuestMemory`); the task (`run_partition_unit`, new) assembles the runner from them (`PartitionUnitRunner::new`, new) and runs it. New private struct `RunnerParts`, functions `PartitionUnitRunner::new` and `run_partition_unit`; the spawn closure is `\|recv\| async move { run_partition_unit(parts, recv).await }`. Same behavior |
| **57** | `vmm_core` (`input_distributor.rs`) | **The input distributor's `Inner` is built by a new function `Inner::new()`** instead of a struct literal inside `InputDistributor::new` (see "The input distributor's initial share" below). Same fields, same values (`running: false`, two empty forwarders); in Verus builds the share starts as `None` (a new Verus-only helper `Inner::no_share()`), not as a placeholder. `Forwarder::new` gets a cfg'd `verus_verify(external_body)` attribute. Same behavior |
| 58 | `vm/devices/virtio/virtio` (`transport/mmio.rs`, `transport/mmio/interrupt.rs`) | `restore`'s inline interrupt-status block (lock, `restore_status`) moved into a new `MmioTransport::restore_interrupt_status(&mut self, status)` (same lock, same call, same order), so that it can carry a contract. Trusted boundary contracts on it and `read_interrupt_status` (see "virtio-MMIO's interrupt status: the guest-memory boundary"). Same behavior |
| 61 | `vmm_core/vmotherboard` (`chipset/backing/arc_mutex/state_unit.rs`) | The private trait `DynDevice`'s `save` and `restore` moved into a new private trait `DynDeviceState`, which `DynDevice` has as a supertrait (it takes over the `DeviceSnapshot` supertrait); its blanket `impl<T: VmmChipsetDevice> DynDeviceState for T` calls `ProtobufSaveRestore::save` / `restore` (before: `self.save()` / `self.restore(state)` in `DynDevice`'s impl, the same functions). Callers are unchanged (supertrait methods are callable through `dyn DynDevice`). Reason: `DynDevice` is `async_trait`, whose `dyn Future + Send` Verus cannot express, so `save` / `restore` need a trait of their own to carry contracts and be checked. Same behavior |
| 62 | `vm/chipset_device_resources` | Verus-enabled: `[package.metadata.verus]`, dependency `vstd`, new `build.rs` (declares the `verus_keep_ghost` cfg), cfg'd imports of `verus_spec` / `verus_verify` in `lib.rs`; the rest of the change is Verus-only (cfg'd attributes and a `verus!` block) |
| **63** | `vmm_core` (`partition_unit/vp_set.rs`, `vp_set/boundary.rs`) | **`VpEvent::Stop` carries an `Rpc<(), ()>` instead of a `mesh::OneshotSender<()>`**: `VpSet::stop` and `stop_at_io_boundary` send `vp.send.call(VpEvent::Stop, ())` and await it with `.ok()` (before: a `mesh::oneshot()` pair); the VP thread's two `Stop` arms answer `complete(())` (before: `send(())`); tests likewise. Same one-shot reply, same order; a VP whose runner is gone is still ignored. Reason: the reply carries the VP's half back to `VpSet` (the VP protocol) |
| 65 | `vmm_core` (`partition_unit/vp_set.rs`) | `VpSet::add` creates the VP's channel with a new private `vp_channel()` (normal builds: `mesh::channel()`, through `mesh_rpc_verus::channel`) instead of `mesh::channel()`; binding `let (mut send, recv)` with `cfg_attr(not(verus_keep_ghost), expect(unused_mut))`; new imports `vstd::prelude::Ghost`, `Tracked`. Same channel |
| 66 | `vm/vmcore/vm_topology` (`processor.rs`) | `VpIndex::new` (`const fn`) moved, same body, into a `verus!` block (no longer cfg'd; normal builds: the same `const fn`), so that it can carry a contract (Verus's attribute form does not take a `const fn`). Same function |
| 48 | `vm/devices/chipset` (`microvm.rs`) | `MicrovmShutdown`'s and `MicrovmSnapshotRequest`'s `SaveRestore::restore` take `_state: Self::SavedState` instead of the pattern `NoSavedState: Self::SavedState` (Verus allows only identifier parameters; both bind and drop the unit value) |
| 49 | `chipset` and the 22 crates below | a `vmcore::placeholder_view!(..)` line before each of 59 `impl SaveRestore for X` blocks (expands to nothing in normal builds) |
| 49 | `vm/vmcore` | **`SaveRestore` gains the supertrait `SaveRestoreView`**: new public trait, in normal builds empty and implemented for every type (so no impl changes); in Verus builds `View` (see "`SaveRestore`'s contract on views"). New exported macro `vmcore::placeholder_view!` (expands to nothing in normal builds) |
| 50 | `vm/vmcore` | **`SaveRestore::SavedState` gains the bound `SaveRestoreView`** (`type SavedState: SaveRestoreView;`; normal builds: an empty trait every type implements, so no impl changes) |
| 65 | `vmm_core` (`partition_unit/vp_set.rs`) | **`VpSet::restore`'s parameter type changes** from `impl IntoIterator<Item = (VpIndex, SavedStateBlob)>` to `Vec<(VpIndex, SavedStateBlob)>` (its one caller already passes a `Vec`) |

**Dependencies and build**

| CC | Where | Change |
|---|---|---|
| 2, 4, 8, 26, 27, 32, 39, 43 | `openvmm_defs`, `openvmm_core`, `openvmm_entry`, `vmcore`, `state_unit`, `closeable_mutex`, `chipset`, `vmm_core`, `vmotherboard`, `virt` | Verus-enabled: `vstd` dependency, `build.rs` declaring the `verus_keep_ghost` cfg (and `verus_keep_ghost_body` in `state_unit`), `[package.metadata.verus]` |
| 39 | `vmm_core` | new dependency `vmbus_ring` (already in its dependency graph) |
| 41 | `vmm_core/vmotherboard` | new dependency `mesh_rpc_verus` |
| 44 | `chipset_arc_mutex_device`, `chipset_device_fuzz` | new dependency `vmcore` (no cycle) |
| 46 | `vmm_core/state_unit` | dev-dependency (tests only): `inspect` with the `initiate` feature, for the new test |
| 47 | `vmbus_channel` | Verus-enabled: `[package.metadata.verus]`, dependencies `vstd` and `mesh_rpc_verus`, new `build.rs` (declares the `verus_keep_ghost` cfg), a cfg'd `vstd` import in `lib.rs` |
| 64 | `vm/vmcore/vm_topology` | Verus-enabled: `[package.metadata.verus]`, dependency `vstd` (`Cargo.lock`), `build.rs` declares the `verus_keep_ghost` cfg, a cfg'd `vstd` import in `lib.rs`; the rest is Verus-only (`VpIndex`'s cfg'd attributes, a cfg'd `verus!` block) |
| 67 | `vm/vmcore/guestmem` | Verus-enabled: `[package.metadata.verus]`, dependency `vstd` (`Cargo.lock`), new `build.rs` (declares the `verus_keep_ghost` cfg); the rest is Verus-only (a cfg'd module declaring `GuestMemory` to Verus) |
| 51 | `vm/devices/chipset` | new direct dependency `mesh_rpc_verus`, for Verus: portb's fields include `mesh::Receiver` and `Rpc`, which that crate declares to Verus. Normal builds compile and link it, but no code uses it |
| 54 | `vm/devices/virtio/virtio` | new direct dependencies `mesh_rpc_verus` (its declaration of `mesh::Sender`, the transport core's device sender) and `async-task` (`pal_async::task::Task` is `async_task::Task`, which Verus must be told about), both for Verus, both already in the build graph; referenced by cfg'd `use .. as _;` lines in `lib.rs`. Normal builds compile and link them, unused |
| 54 | `vm/devices/virtio/virtio_spec` | Verus-enabled: `[package.metadata.verus]`, dependency `vstd`, new `build.rs` (declares the `verus_keep_ghost` cfg), a cfg'd `use vstd::prelude::verus_verify;` in `lib.rs` |
| 49 | `amd_iommu`, `chipset_legacy`, `cxl_spec`, `firmware_pcat`, `floppy`, `floppy_pcat_stub`, `framebuffer`, `generation_id`, `hcl_compat_uefi_nvram_storage`, `ide`, `intel_vtd`, `missing_dev`, `pci_bus`, `pci_core`, `pcie`, `smmu`, `uefi_nvram_storage`, `vfio_assigned_device`, `vga`, `virtio`, `vpci`, `watchdog_core` | Verus-enabled: `[package.metadata.verus]`, dependency `vstd` (for `vfio_assigned_device`, in its Linux-only dependencies), new `build.rs` (declares the `verus_keep_ghost` cfg), a cfg'd `use vstd::prelude::verus as _;` in `lib.rs` |
| 38 | workspace `Cargo.toml` | `[patch.crates-io] verus_builtin_macros` = the patched copy in `verus-patches/verus_builtin_macros` (used by normal builds too; the change affects only `verus_spec(with ...)` on traits and `cfg_attr`'d `verus_spec`, which normal builds strip); `Cargo.lock` follows |
| 40 | `vmm_core/state_unit` | `vstd::prelude` imported in both builds (its macros erase ghost code in normal builds) |

## Recipe: verifying another device

Updated 2026-10-27 for "Chipset devices: the share in the device's lock".
A chipset device's unit and its share are generic now
(`ArcMutexChipsetDeviceUnit`, `DeviceSlot`); what is left per device is its
device-level spec. The PIT's files are the example: `pit/mod.spec.rs`
(view, `saved_view`, `wf()`), `pit/mod.proof.rs` (declarations), and
`pit/mod.rs` (entry points).

1. **Crate setup** (once per crate): `[package.metadata.verus] verify =
   true`, `vstd` dependency, `build.rs` declaring the `verus_keep_ghost`
   cfg, a Verus-only vstd import (`use vstd::prelude::verus as _;` if the
   glob would clash). Most device crates in the verified graph already
   have this (code change 49).
2. **Declare the device's types to Verus** (`external_type_specification`):
   its state types with their fields visible (for the view), its runtime
   glue as opaque types, and its saved-state types with their fields
   visible.
3. **Views and `saved_view`**: a view of the snapshotable state (and the
   configuration that affects `save` / `restore`), its `inv()`; a view of
   the saved-state type (its fields without time; `Seq`s for `Vec`s); and
   `SaveRestoreSpecImpl::saved_view(view)`, the saved state's view of a
   device with that view. Replace the device's two `placeholder_view!`
   lines. Say what the views leave out (state that is not saved; time).
   Put the spec module where it can see the saved-state types (the PIC's
   and the RTC's are inside their `save_restore` modules).
4. **`SaveRestore`'s contract**: `save` keeps the view and returns a state
   whose view is `saved_view(view)`; a successful `restore(s)` makes
   `saved_view(view)` equal to `s@`. Mark the trait impl `verus_verify`,
   its methods `external_body` until proved. The snapshot follows
   (`ProtobufSaveRestore`'s blanket impl, `DeviceSnapshot`); no
   per-device ghost.
5. **Entry points with contracts**, where needed: inherent methods holding
   the trait methods' bodies (Verus cannot add contracts to a foreign
   trait's impl methods), keeping `wf()`.
6. **Device-specific gaps**: activity of its own to stop; `advance_time`
   (like the RTC) or `quiesce_input` / `resume_input`; cross-device effects
   (interrupt lines it drives, vmtime timers); a `restore` that normalizes
   the saved state (breaks "`saved_view(view) == s@`", unless only time is
   normalized); saved-state roots using
   the derive's `transparent` or `prost` mode.
7. **Proofs** (proof phase): remove `external_body`.

Not needed per device: the unit, the share, the lock invariant, the blob
contract, and `ProtobufSaveRestore`'s blanket impl, which are generic.

Units that are not chipset devices have their own `StateUnitSpec` ("Production
units: the contract on `StateUnit`").

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
     `StateUnit` (`vmm_core/state_unit/src/lib.rs:145`) is such a trait.
     Fixed by Verus patch 0004 (2026-10-27).
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

- **VMBus** (decided 2026-10-27, user): the microVM has no VMBus. Its load
  path sets `with_hv = false` (`openvmm_entry/src/lib.rs:1381`), and VMBus
  exists only with it (`lib.rs:2244`: `vmbus: (with_hv &&
  !opt.no_vmbus).then_some(..)`); the microVM also refuses VMBus serial
  (`microvm/config.rs:146`). So the vmbus server, the channels, the vmbus
  devices behind them (`netvsp`, `storvsp`, the 11 simple devices), and
  OpenHCL's relay are not on the microVM snapshot path. The spec written so
  far stays (code changes 46, 47; "The vmbus units"); the remaining vmbus
  items are parked: the vmbus server's share, a channel task starting
  `Running` (open item 1 there), and the vmbus devices' views and
  `saved_blob()` (`netvsp`, `storvsp`, `SimpleDeviceWrapper` and the simple
  devices, with `task_control`'s view).

- **Guest RAM.** It has no in-memory Rust structure (it is a file-backed
  mapping). RAM will be removed from the spec rather than left unverified.
- **Protobuf encoding/decoding** of `SavedState` (`ProtobufMessage`). Each
  unit's `SavedStateBlob` is also protobuf-encoded; that is covered by a
  trusted contract, not verified (see gap 4 above).
- **`SnapshotManifest`** and snapshot files (`manifest.bin`, `state.bin`,
  `memory.bin`), the controller (`VmController`), and publication.
- **Guest time** (decided 2026-10-27). The snapshot spec covers equality:
  after a restore each unit's state is its saved state. Guest time is the
  one place where that is wrong by design: the clocks must continue,
  advanced by the real downtime, all by the same amount. The wanted
  property: each VP's TSC = the TSC at the capture anchor + downtime x the
  declared TSC rate, and VM time (and so the RTC and device timers) = VM
  time at the cut + the cut-to-anchor gap + downtime. Proving it would
  take many assumptions (the backend's TSC/host-clock pairing at the
  anchor, the host clocks' meaning and monotonicity, the hypervisor's TSC
  set and LAPIC advance, the controller's validated time inputs, vmtime's
  saved stop time), so it is not worthwhile for now. How it is kept out:
  the units' views leave time out (the time-keeping units' saved-state
  views have no time fields, and a blob is determined by its saved state's
  view, `of_view`), so every equality claim is about
  everything but time, and the clock restore changes nothing in the view;
  the time capture (`TimeCapture`, the VM-time cut) is unconstrained. What
  it would need, if taken up: a trusted
  contract on the backend's capture anchor (VP 0's TSC and the host sample
  are one instant within `pairing_ns`), vmtime exporting its saved stop
  time, the cut rule (`vm_time_cut_to_anchor_ns = capture_monotonic_ns -
  stop_host_time`), and the restore-side equations. (Under the time ABI the
  partition's saved TSC is dropped on restore, `omit_saved_tsc`, so no rule
  ties `capture_tsc` to the partition's saved state.)

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
   `lemma_load_restores` (see "Restore from snapshot"). (It first needed
   `LoadedVmView::time_units`; that is gone since the views leave out
   time.)
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

- **GAPS: plain-Rust impls on the microVM path** (found 2026-10-28 while
  checking which impls `SaveRestore`'s contract enforces; the full audit,
  "The trust audit", adds gap G2, `DynDevice`). A trait impl outside Verus is assumed to
  meet its trait's contract, silently.
  - **`ErasedChipsetDevice`'s `ProtobufSaveRestore`**
    (`vm/chipset_device_resources/src/lib.rs:222`, forwarding to its
    `Box<dyn DynChipsetDevice>`). The microVM's chipset devices from
    resources (`ioapic`, `microvm-portb`, `microvm-shutdown`,
    `microvm-snapshot-request`, `pic`, `pit`; `base_chipset.rs:295`) are
    wrapped in it, so their units see the wrapper's uninterpreted
    `saved_blob()`, not their devices' specs. To fix: carry the blob
    through `dyn DynChipsetDevice` (as `DeviceSnapshot` does for `dyn`
    devices) and put the wrapper's forwarding impl in Verus.
  - **`WrappedPartition`'s `SaveRestore`** (`openvmm_core/src/partition.rs`,
    placeholder view): the partition's VM-wide hypervisor state
    (`AccessVmState`) is unspecified. With the VP / partition layer.
  - **`WrappedVp`'s `SaveRestore`, `BoundVp`'s `ProtobufSaveRestore`**:
    the VP layer (known TODO).
  - Fixed: **the PIT's `SaveRestore` impl** was plain Rust (code change 60).

- **TODO, in progress: the VP layer** (2026-10-28). Done: the hypervisor
  boundary (layer A, "The hypervisor boundary"); the `VpEvent` protocol
  and `VpSet`'s contracts ("The VP protocol", code changes 63 to 65; its
  "Open" list). Then, by the proof agent: `save_all` / `restore_all`,
  `WrappedVp`, `BoundVp`, the VP threads, `VpSet`. The partition unit's VPs
  run on their own threads; `VpSet` asks each to stop, save, and restore
  (`vmm_core/src/partition_unit/vp_set.rs`). The VPs' saved state is now
  `VpSetView::saved_vps()`, from `VpSet`'s halves. Originally to specify: `VpSet` and the VP
  threads' request protocol (like the units' FracGhost shares, per VP),
  `BoundVp`'s `ProtobufSaveRestore` (trusted today), and below it the
  hypervisor's VP save and restore (trusted: KVM, WHP), with each VP
  blob's view leaving out time (the TSC, the LAPIC timers; also
  `omit_saved_tsc`'s restore, which drops the saved TSC).
- **TODO, not done: the partition runner's other events.** Besides state
  requests, `PartitionUnitRunner::run` handles halt events and partition
  requests (`ClearHalt`, `SetInitialRegs`, `AcceptInitialPages`,
  `StopVps`, `Snapshot`, ...). Those that change the saved state (the halt
  reason, VP registers) must keep the share's rule: allowed while running
  or before the first stop or restore (snapshot `Unknown`); while stopped
  after a snapshot is recorded, only if the change is reflected in the
  share, which the stopped unit cannot do alone. To check per request in
  the proof (or as a protocol on `PartitionRequest`, whose `request_ok` is
  still uninterpreted). The runner's `stop` also handles pending halt
  requests after stopping the VPs, so the halt reason is settled at
  `stop`.

- **TODO, not done: the virtio-MMIO interrupt-state lock protocol, for the
  legacy modes (not used by the microVM).** In the legacy interrupt modes
  the status is `InterruptState.status`, behind
  `Arc<parking_lot::Mutex<InterruptState>>`, written by the transport, the
  device task's interrupt closures, and (legacy with accelerated acks, on
  Linux) the acknowledgement worker. To verify it: a lock with an
  invariant (an executable type change, as for `StateUnits`, code change
  9) and a FracGhost share for the status (the lock's half and the
  transport's, the transport's half in the lock while the device runs),
  with ghost state through the interrupt closures. Also to check there: a
  guest acknowledgement written just before `stop` may be handled by the
  acknowledgement worker after `stop`, changing the status of a stopped
  device. The microVM uses the shared-status mode ("virtio-MMIO's
  interrupt status: the guest-memory boundary"), so neither applies to it.
- **virtio-MMIO's other accessors**: resolved 2026-10-28 (user), now
  defined, not trusted. `TransportRestore::captured()` is a closed spec
  function on the real field (`transport/core/restore.rs`): the validator,
  a `Box<dyn Fn>` that Verus cannot declare, moved into its own opaque
  struct `Validator` (code change 54), so `TransportRestore` is
  `verus_verify`. The feature banks and the status come from
  `VirtioDeviceFeatures::bits()` / `bank_bits(i)` and
  `VirtioDeviceStatus::bits()`, closed spec accessors in `virtio_spec`
  (now Verus-enabled), matching `bank`'s definition.
- **virtio-mmio: `save` takes the captured device state**: resolved
  2026-10-28 (user): option A, code change 52. `save` copies the capture
  instead of taking it. The same change against `origin/main`, for
  OpenVMM developers, is `verus-patches/openvmm-0001-virtio-save-keeps-captured-device-state.patch`
  (see `verus-patches/README.md`, "OpenVMM patches"). Next: virtio-mmio's
  spec (its view holds the capture). Separate, later: the inner devices'
  state (the virtio device task's stop/start protocol; net, console, fs,
  block) is its own layer.
- **A clean design document** (requested 2026-10-27, user; to write once
  the design settles, as it may still be refactored): a fresh document,
  separate from this working log, stating clearly the device-level spec
  (views, saved-state views, `saved_view`, `device_wf`), the `SaveRestore`
  trait spec (and `ProtobufSaveRestore`, where blobs enter), the RPC (the
  mesh RPC wrapper and the request protocol), `LoadedVm` (save, restore,
  quiesce, restore from snapshot), and the FracGhost (the units' shares,
  `DeviceSlot`, the lock invariant).

- **`cargo xtask fmt` lint failures from earlier spec work** (found
  2026-10-27, not caused by code change 49): `closeable_mutex` reported
  unused in `vmm_core/state_unit`, `vmm_core`, `openvmm_core`,
  `vm/devices/chipset`, `vm/chipset_device_fuzz` (used only in Verus
  builds?); `vmbus_ring` unused in `vmm_core`; unjustified
  `expect(unsafe_code)` in `vmm_core/state_unit/src/lib.rs:43` and
  `openvmm/openvmm_defs/src/rpc.rs:29`; copyright headers in the root
  `Cargo.toml` and `rust-toolchain.toml`; the vendored
  `verus-patches/verus_builtin_macros/Cargo.toml` not workspaced. To fix
  before upstreaming.

1. **Prove the contracts.** Every contract above has an unverified body
   (`external_body`); they are listed in `SNAPSHOT-SPEC-TRUSTED.md` as
   assumed until verified.
2. **Next layer, deferred**: the controller
   (`VmController::handle_guest_snapshot_request`), which needs a decision
   on relating the encoded `ProtobufMessage` to the worker's state (roadmap
   item 3). Decided 2026-10-27: only the worker side (`LoadedVm`) for now;
   the controller after the `LoadedVm` layer is verified.
3. **Unit side**: for the production units, attach the `StateUnit` and
   request-loop contracts (async trait functions are supported since Verus
   patch 0004; `StateRequest::apply` no longer uses async closures; the blob layers,
   `SavedStateBlob` and `ProtobufSaveRestore`, are attached); the units not built on
   `SaveRestore`; cross-unit interrupt effects ("Unit-side contract"). For
   the PIT, see "Next steps (agreed)", item 5.
4. **Other `ProtobufSaveRestore` implementations**: `ChipsetDeviceProxy`,
   `BoundVp`, `ErasedChipsetDevice` are trusted to meet its contract (the
   blanket impl for `SaveRestore` types is verified).
5. **Quiesce path**: specified, except the time capture's content, which is
   out of scope for now (guest time, "Out of scope for now").
6. **Pending proof**: `StateUnits::lemma_view_inv`.
7. **Convention TODOs**: `SavedStateUnit`'s sequence helpers, and the
   trusted wrapper types' spec functions ("Convention TODOs").
8. **Soundness review** of the mesh RPC wrapper (`mesh_rpc_verus`,
   `src/mesh_rpc_t.rs`; the reply-ghost type argument). (The registry
   axioms are gone.)
