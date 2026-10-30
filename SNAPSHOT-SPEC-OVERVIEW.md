# OpenVMM snapshot spec: overview

This document explains, for a reader who knows verification but not
OpenVMM, what the snapshot specification is about and how it is built. It
describes the software first, then what we prove, then the main ideas of
the spec. It leaves out file-by-file details, exact contracts, conventions,
and history; those are in the other documents (see the end).

## 1. The goal

OpenVMM is a virtual machine monitor written in Rust. Its **microVM**
profile can take a **snapshot** of a running VM and later **restore** a
new VM from it. We use Verus to specify, and then verify, the **VM
worker's** side of this: the worker is the part of OpenVMM that runs the
VM, through its main object, `LoadedVm` (section 2). The worker is driven
by a **controller** (`VmController`), whose code we have not modelled yet
(it may be later); for now we specify only the channel between them: what
the controller's requests may carry, and what the worker's replies
promise.

```
Snapshot                                           Restore (a new VM worker)

VmController ── QuiesceForSnapshot ───> LoadedVm   InitializedVm::load(saved state)
(not modelled yet)                        │          │  builds the LoadedVm,
                                          │ quiesce  │  every unit stopped
                                          │ save     ▼
VmController <──── reply: saved state ────┘        LoadedVm::restore_from_snapshot
     │                                               │  each unit gets its saved state
     │ if writing the snapshot fails:                ▼
     └──── ResumeAfterFailedSnapshot ───> LoadedVm   ready to run
                                          restarts
                                          the VM
```

- **Snapshot** (the controller's `QuiesceForSnapshot` request): the worker
  stops every device and processor (*quiesces* them, within a deadline),
  saves their state, and replies with it. The spec: after quiesce nothing
  changes, and the saved state equals the state of every device and
  processor once stopped.
- **After the reply**: the worker cannot run the VM on its own, whatever
  the controller does; only the controller's rollback request can make it
  run again.
- **Rollback** (`ResumeAfterFailedSnapshot`, sent if the controller fails
  to write the snapshot): the worker restarts the VM.
- **Restore** (a new worker, started with the saved state): the worker
  builds the VM with every device and processor stopped, and gives each
  its saved state. The spec: each then has the state it was saved with.

Together, saving and then restoring into a VM with the same devices gives
back the same saved state. This is equality of the state **as the spec
models it**: each device's saved state, without the parts left out below.
It is not a claim that the restored VM behaves identically in every
respect.

Not specified: the controller's code (its checks, copying the disk,
writing the snapshot files), the guest's side, guest RAM (a memory-mapped
file, copied separately), and the guest's notion of time (clocks keep
running while the VM is stopped, so restored timers are adjusted, not
equal).

## 2. The software

### 2.1 Two processes: the controller and the VM worker

```
 openvmm process                               VM worker process (default:
 (openvmm_entry)                               a child process; or the same
                                               process with --single-process)
┌──────────────────────────────┐              ┌──────────────────────────────┐
│ VmController                 │   VmRpc      │ LoadedVm                     │
│  - talks to the user / CLI   │ ───────────> │  - the running VM            │
│  - handles the guest's       │   requests   │  - its devices and vCPUs     │
│    "snapshot me" request     │ <─────────── │                              │
│  - writes the snapshot files │   replies    │                              │
│  - copies the scratch disk   │              │                              │
└──────────────────────────────┘              └──────────────────────────────┘
```

The **controller** (`VmController`, `openvmm/openvmm_entry/`) manages the
VM from outside. The **VM worker** runs the VM; its main object is
`LoadedVm` (`openvmm/openvmm_core/src/worker/dispatch.rs`). The two talk
only through messages: `VmRpc` requests (`openvmm_defs/src/rpc.rs`) sent
over a **mesh** channel. Mesh is OpenVMM's message-passing library; a
request can carry a reply channel (an `Rpc`), so it works like a remote
procedure call.

### 2.2 Inside the VM worker: state units

What `LoadedVm` holds:

