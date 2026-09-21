# Mutex acquisition observation experiment

This directory contains unapproved experimental external declarations, not
production TCB. Production sources, frozen inputs, and accepted proofs must
remain unchanged. The selected question is the actual
`StateUnits::inventory` lock and guard borrow, not its enumeration proof.

## Binding obligations recorded before experiments

- The observation must come from the guard returned by this acquisition of
  this mutex, not an independently chosen registry or a contents function
  of a shared mutex reference.
- An immutable guard borrow may expose protected data. Its Rust lifetime
  must prevent release or mutable guard access while that borrow is used.
  A saved ghost observation may outlive the guard, but must say nothing
  about later acquisitions or another mutex's contents.
- Mutation through `DerefMut`, temporary unlock/relock (`unlocked`, `bump`),
  and consuming guard operations must not inherit a read-only frame.
  Existing empty native contracts must not be mistaken for preservation
  guarantees.
- An opaque guard abstraction must retain Rust lifetime, trait-conflict,
  and aliasing checks. Type admission alone is not the observation fact.
- The real annotation overlay must lose the lock-specific errors with the
  candidate, even if independent datatype errors still prevent VCs.
  No replacement inventory implementation is permitted.

Source support: Cargo-selected parking_lot 0.12.4 aliases lock_api 0.4.13.
The latter's `src/mutex.rs` stores `UnsafeCell<T>` (138-141), acquires the
raw mutex and creates its guard (222-226), stores the source mutex reference
in the guard (504-507), dereferences that mutex's cell (651-656), permits
mutable dereference only through `&mut self` (659-663), and unlocks on Drop
(666-674). These support a guard-scoped observation, not a mutex contents
View. Dependency locations are resolved from current build metadata.

Caller evidence is the existing rust-analyzer report
`.verus_agent/cache/destination-inventory-callers.md`: save calls inventory;
restore calls validate_inventory, which calls inventory. The current
`dispatch.rs:3862,4757-4769` connects TOP to those operations. The accepted
ownership diagnostic `../restore-inventory-preservation/README.md` is
source evidence, not a proved registry frame.

Independent obligations remain: Arc<str> ToString refinement, native
HashMap/RandomState admission, StateRequest/SensitivityLevel admission,
ordered enumeration using existing BTreeMap values semantics, TOP registry
ownership, loaded representation, payload decoding, and TOP correctness.

Existing full-crate/check evidence is reused only for the unchanged live
sources. Isolated probes are not production proof completion.

## Result: diagnostic with a viable read-interface candidate

The actual library lock/dereference can expose an observation indexed by
its returned guard, with an origin equal to the lock receiver. There is
no mutex contents View, acquisition-freshness axiom, or frame across
release. `candidate.spec.rs` contains eight proposed external declarations
(eleven marker entries), detailed in `rationale.md`; none is installed.

Exact generic signatures are necessary: declaring only the parking_lot
aliases fails (`alias-admission.log`). The Mutex type has no RawMutex
bound, but its guard and lock do; Deref's implementation additionally
requires `R: 'a, T: 'a` (`generic-admission.log`). A minimal empty
RawMutex trait declaration fixes the enabled trait-conflict checker's
missing-trait failure (`observe.log`). The trait declaration admits no
raw operations. Safe external_fn_specification wrappers are necessary
here because assume_specification generates unsafe code forbidden by
state_unit (`production-assume-form.log`).

The source-supported interpretation of `guard_origin` is the guard's
stored mutex reference. `guard_observation` is the actual data reference
returned through that guard's immutable borrow. Its result lifetime is
the guard-borrow lifetime. Verus reference equality is not a newly
introduced address-identity theory. Different acquisitions need not have
different abstract values, but the interface cannot conclude they have
equal values or observations. This is enough to expose the protected
map to a later inventory proof, not to preserve that map through TOP.

### Final native controls

