# OpenVMM snapshot spec: lessons from the proof agent's review

Mistakes the proof agent found in our spec (its readout, section 5,
reviewed with the user on 2026-11-03), grouped into a few issues. Each
issue gives the rule, then the items it comes from.

A spec that promises too much makes a proof fail, so the mistake shows up
by itself (5.1, 5.2, 5.3, 5.6, 5.9). An assumption that is too strong (a
`false` rule for a message from unverified code, a wrong trusted
contract) makes proofs pass that should not; only review and testing
find it (5.5, 5.10, 5.18). The second kind is the dangerous one.

## A. A high-level statement not carried down to every place it applies

When a contract or a design rule promises something, check that every
part it relies on says so too.

- 5.1 (also in F): `StateUnits::advance_time` promised that no unit's record changes,
  but the `AdvanceTime` protocol rule only said that the returned half is
  valid (`h.valid(k, 0.5real)`), not that its value is unchanged. For
  every request that returns a ghost, say what its value is: unchanged
  (`gout == gin`), or what it becomes.
- 5.6: the design leaves time out of the views. We left out the RTC's
  clock, but its view kept the CMOS bytes that hold the same time, which
  `restore` rewrites from the clock (`update_timers` →
  `set_alarm_timer` → `sync_clock_to_cmos`).