```
LoadedVm
 ├─ state_units: StateUnits     the coordinator of the state units (below)
 ├─ inner: LoadedVmInner        everything else: memory, the partition handle,
 │                              the chipset, configuration, ...
 ├─ snapshot_boundary           a guest-requested snapshot in progress
 └─ snapshot_restore            the state of a restore in progress
```

How `StateUnits` drives the units:

```
StateUnits                                each state unit: a task with its own channel
(the coordinator)
    │                                     ┌ unit "pit"         ┐
    │   a StateRequest to each unit       ├ unit "pic"         │ chipset devices
    │   (Start, Stop, Save, Restore, ...) ├ unit "rtc"         │ (one unit each)
    └───────────────────────────────────> ├ unit "ioapic"      │
        each unit replies                 ├ unit "virtio..."   ┘
                                          ├ unit "partition"     the vCPUs and the
                                          │                      hypervisor's VM state
                                          ├ unit "vmtime"        VM time
                                          └ ...                  (time-abi, input,
                                                                  line sets, ...)
```

The VM's components are **state units**; most have state to save (a few,
like the input distributor, have none). Each unit is an asynchronous
**task** with its own mesh channel. `StateUnits` (`vmm_core/state_unit/`)
is the coordinator: it knows every unit by name and drives them all
through the same life cycle by sending each a `StateRequest` over the
unit's channel, as a mesh RPC (section 2.1): the request carries a reply
channel, on which the unit answers (e.g. with its saved state for
`Save`):

```
                 Start
       ┌──────────────────────┐
       │                      ▼
    stopped                running
       ▲                      │
       └──────────────────────┘
                 Stop

  while stopped:  Save     → the unit replies with its state (a blob)
                  Restore  ← the unit takes a blob as its state
                  Reset, AdvanceTime
```

A unit's saved state is a **blob**: its state encoded as a protobuf
message. `LoadedVm::save` asks every unit to save and collects the blobs
into a `SavedState` (unit name → blob, plus the list of unit names, the
"inventory"). `LoadedVm::restore` sends each blob back to its unit.
`StateUnits` sends requests in dependency order (a unit stops after the
units that depend on it), and waits for all replies.

### 2.3 A chipset device and the vCPUs

Most units are **chipset devices**: emulated hardware such as the PIT
(timer), the PIC and IOAPIC (interrupt controllers), the RTC (clock), and
the virtio-MMIO devices (network, console, disks). A chipset device is
reached from two sides:

```
   vCPU threads                                 the device's unit task
   (the guest accesses an I/O port              (handles the StateRequests
    or a memory-mapped register)                 from StateUnits)
        │                                              │
        │ the dispatcher (Chipset) calls               │
        │ the device that owns the address             │
        ▼                                              ▼
   ┌───────────────────────────────────────────────────────────┐
   │ the device's state (e.g. the PIT's registers),            │
   │ behind a lock (CloseableMutex)                            │
   └───────────────────────────────────────────────────────────┘
```

- While the VM runs, **vCPU threads** call into the device whenever the
  guest touches its I/O ports or memory-mapped registers: the vCPU exits
  to OpenVMM, and OpenVMM's dispatcher, an object called `Chipset`, looks
  up which device owns that address and calls the device's handler.
- The device's **unit task** handles the `StateRequest`s (stop, save,
  restore, ...). Each chipset device has its own unit task, but all of
  them run the same code: one generic Rust type,
  `ArcMutexChipsetDeviceUnit`, created once per device.

Both sides take the device's lock (a `CloseableMutex`) to use it, so a
vCPU and the unit never use the device at the same moment.

Note that the lock cannot prevent the vCPUs from accessing a stopped
device. To prevent that, each unit, chipset device or not, may declare
the units it depends on, and `StateUnits` handles the units in that
order. In the microVM (an arrow means "depends on"):

```
   partition (the vCPUs) ─────────────┐
        │                             │
        ▼                             ▼
   chipset unit                     vmtime
        │                             ▲
        ▼                             │
   the chipset devices ───────────────┘

   Also: a device that raises interrupt lines depends on those line sets;
   a device that receives them (e.g. the PIC) is a dependency of them.
   time-abi, input, ...: no declared dependencies, handled in no particular order.
```

