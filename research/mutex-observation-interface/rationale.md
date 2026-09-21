# Proposed guard-scoped parking_lot observation boundary

## Decision requested

Admit only the external declarations in
`vmm_core/state_unit/src/mutex_observation/observation.spec.rs`, plus their
scope/TCB manifest entries and ghost-only module/dependency wiring.
Neither patch changes TOP, an executable expression, or existing trust.
The proposal is not approval and must not be installed by Engineer.

`freeze.patch` applies independently to the current frozen branch;
`run.patch` applies independently to the current working branch commit.
The former additionally supplies the state_unit Verus metadata and vstd
dependency already present on the working branch. No dirty accepted proof
is overwritten or incorporated into this request.

## Exact proposed declarations and meanings

| Declaration | Marker(s) | Meaning |
| --- | --- | --- |
| `ExRawMutexTrait` | `external_trait_specification` | Admit the existing unsafe `lock_api::RawMutex` trait as a bound. The proxy declares no raw operations, associated types, invariants, or behavioral axioms. |
| `ExRawMutex` | `external_type_specification`, `external_body` | Opaque representation of the selected `parking_lot::RawMutex`. |
| `ExMutex<R, T>` | `external_type_specification`, `external_body` | Opaque existing `lock_api::Mutex<R, T>`, including unsized T. No contents View. |
| `ExMutexGuard<'a, R, T>` | `external_type_specification`, `external_body` | Opaque actual guard with its original lifetime and trait bounds; not a replacement guard. |
| `guard_origin` | `uninterp` | The mutex reference stored in this guard. |
| `guard_observation` | `uninterp` | The protected data reference obtained by immutably borrowing this live guard; result lifetime is the borrow lifetime, not the mutex lifetime. |
| `mutex_lock` | `external_fn_specification` | The actual generic lock operation returns a guard whose origin is its receiver. |
| `guard_deref` | `external_fn_specification`, `when_used_as_spec` mapping | The actual generic Deref implementation returns this guard borrow's observation. |

These are eleven new manifest entries over eight named declarations.
The exact Rust bounds, lifetimes, and formulas are in the patches, not an
implicit permission for a whole path. No extra `assume`, axiom, iterator,
mutable-dereference contract, project datatype opacity, or registry frame
is proposed.

Reference equality here is Verus's reference/value equality, not a new
pointer-address or allocation-identity theory. The result is tied to the
actual lock receiver and returned guard, but asserts neither guard
freshness nor equality between different guards. No contents function of
`&Mutex<T>` appears. A retained ghost observation describes an earlier
acquisition, not the registry at function return or after a later lock.

## Primary source justification

`cargo metadata --locked --offline` selects parking_lot 0.12.4 and
lock_api 0.4.13. Current dependency locations and exact compiled externs
are resolved by the experiment scripts, not copied from an old machine.
Parking_lot `src/mutex.rs:86,100` defines the mutex and guard aliases.
Lock_api `src/mutex.rs` provides:

- `138-141`: private `UnsafeCell<T>` data in Mutex.
- `205-210,222-227`: guard construction stores `self`; lock acquires the
  raw mutex before calling that constructor.
- `504-507`: guard stores `&'a Mutex<R,T>` and the original mutable-data /
  sendability PhantomData marker.
- `651-656`: Deref returns `&*self.mutex.data.get()`.
- `659-674`: mutable access requires `&mut guard`; Drop unlocks.
- `546-563,637-648`: temporary unlock and bump require mutable guard
  access; they do not warrant a preserved observation.

The RawMutex safety contract requires actual exclusion (`src/mutex.rs:31-35`).
The generic interface relies on that existing unsafe-library obligation;
it does not certify arbitrary incorrect unsafe RawMutex implementations.
The production selection is parking_lot's RawMutex.

This leaf cannot be discharged by the current project proof alone:
the selected external dependency's private UnsafeCell/raw synchronization
implementation is not natively annotated or included in the verification
closure. No compatible direct parking_lot interface exists in pinned vstd.
Vstd cells/invariants are different runtime types, not replacements for the
frozen expression. The existing generic Deref/DerefMut traits have empty
contracts (`vstd/std_specs/core.rs:30-44`), so mere trait admission does not
connect a guard borrow to the protected registry.

## Native evidence and rejected constructions

Use `probes.py` in this directory. It calls the pinned native verifier on
the real compiled parking_lot dependency; lifetime and trait-conflict
checks stay enabled, with no raised rlimit.

- `observe`: the original `*mutex.lock()` expression and a proof using
  the returned guard's origin and borrow verify.
- Three positive controls verify same-guard reads, distinct origins under
  an explicit distinct-mutex premise, and a write/read through one live
  mutable reference.
- Six negative functions fail at the intended assertions: invented
  cross-mutex contents, invented cross-mutex origin, reacquisition
  persistence, release/mutation/reacquisition persistence, stale
  observation after mutation, and a post-mutation guard frame.
- Lifetime controls reject escaping the borrow, use after scoped guard
  destruction, and simultaneous immutable/mutable guard access.

Native vstd already admits DerefMut with an empty contract. Consequently
`*guard = 7` is admitted but havocs the guard model: neither a later
`*guard == 7` nor origin preservation is available. This is intentionally
weaker than the actual implementation, not a false persistence guarantee.
No extra mutable interface is needed for this read-only result.

Aliases alone fail the exact-generic-signature checks. Omitting the
RawMutex trait declaration fails the enabled trait-conflict checker.
The `assume_specification` spelling generates unsafe Rust and violates
state_unit's `forbid(unsafe_code)`; safe external_fn_specification wrappers
express the same external contracts without changing that policy.
A trial mutable contract was unnecessary and removed; its unsized value
equality also failed Rust typing. Explicit `std::mem::drop` lacks a native
contract, so the release control uses ordinary scoped destruction instead.

## Production scope demonstrated, and not demonstrated

`production.py baseline` and `production.py candidate` generate isolated
copies from the current real state_unit source and apply the existing
`destination-inventory-observation/native-admission.patch`. The candidate
adds only the external declaration module. The inventory expression is
unchanged byte for byte. Cargo fingerprint references select exact
dependency artifacts and Verus exports, including transitive exports.
The verifier's native `--internal-test-mode` prevents injecting a second,
incompatible bundled builtin/vstd; it does not disable lifetime, trait,
erasure, or proof checks. No dependency is rebuilt or edited.

The baseline reproduces all eight known admission errors. With the
candidate, all four mutex-specific errors disappear. Four independent
datatype errors remain: HashMap, RandomState, StateRequest, and
SensitivityLevel. Therefore inventory still does not reach VCs and is
not proved. Do not infer production proof sufficiency from the primitive
passes, or silently supply interfaces for those four types.

The separate Arc<str> ToString refinement remains unproved. Existing
BTreeMap values semantics already cover ordered enumeration; no new
iterator trust is justified. Once the boundary is decided, subsequent
work must prove the ordered inventory observation, connect its names to
the accepted component identity encoding, and establish the actual
TOP-owned registration frame. Loaded representation, payload decoding,
the four recorded representation bridges, and TOP correctness remain
unproved. The inventory-body proof must not resume on this experimental
trust before that boundary decision.

Live production sources and frozen manifests are unchanged. Existing
matching full-crate, boundary, specification-drift, and executable-drift
evidence remains applicable; this diagnostic installs no new trust.