- 5.10: the same rule was never applied to the VP's saved state, which
  had the identity view (TSC, TSC deadline, the timer's count).

## C. Corner cases in our code that break a general claim

Each claim was true in the common case, but the code has a corner case
where it fails, found only by reading the code closely: a configuration
branch, a guest setting, a state left after a failure, an absent field.
Before stating a general fact about the code, look for its corner cases:
its branches (by configuration too), its callees, its callers, and
every value of its key state types.

- 5.2: who sends a request, before declaring that no one does.
- 5.3: `load` promised that a fresh VM has no recorded unit states. That
  holds for a microVM, but with PCIe host bridges the code starts and
  stops the units first (`assign_pci_resources`). A promise that holds
  only within our scope needs the scope as a precondition.
- 5.6: the call chain from the RTC's `restore` to `sync_clock_to_cmos`.
- 5.9: our view reduced the state units' `State` (9 values) to
  `running`. `QuiesceUncertain` stays after the operation that set it,
  and `run_op`'s `assert_eq!` panics on it. For each key state type, go
  through every value: either the view represents it, or `wf()` or a
  precondition rules it out.
- 5.12: a VP's saved state may leave an element out (an `Option`);
  `restore` then keeps the fresh VP's value, and the next save has the
  element present, with that value. Under the identity view the two
  differ, so the `Save` rule breaks after such a restore. The view reads
  an absent element as its initial value. Same root as 5.10: the VP's
  view was never written, and the identity view was wrong for it.

## D. Assumptions about parts outside the proof

What the spec assumes of a part outside the proof must be what it really
does or sends, and must be written as a weak but true statement. Three
kinds:

- **D1. Truly external, trusted for good** (KVM, the hypervisor, the
  clock). No proof ever checks the assumption; only reading the
  component's source and testing it do.
  - 5.10: we assumed KVM gives back exactly the VP state we restored. It
    sets or recomputes fields: the segments' accessed bit, `cr8` (tied to
    the TPR), the APIC version, `rflags` bit 1, the x2APIC LDR.
  - 5.18: KVM cuts `SYSENTER_EIP`/`ESP`, `SYSENTER_CS` and `TSC_AUX` to
    the host's widths on restore, so a VP's record after a restore is not
    exactly the buffer. Found by reading KVM's `x86.c` and by tests, not
    by a failed proof.
  - 5.13: the trusted state-access contracts (`open_vm_state` and the
    getters) stated a VP's state at any time, but KVM changes a vCPU by
    itself while VPs run (reading the MP state applies a pending INIT
    or SIPI; other vCPUs' IPIs arrive). The facts now hold only while
    the partition is quiet (all VPs stopped), which the snapshot paths
    meet. Also: a CR8 above 15 changes nothing, a segment's attribute
    bits 8-11 and the pending NMI leave the VP view, and a device NMI
    or SMI to a stopped vCPU changes only its pending NMI or SMI.
  - 5.22: we assumed a KVM read changes nothing ("a getter returns the
    element's current value and changes nothing", `virt/src/state.rs`).
    On a host whose KVM supports MPX, reading a VP's MP state
    (`KVM_GET_MP_STATE`) loads and saves back the guest's FPU state,
    which can change bits of the XSAVE header; two saves of the same,
    unchanged, stopped VP can then differ byte for byte. Most likely only
    the encoding changes (the header marks which FPU parts are in their
    initial state), not the guest's FPU state: not yet checked.
  - 5.19: a trusted contract tied a ghost to a real external object
    without checking which ghost it was (see F, which has the rule).
    `open_vm_state` accepted any whole `FracGhost<VmSavedState>` and
    promised that the partition's real state equals its value.
- **D2. Temporarily external**: our own code not yet specified or
  proved, but planned. The assumption is unchecked only until that code
  is proved; then the sending side's proof fails if it is wrong. Until
  then, our guarantees rest on it.
  - 5.5: 17 `VmRpc` requests had the rule `false`, but the controller,
    not verified yet, can send them; the worker's guarantees silently
    assumed it never does. The rule must hold for anything such a
    sender may send: no ghost either way, any reply.
  - 5.14. Lesson: for every input that comes from unverified code,
    decide which values are valid and write it into the contract:
    either a precondition that a verified check establishes, or an
    `Err` case for the invalid values. An out-of-scope part is no
    exception: an input that reaches it must not make it panic.
    What was wrong: `load` had no precondition, so its contract covered
    every configuration, including snapshot RAM ranges that are not
    page-aligned or end before they start; on those the code panics
    (`MemoryRange::new`; their BUG-23). We never asked what `load`
    should do with a bad configuration. Fix: `validate_machine_config`
    rejects such ranges, and `load`'s precondition (the microVM
    configuration, 5.3) includes well-formed ranges.
  - 5.3, 5.14, 5.16 and 5.17 together: a type with some methods specified and some
    not, and no view, is particularly tricky. `InitializedVm::load` is
    in scope and has a contract, but `InitializedVm` is declared to
    Verus by name only (fields hidden, `dispatch.proof.rs`), and
    `InitializedVm::new`, which builds it from the configuration, has no
    contract. So the type had no `wf()` to require, and `load` had no
    precondition: it promised its results for every possible
    `InitializedVm`, assuming nothing about the code that built it. The
    PCIe host bridges (5.3) and the malformed RAM ranges (5.14) are two
    inputs where that broke; 5.16 is a third fact `load` relies on (the
    RTC's offset from UTC is 0, which the RTC's clock constructor needs),
    checked by `validate_machine_config` but not stated. When a method of a type is specified, the
    type needs a spec too, at least a `wf()` with what the method relies
    on, stating the assumption about the unspecified code that builds
    it (here: built from a validated microVM configuration).
    5.17 closes the gap that way: `InitializedVm::new` gets a contract
    (`created_from(cfg)`: the manifest, the VP count, the time keeper
    stopped, no IOMMU, ...), and `load` requires it.
- **D3. Out of scope, hidden instead of stated.** To leave something
  out of scope, state only what the rest of the spec relies on, and
  nothing false: never a protocol rule of `false`, never an
  `assume_new()` ghost at a call site. Both are assumptions about a part
  outside the proof, written as "this never happens". Between two
  verified parts the sender's proof catches them; when the other side
  is outside the proof, nothing does.
  - 5.2: the partition unit's protocol gave `CheckTimers`, `AdvanceLapic`,
    `OmitSavedTsc` and `SetInitialRegs` the rule `false` ("verified code
    cannot send them"), but our own entrance functions send them; the
    send sites used `assume_new()` marked "OUT OF SCOPE". A made-up ghost
    does not make a forbidden send provable (`call` still requires the
    rule); the bodies were simply not verified yet. The weak but true rule:
    no ghost either way, and the records stay the same.
  - 5.8: a weak but true rule still says what the request changes in our
    records: `ClearHalt` clears the partition's `halt_reason`, so it
    carries `StateUnits`' half and returns it with the halt cleared.
  - A rule is also what the receiver learns about each message it receives
    (`Receiver::recv`), so `false` lets the receiver prove anything once
    the message arrives. Between two verified parts the sender's proof
    catches it (5.2); when the sender is not verified, nothing does (5.5,
    D2).

## E. Error paths under-specified

Our spec said little about error paths in general (OPEN-ITEMS 1.2). An
error path needs two things in the contract:

1. When the function may fail: the reasons the design allows, taken
   from its intent (CONVENTIONS.md, "Errors"). Without them, a function
   that always fails meets its contract. Missing almost everywhere at
   first; added for `save`, `restore` and the guest-requested snapshot
   (commits `cb57ac701`, `219fe89c4`), the rest still open.
2. What state the failure leaves, so that a caller can go on after it.

- 5.11: `resume`'s `Err` clause said only that the units are stopped. A
  caller that sends `Resume` again needs more: `resume` requires that a
  stopped VM's halves are all held, which the failure did not promise.
  The fix says what is left: the worker holds every half again, or a
  unit is stuck (which ends the worker). Likewise
  `resume_after_failed_snapshot`'s failure (the units run, or every half
  is held, or a unit is stuck), and `Uncertain` leaves the units marked
  running. Part of the error-case work (OPEN-ITEMS 1.2).

## F. A FracGhost not pinned down: its id and its value

Wherever a contract or a protocol rule takes or returns a FracGhost
(`gin`, `gout`, a trusted token), say which ghost it is and what it
holds; never accept "any FracGhost of the right type".

- Its id: require that the ghost's id is its object's id (e.g. `t.id()
  == partition.state_token_id()`, or `h.valid(k, ...)` with the unit's
  `k`), and have the code that creates the ghost promise that id.
  Otherwise a ghost that verified code makes with `FracGhost::new` and
  any value is accepted.
- Its value: say what the returned ghost holds, unchanged (`gout ==
  gin`) or what it becomes, not only that it is a valid half.

This is the classic way to use a FracGhost; we forgot it more than once.

- 5.1 (the value): the `AdvanceTime` protocol rule returned
  `gout matches Some(h) && h.valid(k, 0.5real)`: the right unit's half,
  but with any value, while `StateUnits::advance_time` promised that no
  record changes (also in A).
- 5.19 (the id): `open_vm_state` (trusted) required only "a whole
  `FracGhost<VmSavedState>`" and promised that the partition's real state
  equals the ghost's value. Verified code could make a ghost holding X,
  open and close a handle with it, make one holding Y, and open again:
  the contracts then say the same, unchanged partition is in state X and
  in state Y, which proves `false`. In a trusted contract Verus does not
  check the promise, so nothing reports it. Fix: `new_vm_state_token`
  ensures `t.id() == partition.state_token_id()`, and `open_vm_state`
  requires it. (A fresh `FracGhost::new` always has a new id, so it
  cannot match.) That `new_vm_state_token` is called once per partition
  stays a PROOF TODO: two whole ghosts with one id cannot exist, so a
  second call's promise would prove `false`.

## Every item of section 5

The problem behind each numbered item of the proof agent's readout,
section 5, in the readout's order, with its group above. "Not a mistake"
items say what the change was and why.

- 5.1 (A, F): `StateUnits::advance_time` promised that no unit's record
  changes; the `AdvanceTime` protocol rule let the unit return its half
  with any value.
- 5.2 (D3, C): four partition-unit requests (`CheckTimers`,
  `AdvanceLapic`, `OmitSavedTsc`, `SetInitialRegs`) had the rule `false`
  ("verified code cannot send them"), but our own entrance functions send
  them; the send sites used `assume_new()` marked "OUT OF SCOPE".
- 5.3 (C, D2): `InitializedVm::load` promised that a fresh VM has no
  recorded unit states, false with PCIe host bridges
  (`assign_pci_resources` starts and stops the units); `load` had no
  precondition, as `InitializedVm` has no spec.
- 5.4 (not a mistake): helper contracts the proof needs (`StateUnits::
  start`, `run_op`, `UnitBuilder::spawn`, the partition unit's handles,
  the `PROOF TODO` ghosts). Left to the proof agent on purpose.
- 5.5 (D2, D3): 17 `VmRpc` requests had the rule `false`, though the
  controller, not yet verified, can send them; the worker would receive
  `false` as a fact.
- 5.6 (A, C): the RTC's view kept the CMOS bytes that hold the date and
  time, which `restore` rewrites from the clock when the guest's alarm is
  enabled; time was meant to be left out of every view.
- 5.7 (not a mistake): `StateUnits::reset` now promises every unit's
  record is `Unknown` afterwards. Our contract was true but too weak for
  5.2's fix (`SetInitialRegs` only while the record is `Unknown`, sent by
  `reset`'s firmware reload); we had left `Reset` for after the proof.
- 5.8 (D3): `ClearHalt` had the rule `false`; it clears the partition's
  `halt_reason`, which is in the record, so its true rule carries the half
  and returns it with the halt cleared.
- 5.9 (C): our view reduced the state units' `State` (9 values) to
  `running`; a unit left `QuiesceUncertain` after a failed quiesce makes
  `run_op`'s `assert_eq!` panic, so `pause` and others now require that no
  unit is stuck.
- 5.10 (A, D1): the VP's saved state had the identity view: it kept time
  (TSC, TSC deadline, the timer's count) and fields KVM sets or
  recomputes (the accessed bit, `cr8`, the APIC version, `rflags` bit 1,
  the x2APIC LDR) or that devices change after the VP stops (IRR, TMR).
- 5.18 (D1): a VP's restore recorded exactly the buffer, but KVM cuts
  `SYSENTER_EIP`/`ESP`, `SYSENTER_CS` and `TSC_AUX` to the host's widths.
- 5.12 (C): under the identity view, an element absent from a VP's
  saved state differs from the same element present with its initial
  value, which is what `restore` leaves and the next save returns.
- 5.11 (E): failure clauses did not say what state is left: after a
  failed `resume`, a second `Resume` needs every half held, which the
  failure did not promise; likewise `resume_after_failed_snapshot`, and
  `Uncertain` leaving the units marked running.
- 5.13: four changes to trusted parts.
  (1, D1) the trusted state-access facts held at any time, but KVM
  changes a running vCPU by itself; they now hold only while all VPs are
  stopped. (2, not a mistake) our lock wrappers: acquire and release open
  no invariant, and a lock mode that is never closed; strengthenings the
  proof needed. (3, D1) more KVM facts corrected: a CR8 above 15 changes
  nothing, segment attribute bits 8-11 and the pending NMI leave the VP
  view, a device NMI or SMI to a stopped vCPU changes only its pending
  NMI or SMI. (4, not a mistake) new trusted modeling axioms for Verus
  limits (the slab's occupancy, a lock's content identity, a device lock
  unsized to a trait object).
- 5.14 (D2): the controller's configuration gives the snapshot RAM
  ranges, decoded unchecked; `load` panicked on a range not page-aligned
  or ending before it starts (their BUG-23), as `load` had no
  precondition. A code fix (`validate_machine_config` rejects them).
- 5.15 (not a mistake): contracts for the helpers `StateUnits::start` /
  `start_with_rollback` (a failed start stops the units it started). Our
  top-level `resume` already said it; the helpers are the proof agent's.
- 5.16 (D2): `load` relies on the RTC's offset from UTC being 0 (the
  RTC's clock constructor needs it); `validate_machine_config` checks it,
  but nothing stated it for `load`.
- 5.17 (D2): the fix of 5.3, 5.14 and 5.16: `InitializedVm::new` gets a
  contract (`created_from(cfg)`), and `load` requires it.
- 5.19 (D1, F): `open_vm_state` (trusted) accepted any whole
  `FracGhost<VmSavedState>`, not the partition's own (no id check), and
  promised the partition's real state equals its value, so verified code
  could prove `false`. We disagree with the fix (see below).
- 5.20 (not a mistake of ours; we disagree with the fix): they require the KVM hypervisor
  for the microVM worker, to prove that a restore's VP count is ignored
  off MSHV: Verus cannot read MSHV's id (an associated constant of an
  external trait). But the microVM runs on KVM, MSHV and WHP (the microVM
  developer, 2026-11-03), `validate_machine_config` accepts all three,
  and our spec has MSHV in scope (`virt/src/state.rs`; our C3 covers
  MSHV's restore with fewer VPs). The narrowing comes from a Verus limit,
  not from the intent (see below).
- 5.21 (not a mistake of ours; a limit to record): the result holds only on
  a VMX (Intel) host, since the trusted KVM facts they wrote (we had
  deferred them) were checked only there. The microVM also runs on AMD
  hosts (the microVM developer, 2026-11-03), so this is a gap in the
  result, not "the intent's configuration" (see below).
- 5.22 (D1, pending): we assumed a KVM read changes nothing; on a host
  whose KVM supports MPX, `KVM_GET_MP_STATE` saves the guest's FPU state
  back and can change the XSAVE header, so two saves of an unchanged VP
  can differ byte for byte. Their fix limits the result to hosts without
  MPX. Most likely the code works there and only our byte-for-byte
  demand is too strict; the alternative is to treat the two encodings as
  equal in the VP's view (as in 5.10). Waiting for the microVM developer
  (MPX hosts) and a check of the XSAVE header's meaning.

## Questions for the microVM developer

Asked or to ask; the answers decide the pending items.

1. Does the microVM run on MSHV and WHP, or only on KVM? (5.20)
   Answer (2026-11-03): it runs on KVM, MSHV and WHP.
2. Does it run on AMD hosts, or only on Intel? (5.21) Answer
   (2026-11-03): it runs on AMD.
3. Do older Intel hosts whose KVM supports MPX matter? (5.22) Not yet
   answered.

## To send to the proof agent

Items of its readout's section 5 where we disagree with its fix.

- 5.19 (`open_vm_state` and the state token): we agree with the defect,
  not with the fix. The fix wraps the FracGhost in a new tracked type,
  `VmStateToken`, with private fields and a ghost `lent` flag, so that
  only `new_vm_state_token` can make one. It may be equivalent, but it is
  the wrong way to use a FracGhost: the classic fix is to tie the ghost
  to its object by id. Please use instead: `new_vm_state_token(partition)`
  ensures `t.id() == partition.state_token_id()` (a new uninterpreted spec
  function on the partition), and `open_vm_state` requires
  `t.id() == partition.state_token_id()`; keep `FracGhost<VmSavedState>`
  as the token's type, and drop `lent` (verified code gets a handle only
  through `open_vm_state`, which takes the token away until
  `close_vm_state`).

- 5.20 (the microVM worker requires KVM): please drop the `"kvm"`
  precondition. The microVM runs on KVM, MSHV and WHP (its developer,
  2026-11-03), and our spec has MSHV in scope. State the restore's VPs
  per hypervisor instead, with the trusted `"mshv"` constant you already
  have: on MSHV, the named count of VPs is restored; on any other
  hypervisor, all of them.
- 5.21 (the result holds on a VMX host): the microVM also runs on AMD
  hosts (its developer, 2026-11-03), so please record the VMX-only result
  as an open gap, not as the intended configuration. Closing it needs
  KVM's SVM facts (`svm.c`), cited and tested on an AMD host.

## How to reduce these mistakes

Three kinds of measures: mechanical checks, a checklist for each
contract, and earlier feedback from proofs and tests.

1. **Mechanical checks** (to add to `scripts/verus-spec-audit.py`):
   - D3: fail on a protocol rule `=> false`, and on an `assume_new()`
     marked OUT OF SCOPE.
   - F: fail on a `requires` or `ensures` that takes a FracGhost without
     an id clause (`id() ==`, or `valid(k, ...)`); flag a returned ghost
     whose value is never stated.
   - E: flag an entrance contract without a `result is Err ==>` clause.
   - D2: flag a type with a specified method but no `View` or `wf()`.
   These would have caught 5.1, 5.2, 5.5, 5.8, 5.11, 5.14 and 5.19.
2. **A checklist for each entrance contract**, before it is done:
   - A: for each promise, name the rules or contracts it rests on, and
     check that each says enough. For a design rule ("time is out of the
     views"), list every place it applies.
   - C: list the code's corner cases (configuration branches, every value
     of key state types, callees that change state, absent or optional
     fields), and check the promise against each.
   - D1: every trusted fact about an external component (KVM) cites its
     source (file:line) and has a test against the real component.
   - E: say when the function may fail, and what state each failure
     leaves.
3. **Earlier feedback.** A spec that promises too much makes a proof
   fail, so have the proof agent try each entrance contract soon after it
   is written, not at the end; most of 5.1, 5.2, 5.3, 5.6 and 5.9 would
   have come back quickly. The dangerous kinds (D1, D2) do not show up in
   proofs: they need the citations and tests of item 2.