So the partition unit (the vCPUs) depends, through the chipset unit, on
every chipset device, and `StateUnits` follows that order:

| Operation | Order | So |
|---|---|---|
| `Stop` (and quiesce) | the units that depend on others first | the vCPUs stop before any device |
| `Start` | the units others depend on first | every device runs before the vCPUs do |
| `Restore`, `Reset` | the units others depend on first | the devices before the partition (all are stopped) |
| `Save` | all at once | all are stopped |

So while any device is stopped, the vCPUs are stopped too.

### 2.4 The partition and its vCPUs

The **partition** is the VM as the hypervisor (KVM, MSHV, or WHP) sees
it: the vCPUs and some VM-wide registers, all kept by the hypervisor. Its
unit's task receives two kinds of requests:

```
StateUnits ──────── StateRequest ────────┐
 (like every unit: Stop, Save, ...)      │
                                         ▼
a handle in LoadedVm ── PartitionRequest ──> the partition unit's task
 (PartitionUnit)        (e.g. StopVps,        ├── one thread per vCPU ──> the hypervisor:
                         StartVps)            │                            the vCPU's registers
                                              └─────────────────────────> the hypervisor:
                                                                           the VM-wide registers
```

- **`StateRequest`s**, from `StateUnits`, as for every unit. Its saved
  state has two parts, both read from the hypervisor: each vCPU's
  registers (through the vCPU's thread) and the VM-wide registers.
- **`PartitionRequest`s**, from the rest of the worker, through a handle
  that `LoadedVm` keeps. Among them: stop the vCPUs for a while
  (`StopVps`), and let them run again (`StartVps`), without stopping any
  unit.

The second kind is used through a **`StopGuard`**: asking to stop the
vCPUs returns a guard; while the guard exists, the vCPUs stay stopped;
when it is dropped (Rust's word for destroyed), it sends `StartVps`, and
the vCPUs run again once no guard holds them. The snapshot boundary
(section 2.5) is such a guard.

### 2.5 Taking a snapshot

The microVM's snapshots are requested by the guest itself:

```
                      ┌┄┄┄┄┄┄┄┄┄┄┄┄ specified ┄┄┄┄┄┄┄┄┄┄┄┄┄┐
 guest                ┆   VM worker (LoadedVm)              ┆  controller (VmController)
   │                  ┆         │                           ┆       │
   │ writes to the    ┆         │                           ┆       │
   │ snapshot port ─────────>   │ stops the vCPUs at that   ┆       │
   │                  ┆         │ instruction: a StopGuard, ┆       │
   │                  ┆         │ the "snapshot boundary"   ┆       │
   │                  ┆         │ ──── "the guest asks" ──────────> │
   │                  ┆         │                           ┆       │ checks that a snapshot
   │                  ┆         │                           ┆       │ can be taken; flushes
   │                  ┆         │                           ┆       │ the console
   │                  ┆         │ <──── QuiesceForSnapshot ──────── │
   │                  ┆         │ stops every unit          ┆       │
   │                  ┆         │ (quiesce), saves every    ┆       │
   │                  ┆         │ unit, encodes the         ┆       │
   │                  ┆         │ SavedState                ┆       │
   │                  ┆         │ ──── reply: the saved state ────> │
   │                  ┆         │                           ┆       │ copies the scratch disk,
   │                  ┆         │                           ┆       │ writes the snapshot files
   │                  ┆         │                           ┆       │
   │                  ┆         │   on success: the source VM terminates
   │                  ┆         │                           ┆       │
   │                  ┆         │   on failure:             ┆       │
   │                  ┆         │ <── ResumeAfterFailedSnapshot ─── │
   │                  ┆         │ restarts every unit,      ┆       │
   │                  ┆         │ releases the boundary     ┆       │
                      └┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┘
```

Inside the dashed box: specified. For the messages that cross its right
edge (`QuiesceForSnapshot`, its reply, `ResumeAfterFailedSnapshot`), what
each may carry and what it promises is specified (the channel's
protocol), but not the controller's code that sends or uses them. The
"the guest asks" notification goes on another channel, which is not
specified. Neither are the guest's side and the controller's own steps.

"Quiesce" is a stop with a deadline: `StateUnits::quiesce_for_save` sends
every unit the same `Stop` request as a normal stop, but gives up after a
timeout and reports whether the failure left any unit in an uncertain
state.

### 2.6 Restoring

A restore starts a **new** VM, with a new VM worker:

```
                                      ┌┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄ specified ┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┐
 controller (VmController)            ┆   new VM worker                         ┆
     │                                ┆         │                               ┆
     │ reads the snapshot files       ┆         │                               ┆
     │ launches the worker with       ┆         │                               ┆
     │ the saved state ─────────────────────>   │ InitializedVm::load:          ┆
     │                                ┆         │ builds the VM (a LoadedVm),   ┆
     │                                ┆         │ every unit stopped            ┆
     │                                ┆         │                               ┆
     │                                ┆         │ LoadedVm::restore_from_       ┆
     │                                ┆         │ snapshot: each unit gets its  ┆
     │                                ┆         │ saved state; the guest's      ┆
     │                                ┆         │ clocks are adjusted           ┆
                                      └┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┘
     │                                          │
     │ VmRpc::Resume ─────────────────────────> │ starts every unit: the VM runs
     │                                          │ (not yet specified)
```

The controller reads the snapshot files and launches a new worker with
the saved state. The worker builds the VM with every unit stopped
(`InitializedVm::load`), gives each unit its saved state
(`LoadedVm::restore_from_snapshot`), and adjusts the guest's clocks
(which kept running since the snapshot). Then the controller starts the
VM with `VmRpc::Resume` (unless asked to leave it paused); `Resume` is
not specified yet.

## 3. What we prove

One contract per step of the transaction (section 1), in words:

| Step | Function | What its contract says |
|---|---|---|
| the boundary | `LoadedVm::establish_snapshot_boundary`, `release_snapshot_boundary` | while the boundary is active, the vCPUs are stopped; nothing else changes |
| stopping and saving | `LoadedVm::quiesce_for_snapshot` | on success: every unit is stopped, and the saved state in the reply equals the state of every unit once stopped. If refused: nothing changed. If it failed after stopping: every unit is still stopped (rollback is possible). If the failure left a unit in an uncertain state: nothing is promised (the VM must end) |
| handing over | the reply to `QuiesceForSnapshot` | until the controller asks for a rollback, the worker cannot restart, reset, or restore any unit |
| rollback | `LoadedVm::resume_after_failed_snapshot` | the VM runs again, and the boundary is released |
| save | `LoadedVm::save` (called by `quiesce_for_snapshot`) | with the VM stopped: returns the state of every unit, and changes nothing |
| restore | `InitializedVm::load`, `LoadedVm::restore_from_snapshot` | the VM is built with every unit stopped; afterwards each unit has the state it was saved with (a unit missing from the saved state keeps its own), and the saved list of units matches the VM's |

These rest on contracts at the layers below: `StateUnits`, each unit,
each chipset device, the partition and its vCPUs, and the channels
between them (section 4). **Status:** the contracts are written; their
bodies are not yet proved. Proving them is the next stage.

## 4. How the spec works

### 4.1 The challenge: concurrency

As we can see, the controller, the worker (`LoadedVm`), the units, and
the vCPUs all run concurrently. This concurrency makes it hard to specify
the correct behavior of the system. For example, when `LoadedVm` finishes
its `quiesce_for_snapshot` method, how can it know that those concurrent
units have indeed stopped? When `LoadedVm::save` returns a snapshot, how
do we know it is a correct snapshot, when the VM's state is spread across
many units, each owning its own part? And while a unit is saving a
device, how do we know a vCPU is not changing it?

### 4.2 The ideas: fractional ghosts and lock invariants

To answer these questions, the spec uses two tools: **fractional ghosts**
(4.2.1) and **lock invariants** (4.2.2).

#### 4.2.1 Fractional ghosts

A **fractional ghost** (`FracGhost`, from Verus's standard library,
`vstd::resource::frac`) is a ghost resource built on a partial commutative
monoid (PCM) of fractions; like every ghost resource, it can be moved
but never duplicated.

The high-level idea, like read and write permissions:

- A fractional ghost conceptually holds a splittable **permission** and 
  a **value** guarded by the permission. 
- The fractional ghost can be split into pieces (e.g. two halves), each owned 
  by a different thread or task. 
- All pieces of one ghost always **agree** on the value.
- The pieces' fractional permissions add up to 1. Holding the **whole** (1)
  permission allows you to **change** the value; holding a piece lets you 
  **read** the value.
- So if you hold a piece, nobody else holds the whole, and hence **nobody 
  can change the value** while you keep your piece.

The spec ties such a value to real state, e.g. "this ghost's value is the
device's current state". Then a piece is proof of what the state is, and
holding one is proof that nobody is changing it.

**Each unit's state.** Each unit has one fractional ghost, and the proof
annotations should make sure the value of this fractional ghost always matches
the unit's state as a snapshot (`Unknown`, or the abstract value of
its saved state). Its pieces move between `StateUnits` and the unit with
the `StateRequest` RPCs:

| Phase | Who holds the ghost | What it guarantees |
|---|---|---|
| running | the unit holds the whole | the unit may change its state, and keeps the ghost's value in step |
| `Stop` | the unit gives **half** to `StateUnits`, recording its state | with only half left, the unit cannot change its state; `StateUnits` now knows it |
| `Save` | `StateUnits` lends its half | the unit must return a saved state equal to what the half records |
| `Restore(saved state)` | `StateUnits` lends its half | afterwards the half records the saved state |
| `Start` | `StateUnits` gives the half back | the unit holds the whole again and may run |

The state that `save`'s result is compared with ("the state of every
unit once stopped", section 3) is read from the halves `StateUnits`
holds. That is how we can allow `save`'s contract to talk about the 
units' real state (assuming proof annotations have maintained the value 
in `StateUnits` to be the same as the unit's real state), not a value
the spec makes up. And in verified code, "a stopped unit does not change"
follows from the accounting: changing a unit's state needs the whole
ghost, which a stopped unit does not have.

**How a ghost travels with an RPC.** Verus has no model of mesh, so we
wrap mesh's calls (`mesh_rpc_verus`): a request and its reply each carry
a ghost value, and each message type has a **protocol**, a predicate
saying what a request must carry and what its reply promises. The
protocols on the snapshot path:

| Channel | Between | What travels |
|---|---|---|
| `StateRequest` | `StateUnits` → each unit | the unit's ghost halves (the table above) |
| `VpEvent` | the partition runner → each vCPU thread | the same scheme, one ghost per vCPU |
| `PartitionRequest` | `StopGuard` and `PartitionUnit` → the partition runner | **stop tokens**: each `StopGuard` holds one; while any is out, the vCPUs are stopped |
| `VmRpc` | the controller → the VM worker | during a snapshot, the units' halves are **lent to the controller**: while it holds them (copying the disk, writing the files), no unit can run; on rollback they come back |

#### 4.2.2 Lock invariants

A **lock invariant** is a property of the data a lock protects. Whoever
takes the lock may assume it, and must re-establish it before releasing
the lock.

The spec uses lock invariants for:

| Lock | Protects | Its invariant |
|---|---|---|
| a chipset device's lock (`CloseableMutex`) | the device's state, reached by its unit and by vCPU threads (section 2.3) | the device's state is well formed, and **its piece of the unit's fractional ghost** is kept here: the ghost's value is the device's state |
| `StateUnits`' registry (a mutex) | the list of registered units, shared by `StateUnits` and the code that registers units | each registered unit has an entry with its name and its ghost's id; names are distinct; the record of which units `StateUnits` has collected agrees with `StateUnits`' own |
| (planned) the same registry | each unit's state and dependency list | **the stop order**: if a unit is stopped, every unit that depends on it is stopped too (section 2.3); not yet in the spec, next to be specified |

The first row combines the two tools: since the device's piece of the
ghost lives in its lock, any code that changes the device under the lock
(a vCPU, the unit) must also update the ghost, which it can only do while
holding the whole ghost, that is, while the device is running. A vCPU
never reaches a stopped device because of the stop order (the third
row).

**The lock wrappers.** Verus has no model of OpenVMM's locks, so, as for
mesh, we wrap them with trusted specifications that attach an invariant
to each lock:

| Lock | Wrapper | The invariant comes from | Taking / releasing |
|---|---|---|---|
| `parking_lot::Mutex` (the registry) | `Mutex<V, Pred>`, `vmm_core/state_unit/src/mutex_t.rs` | a predicate type `Pred` (`MutexPredicate`) chosen when the mutex is created | `acquire` gives the value, with the invariant to assume; `release` requires it again |
| `CloseableMutex` (each chipset device) | `support/closeable_mutex/src/closeable_mutex_t.rs` | the protected value's type (`LockInvariant`), since the mutex is created by generic, unverified code | `lock_verified` / `close_verified` give the value with its invariant; `release` requires it again (and the same key: for a device, its ghost's id) |