`*-checked.log` files record complete commands, diagnostics, exits, and
elapsed times. No lifetime/trait/erasure check was disabled. No rlimit
annotation was introduced; the native default is 10.

| Probe | Observed result |
| --- | --- |
| `observe.rs` | 2 verified, 0 errors: unchanged `*mutex.lock()` plus actual guard-origin/borrow connection |
| `controls.rs` | 3 verified, 6 expected failed functions; all seven assertion diagnostics checked at their exact source lines |
| `lifetime-escape.rs` | E0515: returning a reference to the local guard's borrow |
| `lifetime-release.rs` | E0597: using a borrow after scoped guard destruction |
| `lifetime-alias.rs` | E0502: mutable guard access while an immutable borrow is still used |
| `unconstrained-mutation.rs` | 1 verified, 0 errors; native empty DerefMut contract admits the write but supplies no later guard frame |

The positive controls establish repeated immutable reads of one guard,
origin distinction under an explicit unequal-mutex premise, and writing
then reading through one live mutable reference. Negative assertions
reject invented cross-mutex origins/contents, equality across repeated
acquisitions, equality after release and intervening mutation, stale
observations after mutation, and a post-mutation guard frame.

One early hypothesis was incorrect: missing a specialized DerefMut
contract does **not** make mutation unadmitted. Pinned
`vstd/std_specs/core.rs:39-44` already supplies an empty generic contract.
It havocs the guard model, including its origin and observation. This is
soundly weaker than the library behavior. A trial extra mutable contract
failed unsized-value equality typing and was removed; no new mutable
trust is proposed. Explicit `std::mem::drop` is unsupported by the pinned
interface; the release control instead uses ordinary scope exit.

### Source-matched production comparison

`production.py` materializes isolated copies of the current real
state_unit crate source, applies the supplied native annotation overlay,
and optionally includes the candidate declarations. It selects exact
externs and transitive `.vir` files using the active Cargo fingerprints.
It never builds, patches, or rewrites the live crate or dependencies.

The baseline reproduces the eight original errors
(`production-baseline.log`, exit 1, 0.624s). The candidate removes the four
Mutex/MutexGuard/RawMutex/lock errors (`production-candidate.log`, exit 1,
0.647s), leaving only HashMap, RandomState, StateRequest, and
SensitivityLevel admission failures. No inventory VCs are emitted.
That is a resolved lock-interface frontier, **not** inventory proof.

The standalone production harness uses the verifier's native
`--internal-test-mode` with Cargo-selected imports. This prevents the
driver from injecting its bundled builtin/vstd in addition to the
different Cargo-built instances. Ordinary standalone wiring first hit
duplicate trait metadata / builtin diagnostic items
(`production-duplicate-{vstd,builtin}.log`). The native mode changes
library injection, not lifetime or trait-conflict checking. Primitive
controls use the ordinary standalone mode.

Reproduce from the repository root (negative probe exits are checked by
the runner; production commands intentionally return 1):

```bash
"${ARGUS_SKILL_PYTHON:-python3}" research/mutex-observation-interface/probes.py
"${ARGUS_SKILL_PYTHON:-python3}" research/mutex-observation-interface/production.py baseline
"${ARGUS_SKILL_PYTHON:-python3}" research/mutex-observation-interface/production.py candidate
```

`prepare-request.py` regenerates independent patches from current frozen
and working branch inputs; it never applies them. The exact proposed
declarations and outstanding obligations are in `rationale.md`.
`submit-receipt.json` retains the authoritative durable submission receipt.
No approval or production proof follows from request validation.

The authoritative submit completed in 8.2s with `freeze_request: VALID`:
both patches apply independently, and their resulting TOP/BOTTOM specs
and executable behavior match. The persisted three-file request is
`research/freeze_requests/mutex-guard-acquisition-observation/`.
Submission stdout is
`.argus_subagents/mutex-observation-freeze_logs/stdout.log`.
It is **unapplied and awaiting review**, not Human-approved.

