# Comparing two snapshot specs: ours and the natural-language-prompted one

Compared (2026-11-01):

- **Ours**: `tangruize/openvmm`, branch
  `system-proof/snapshot-spec-review-20261023`, at `4208c4e7d`
  (2026-10-31), as written: contracts with unproved (`external_body`)
  bodies. The proof run on it is not on GitHub, so its final trust base is
  not compared here.
- **Theirs**: `noah-verification/openvmm`, branch `verification/2026-10-31`,
  at `e5c25a006` (2026-10-31, the branch's latest), folder `system-proof/`
  (spec written by a model from the prompt `INPUT.md`, then proved in the
  same run). It already includes the proof, so it has many more
  annotations, and specs of helper functions; those are not counted as
  differences.

The OpenVMM code each starts from is in section 2 (ours: `7f225e2`;
theirs: `bcfc47cff`).

Only the **entrance contracts**, the **meaning** they give to "the
state", **how they establish agreement across threads and RPCs**, and
**what they trust** are compared.

## The gist

**Theirs specifies the code sequentially.** It specifies the `LoadedVm`
side (`StateUnits`, `quiesce_for_snapshot`, `load`) and the device side
(each device's own `save` and `restore`) separately, and assumes what
connects them. It uses no fractional ghosts and no lock invariants (none
on any of its branches). As a result, its spec does not tell us:

1. that what `LoadedVm` saved is indeed what each device returned (and,
   at restore, that each device gets the state it saved);
2. that a device indeed stops, and stays unchanged until it is saved;
3. that the vCPUs and the devices' units use the device locks correctly
   (e.g. their BUG-16: a vCPU locking a device whose lock the snapshot
   closed).

Ours specifies all three, through the units' fractional ghosts and the
device locks' invariants.

**What ours misses: the error cases.** Our contracts say what holds on
Ok and after each kind of error, but not when an error may happen, so a
function that always fails satisfies them; `load` has no Err clause at
all (`OPEN-ITEMS.md`, item 1.2).

## 1. Summary

| | Ours | Theirs |
|---|---|---|
| Uses fractional ghosts | yes: one per unit, its pieces moving with the RPCs and kept in the device locks | **no**: no `FracGhost`, no `GhostVar`, no invariants, although the prompt suggested them |
| How "saved = real state, unchanged after stop" is established | from the ghosts' accounting (a stopped unit has only half, so cannot change) | **assumed**: an abstract world number, kept fixed by `StateUnits`; that VP threads and devices change nothing is a set of premises |
| Device states in the save and load contracts | yes: each unit's view, defined from its fields | **no**: undefined (uninterpreted) functions; only each device's own `save` and `restore` mention its registers |
| Trusted beyond external code | none (by rule: our code is to be proved) | about 25 premises, most about OpenVMM's own code; about 3,370 lines of in-scope code trusted without a Verus limit forcing it |
| Scope | the worker's side: quiesce, save, restore, start, rollback, the boundary, and the channels' protocols | five functions across both processes, including the controller's publication of the files, the restore's checks, guest RAM and guest time |

In short: **ours states what the devices' states are in the save and
load contracts, and the concurrency facts; theirs assumes both, but
covers more of the round trip** (the controller, the files, RAM, time,
error reasons), at the cost of trusting much more.

## 2. Differences before the spec

They were not given the same task, nor the same code, so some differences
come from the inputs, not from the spec writer.

| | Ours | Theirs |
|---|---|---|
| Code | OpenVMM's microVM branch at `7f225e2` (2026-10-13) | `microsoft/openvmm` `feature/microvm/dev` at `bcfc47cff` (2026-10-29), 16 days newer: about 7,100 more lines of Rust, mostly outside the snapshot path (device-tree loader, CPU profiles, other hypervisor backends, CLI). On the snapshot path the code is nearly the same (`state_unit` identical; `microvm.rs` +55/−12), except the time ABI (`time_abi.rs` +730/−79), which only theirs specifies |
| Entry functions asked for | the worker's snapshot transaction (decided with the user) | five, named in `INPUT.md`: `LoadedVm::quiesce_for_snapshot`, `VmController::handle_guest_snapshot_request`, `prepare_snapshot_restore_for_config`, `InitializedVm::load`, `LoadedVm::start_state_units` |
| Out of scope | the controller's code, guest RAM, guest time, virtio devices' inner state | virtio devices, sandbox blocks, other hosts and hypervisors, cold boot, tampered files |
| Configuration | microVM, Linux host, x86-64 guest; any hypervisor | the same, on KVM only |

## 3. The entrance contracts, side by side

| Step | Ours | Theirs |
|---|---|---|
| Quiesce and save | saved state = each unit's state as recorded at its stop; nothing changed it after | saved state = "whatever each unit replies"; that this is the real, unchanged state is **assumed**. Also requires the time records, CPU profile and command line in the reply, and gives a reason for every error |
| The worker after replying | cannot touch the units | **assumed** (W-1) |
| The boundary | vCPUs stay stopped while a stop token is out | **assumed** (PC-2; a proof was tried and found too costly) |
| Restore (`load`) | the units hold the saved states; **no Err clause** | the unit names match the saved inventory; each device's state is `restored_unit_obs(name, saved)`, an undefined function, so which state is not said; time and portb's restore identity specified; errors only for named reasons |

Theirs also covers steps outside our scope, because their prompt named
them: starting the restored VM (`start_state_units`), the controller's
publication of the snapshot files, and the restore's compatibility checks.
No function both saves and restores (they run in different processes), so
the round trip follows from save's and load's contracts; theirs states it
as a separate proof function.

## 4. The main technical difference: how agreement is established

**The question both specs must answer:** the saved state is collected
from many units, each in its own task, while vCPU threads can reach the
devices. Why does it describe one consistent, unchanging state of the VM?

**Ours, with fractional ghosts.** Each unit has a fractional ghost whose
value is meant to be the unit's state. At `Stop` the unit gives half to
`StateUnits`; with only half left it cannot change its state, and
`StateUnits` knows it. A device's piece lives in its lock, so any code
that changes the device under the lock, including a vCPU's access, must
update the ghost, which needs the whole. Closing a device's lock needs its
close token whole, locking needs a piece, so "locking a closed lock
panics" becomes a precondition.

**Theirs, with an abstract world.** `StateUnits` carries a ghost number,
`world: Ghost<int>`. A unit's reply to `Save` in world `w` is an
uninterpreted function, `unit_save_reply(w, name)`. Starting the units
leaves the world unconstrained ("the world may change from now on"),
`StateUnits`' operations keep it fixed, and restoring changes it to
`world_after_restore(w, saved)`. So "save returns the units' state" holds
**by definition**, and "nothing changes between stop and save" holds as
far as `StateUnits` is concerned. That nothing **else** changes the
devices, and that the world means the devices' real state, are premises:

| Premise | What it assumes | Our spec's counterpart |
|---|---|---|
| W-1 | no management mutation (inspect writes, debugger) during the capture | the lent halves (no unit operation without them) |
| U-1 | the captured units are mutually consistent; "nothing else changes a device" | a stopped unit holds only half its ghost |
| DISPATCH-1 | a vCPU's device access during the boundary finds the device's lock open | the close token: locking needs a piece, so this is a proof obligation |
| PC-2 | every `StartVps` follows one successful stop | stop tokens in `PartitionRequest`'s protocol |
| PA-1..PA-4 | no unit added or removed during an operation; units answer; refusals and timeouts mean what they say | the registry's lock invariant; the request protocol |
| M-1 | seven "model links" (trusted proof functions) from the abstract world to each device's proved round trip | not needed: each unit's ghost is tied to its own state |

A concrete case: their run found **FW-2 / BUG-16**, a VP's device access
during the snapshot's input gate panicking ("lock should not be closed")
with two or more VPs, by end-to-end testing, and then made "it does not
happen" premise DISPATCH-1. In our spec the same fact is a precondition
(locking needs a piece of the close token), so a proof of our contracts
cannot go through while the bug exists. Worth checking whether it is among
the 33 bugs found with our spec.

## 5. Where the devices' states appear

Checked in their code (not only their documents), 2026-11-01.

**Theirs specifies two levels, and assumes the link between them.**

| Level | Save | Restore |
|---|---|---|
| `LoadedVm` and `StateUnits` (`quiesce_for_snapshot`, `load`) | each saved blob is `unit_save_reply(w, name)`, "whatever the unit answered": in order, under its name, the inventory complete | the unit names match the saved inventory; each device's state is `restored_unit_obs(name, saved)` |
| Each device's own code (e.g. `PitDevice::save`, `restore`) | the saved state describes the device's registers | the registers become what the saved state gives |
| The link between the two levels | assumed (M-1, `interface_unit_round_trip`) | assumed (M-1, `interface_world_after_restore`) |

`unit_save_reply`, `restored_unit_obs` and `guest_obs` are uninterpreted
(declared without a body); the only facts about their values are
`external_body` proof functions. So neither `quiesce_for_snapshot` nor
`load` says what a device's state is.

**Their device level, unit by unit:**

| Unit | Save and restore of the device's own code |
|---|---|
| PIT, PIC, IOAPIC, RTC, portb, vmtime | specified, from the device's fields |
| shutdown, snapshot-request | nothing to save |
| partition (the vCPUs) | through uninterpreted functions: "what the hypervisor saves / restored" (`vp_saves`, `partition_saves`, ...) |
| virtio-MMIO | not specified (out of their scope) |

Their device-level views are written as what the guest can observe, and
they list the guest-visible state a device's save does not carry (e.g.
PART-1, `IA32_SPEC_CTRL` not saved, so Spectre mitigations are silently
off after a restore). A unit's state in ours is what the unit saves, so
state never saved is invisible to our spec (our overview, section 1:
"equality of the state as the spec models it").

**Ours, for the two units theirs leaves weaker:**

| Unit | Ours |
|---|---|
| partition (the vCPUs) | the hypervisor's blobs (the partition's, and each vCPU's), each vCPU's tied to its own fractional ghost (`VpSet`'s halves) |
| virtio-MMIO | the transport's fields (status, features, queues, interrupt status); the inner device's state as a blob (its content out of scope) |

Theirs also covers what ours leaves out: guest RAM (through a trusted
kernel axiom, `axiom_shared_ram_is_file`), guest time (the TSC, LAPIC
timers, VM time and the RTC advance by the measured downtime, at most 30
days), and why each error happens.

## 6. The trust base

| | Ours (spec) | Theirs (finished proof) |
|---|---|---|
| External, trusted | the tools; protobuf blob encoding; mesh RPC delivery (one generic wrapper, with a protocol per channel); the locks' runtime; the hypervisor and the guest | the kernel (file-backed RAM), the file system (22 models), KVM and the CPU, protobuf round trips; mesh delivery as **per-request reply axioms** (about a dozen, each naming one request's handler) |
| Assumptions about OpenVMM's own code | none: every such fact is to be proved (`OPEN-ITEMS.md`, section 2) | about 25 premises (W-1, J-9, PA-1..4, PC-1..4, U-1, PU-1, PU-2N, MR-1, SR-1, DISPATCH-1, M-1, PU-HALT, VP-CAP, and others) |
| Our code trusted although Verus could check it | none | about 3,370 lines, e.g. the VM's construction `construct_parts` (1,697 lines), the restore's shape check (548), the CLI's restore inputs (195) |
| Trusted because of Verus limits | 8 functions, marked `PROOF DEFERRED` | 457 lines (43 items), each with a reproducer |
| Uninterpreted functions | 23, for external things, plus `VmTimeKeeper`'s view (by decision) | 179, including the meaning of the world (`unit_save_reply`, `world_after_restore`, `units_consistent`, ...) |
| `assume_specification` | 5 | 299 (mostly library contracts the proof needed) |

The difference in kind: theirs trusts facts about OpenVMM's own code
and about the meaning of its model; ours does not.

## 7. Where each is stronger

**Ours:**

- Proves, rather than assumes, that stopped units and devices do not
  change, including against vCPU threads (theirs: U-1, W-1, DISPATCH-1).
- The vCPUs stay stopped while any stop token is out (theirs: PC-2,
  abandoned as too costly).
- The worker cannot run on its own after replying with a snapshot (theirs:
  W-1).
- One generic, trusted mesh wrapper with a protocol per channel (all 22
  `VmRpc` requests listed), instead of per-request reply axioms.
- The save and load contracts say what each unit's state is (its view);
  theirs use undefined functions there.
- virtio-MMIO's transport state is specified.
- Almost no uninterpreted function stands for our own code's behavior
  (only `VmTimeKeeper`'s view).

**Theirs:**

- Each device's own save and restore are about what the guest observes,
  so state a device does not save is found (PART-1 and others).
- Covers the controller, the snapshot files, the restore's compatibility
  checks, guest RAM and guest time.
- Every error has a stated reason; `load` has an Err clause
  (ours has none).
- `start_state_units`' contract (ours has only `resume`'s).

## 8. What we could take from theirs

1. A guest-observable meaning for each device's state, at least as a
   check of each device's view (it finds unsaved registers).
2. When each entrance function may fail: today an implementation that
   always returns an error satisfies our contracts, and `load` has no
   Err clause at all (now open item 1.2).
3. The controller's outcome contract (already open item 5.1).
4. A contract for `start_state_units`.

To compare the two runs' **bugs**, I need the list of the 33 found with
our spec; theirs are in `system-proof/BUGS.md` (BUG-01..16, plus
differences and known defects).