How they relate to Verus's own lock, `vstd::rwlock::RwLock<V, Pred>`:

- **The same interface.** Like vstd's `RwLock` (`acquire_write` gives the
  value and a handle; `release_write(v)` requires the invariant of `v`),
  taking one of our locks gives the value with its invariant, and
  releasing it requires the invariant again. A reader who knows vstd's
  lock can read our contracts the same way.
- **A different implementation.** vstd's `RwLock` is written in Verus and
  verified, from a cell, atomics, and a token state machine. Using it
  would replace OpenVMM's locks with a different implementation, changing
  executable code and behavior, and vstd has nothing like
  `CloseableMutex`'s "close" operation. So we keep OpenVMM's real locks
  underneath and give them trusted specifications instead: the
  registry's wrapper is a thin type holding a `parking_lot::Mutex` (a
  change to OpenVMM's code, with the same behavior), and `CloseableMutex`
  is unchanged.
- **The invariant: per type today, per lock next.** In vstd, the
  invariant is a ghost value given when the lock is created, so each lock
  has its own (e.g. "the value's parity is this lock's parity"). Our
  wrappers currently take the invariant from a type instead, the same for
  every lock of that type, plus a **key** (a constant of the protected
  value that every holder keeps; for a chipset device, its ghost's id) to
  tie each lock to its own ghost. This is being changed to vstd's way
  (decided 2026-10-31): each lock's own invariant, given as a ghost value
  when it is created, naming that lock's ghost directly, with no key.

The wrappers' specifications are trusted (we do not verify the locks
themselves); the code that uses them is to be verified.
`SNAPSHOT-SPEC-MUTEX.md` explains the mutex wrapper in detail.