### Earlier submission-readiness blocker

The separately authorized delivery continuation has repaired ordinary Rust
erasure by gating only the ghost `restore_proof` module with
`verus_keep_ghost`. The completed `restore-proof-erasure-validation` run
passes package Clippy and documentation checks, all 69 unit tests, native
verification (1949 verified, 0 errors), and both drift checks. Its logs are
in `../restore-proof-erasure-repair/`. Boundary completion still reports
seven existing temporary trust locations; admission passes. No proposed
mutex trust has been installed. The final delivery commit and clean-tree
request validation are recorded in the shared mission checkpoint.

The generated intake backup described below is retained, without changing
its contents, at `target/mutex-observation-delivery/accepted-intake.tar`
relative to the repository root. The remaining text records the earlier
research-only delivery attempt and its blocker, not the current repair.

The requested preservation commit cannot currently pass its mandatory
precommit check. With the repository-supported Rust 1.95.0 toolchain,
`cargo clippy --locked --all-targets -p openvmm_core -p openvmm_defs
-p state_unit` exits 101 with ten E0432 unresolved imports. Complete output
is `delivery-clippy.log`; the durable task `mutex-delivery-precommit`
completed in 33.0s. `delivery-precommit.sh` records the exact command and
stops before doc, formatting, or any commit after that failure.

This is an ordinary Rust erasure problem in the already-accepted proof
work, not in the unapplied mutex candidate. `dispatch.rs:18-21` includes
`dispatch.proof.rs` unconditionally, but the latter's imports at 21-23,
25-30, and 33 refer to ghost-only items:
three erased specification functions, six imports from the
`#[cfg(verus_keep_ghost)]` saved-state proof module, and one from the
likewise-gated Duration observation module. Those imports remain outside
`verus!` when normal Rust compilation erases their targets. The passing
native Verus evidence does not exercise this ordinary-build condition.

No production repair, commit, stash, branch change, trust application, or
cleanliness workaround was performed. Existing accepted files and the
exact three request files were preserved byte for byte in the ignored
`target/accepted-intake.tar` snapshot. Committing only the package would
still leave the required decision tree dirty; discarding or hiding the
accepted work is not a valid substitute.

The next prerequisite is a separately authorized erasure-only repair of
the ten proof imports, followed by the remaining package checks, final
formatter, preservation commit, and readiness validation against its new
branch inputs. This readiness continuation does not authorize changing
live production proof sources. Request validation is not repeated on
unchanged branch inputs, and the blocked inventory proof remains untouched.

### Preserved live evidence and debt

No live proof, executable, manifest, or dependency source changed. The
matching latest full-crate/check evidence is reused, not repeated:

| Required check module | Current-source evidence |
| --- | --- |
| `make_verify --crate-root .` | Exit 0, 1949 verified, 0 errors; wrapper 73.910s, native runner 73.825s |
| `boundary --crate-root . --baseline-dir .verus_agent check` | Exit 3, INCOMPLETE; seven temporary locations, no permanent violations or assumptions; 1.069s |
| `spec_drift --crate-root . --baseline-dir .verus_agent` | Exit 0, frozen specs match; 1.234s |
| `exec_drift --crate-root . --baseline-dir .verus_agent` | Exit 0, no executable drift; 4.935s |

Logs remain `.verus_agent/cache/checks/<module>/latest.log`. These checks
cover the unchanged live selection, not the proposed TCB extension.
No temporary marker was added, removed, or moved in live production.
InitializedVm/LoadedVmInner representation, TOP's external body, and all
four recorded uninterpreted bridges remain debt. Out-of-scope datatype
opacity and unsanctioned compatibility declarations described in
`verification/restore/UNINTERP.json` also remain unchanged.

After an accepted boundary decision, the independent datatype/name
refinement prerequisites and actual ordered inventory proof still need
separate work. Do not resume that proof on this experimental interface.
