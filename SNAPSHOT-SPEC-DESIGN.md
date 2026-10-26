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
  `vmm_core/state_unit/src/mesh_rpc_t.rs`, and the mutex wrapper
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
  `of::<T>` is injective; `new`/`parse` are stated there but attached only
  with the unit side (their `SavedStateRoot` bound needs the protobuf trait
  chain declared to Verus).
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
   (`vmm_core/state_unit/src/lib.spec.rs`): `Unknown`, `NoSavedState`, or
   `Saved(SavedStateBlobView)`, starting `Unknown`. (`NoSavedState` is
   needed because a unit whose save returns `None` has a known state that
   is not a blob.) On `Stop`, `StateUnits` lends its half and the unit
   records what its save would return; `Restore` records `Saved(blob@)`
   (this covers restore on a fresh VM); `Save` only lends. `save` requires
   no half to be `Unknown`. The unit's run loop keeps a ghost-only
   running/stopped flag; its invariant is "while stopped, my half records
   what my save returns".
3. **Stored in `StateUnits`.** Registration is through `&self`, so the
   sender's half stays attached to the `Sender` that `build` (unverified)
   stores in the registry, behind the mutex. The lock invariant
   (`InnerPred`, `lib.proof.rs`): each registered sender's attached half,
   if still there, belongs to its channel and records `Unknown`; registered
   units have distinct names. `StateUnits::collect_halves(&mut self)`
   (verified, Verus-only) takes the attached halves into the Verus-only
   field `StateUnits::units_ghost` (`UnitHalves`: halves by unit id, and
   the collected units' names and ghost ids), which the view reads by name.
   Each `&mut` operation (`stop`, `save`, `restore`) will call it first,
   under `#[cfg(verus_keep_ghost)]`; it has no runtime effect.
   `StateUnits::wf` ties the halves to the collected units.

   Verified code locks the registry with `StateUnits::lock_registry`
   (`registry_t.rs`), which carries the **registry axiom**: the value taken
   out agrees with the collected units (none removed or changed; units
   added since still have their halves attached). No lock invariant can say
   this, because registration and removal are unverified and go through
   `&StateUnits` and `Weak` handles. The axiom and the other trusted items
   are listed in `SNAPSHOT-SPEC-TRUSTED.md`.
4. Done: `LoadedVm::halves` is removed; `LoadedVm`'s view reads
   `state_units@`.

No atomic invariant is needed: exact knowledge comes from holding halves;
the lock invariant only ties the uncollected halves to the registered
senders.

### Request protocol

`MeshProtocol::request_ok` for `StateRequest` (`vmm_core/state_unit/src/lib.proof.rs`).
`k` is the unit's ghost id; the caller (`StateUnits`) holds one half, the
unit the other.

| Request | Ghost sent | Reply must satisfy |
|---|---|---|
| `Stop` | caller's half (lent) | a half back recording a known state (not `Unknown`) |
| `Save` | caller's half (lent), not `Unknown` | the same half back; `Ok(Some(blob))` iff it records `Saved(blob@)`, `Ok(None)` iff `NoSavedState` |
| `Restore(blob)` | caller's half | a half back; on `Ok` it records `Saved(blob@)` |
| `Reset`, `AdvanceTime` | caller's half | a half back, any value |
| `Start`, `QuiesceInput`, `ResumeInput`, `Inspect` | none | no ghost |

A reply to a request that carried the half returns a half also on `Err`,
so the caller keeps it (`save` promises to change nothing).

Verified examples (Verus-only, in `lib.proof.rs`): `example_save_call`, a
caller that lends its half for `Save` and learns that the blob matches it;
and `ExampleUnit::handle`, a stopped unit that answers `Stop`, `Save`, and
`Restore` by the rules (its invariant: its half belongs to the channel and,
unless `Unknown`, records what its save returns). Three injected bugs (`Save`
always `None`; `Restore` ignoring the blob; `Stop` not recording) each fail.

New trusted facts and assumptions:

- `SavedStateBlob::clone` preserves the encoded value
  (`assume_specification`, `vm/vmcore/src/save_restore.spec.rs`; part of the
  blob contract, gap 4).
- `Inspect` requests do not change a unit's saved state (they reach the unit
  through `InspectMut`, with no ghost).
- Channel failures (a unit's task gone) do not happen for a registered unit:
  OpenVMM's `run_op` panics on them, and the ghost would be lost.

Not yet done: the real unit side (`run_unit`, `StateRequest::apply`, each
`StateUnit`) is blocked on async trait functions and async closures (gap 1);
`StateUnits::stop`/`save`/`restore` and `run_op` are not yet verified
against the protocol.

### Mutex wrapper

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

Now in `vmm_core/state_unit/src/mesh_rpc_t.rs` (first drafted in
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
   state. "Frozen" then relies on the closed mutex keeping vCPUs out while
   stopped, a trusted assumption.
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