## 5. What is trusted, and what is not done

Every assumption is listed in `SNAPSHOT-SPEC-TRUSTED.md`, in three groups:

| Group | Meaning | Examples |
|---|---|---|
| **Trusted base** | outside our code; not to be proved | the hypervisor (KVM, MSHV, WHP) and thin wrappers over it; the guest's behavior; mesh message delivery; protobuf encoding; the locks' runtime; the tools (Verus, the Rust compiler) |
| **Deferred** | needs something Verus cannot express yet | contracts on methods of `async` trait objects and on futures: the vCPU threads' save and restore, the partition's handling of its other requests |
| **Assumed until proved** | our code; contracts stated, proofs to come | the bodies of every function with a contract; a few facts, e.g. "a unit receives `Stop` only while running", and the stop order (section 2.3) |

Out of scope for now: guest RAM, guest time, the controller's own code,
and the inner state of virtio devices (network, console, disks) beyond
what their transport saves.

## 6. The other documents

| Document | What it holds |
|---|---|
| `SNAPSHOT-SPEC-TRUSTED.md` | every assumption, in the three groups above |
| `SNAPSHOT-SPEC-CODE-CHANGES.md` | every change made to OpenVMM's code for the spec, numbered ("code change N") |
| `SNAPSHOT-SPEC-DESIGN.md` | the working log: every decision, its options and reasons, in the order they were made; the exact contracts; the open items (at its end) |
| `SNAPSHOT-SPEC-MESH-RPC.md` | the mesh RPC wrapper: how ghosts travel with messages |
| `SNAPSHOT-SPEC-MUTEX.md` | the mutex wrapper: locks with invariants |
