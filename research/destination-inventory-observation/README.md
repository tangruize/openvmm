# Production destination-inventory observation

## Unresolved bindings recorded before verification experiments

The selected obligation is the unchanged `StateUnits::inventory` body in
`vmm_core/state_unit/src/lib.rs`. This is not an entry-to-return registry
frame or completion of `LoadedVm::restore_snapshot_state`.

The source-supported candidate observation is: for the `Inner.units` map
actually borrowed through this invocation's mutex guard, enumerate every
key exactly once in ascending `u64` order, and return the corresponding
`Unit.name` strings in that order. An empty observed map returns an empty
vector. No saving, payload lookup, RPC, or component callback occurs in this
body. Consequently the observation must not depend on any saved blob or
callback succeeding. These are proof obligations, not proved postconditions.

Before constructing that contract, the unresolved interface facts are:

- Native admission of the actual `Arc<parking_lot::Mutex<Inner>>`, its
  protected borrow, and the guard lifetime. A value chosen independently
  of the acquired guard is not an observation of the actual registry.
- Native semantics for this exact `BTreeMap::values().map(...).collect()`
  expression, including ascending key order, complete coverage, no extra
  elements, and preservation of each `Arc<str>`'s Unicode contents by
  `to_string()`. A copied loop or trusted inventory accessor is not a proof.
- Binding the observed names to the accepted injective `component_name_id`
  in `openvmm_defs::worker::saved_state_proof`, without a dependency cycle
  or a second identity encoding.
- A caller-usable observation witness tied to this registry and acquisition,
  rather than a claim about registry contents after guard release.
  `UnitHandle::remove_if` can independently remove entries.

The selected observation needs no consistency assumption about
`Inner.names`: the body reads only `Inner.units`. Relating restore lookup to
the observation would additionally require proving consistency through
`StateUnits::new`, `UnitBuilder::build`, and `UnitHandle::remove_if`.
Uniqueness of returned *names* likewise cannot be assumed from uniqueness
of BTreeMap keys; name uniqueness requires its own construction invariant.

Source connections checked at intake:
`LoadedVm::save` stores this vector in `SavedState.inventory`;
`LoadedVm::restore` invokes `validate_inventory` only for nonempty saved
inventory, and that method compares the actual vectors for equality.
TOP calls `LoadedVm::restore`. The frozen compatibility predicate compares
component-identity sets and the frozen projection copies the request's set.
The accepted saved-side proofs already bind those sets to the actual
`SavedState.inventory`. Destination binding is still owed.

Even a proved observer would leave the actual `LoadedVm` owner-bundle
invariant, its preservation through callbacks and awaits, payload decoding,
complete `loaded_vm_representation`, and TOP body proof outstanding.
The accepted construction/frame source analysis is in
`../restore-inventory-preservation/README.md`; none of its source-supported
ownership conclusions is silently promoted to a Verus invariant here.

## Result: diagnostic, not an observation proof

The real production body cannot currently reach verification conditions with
the available external interfaces. `native-admission.patch` removes only
`StateUnits`' datatype opacity, admits its local `Inner`, `Unit`, and `State`
declarations, imports the annotation macros, and selects the real inventory
body. It changes no executable expression and supplies no postcondition or
assumption. The focused native run fails at the actual `.lock()` expression:

```text
lock_api::mutex::MutexGuard is not supported
parking_lot::raw_mutex::RawMutex is not supported
lock_api::mutex::impl&%4::lock is not supported
lock_api::mutex::Mutex is not supported
```

Translation additionally reports the carried `std::collections::HashMap`
and `std::hash::RandomState` in `Inner.names`, and the project-owned
`StateRequest` and `inspect::SensitivityLevel` in `Unit`. The `names` map
is not read by inventory, but its datatype must still be admitted when
`Inner` is transparent. Its missing datatype declaration is **not** a reason
to assume its consistency with `units`. The two project-owned declarations
are ordinary native-admission work, not proposed permanent trust.

The focused run took 5.058 seconds, exit 101, with eight frontend errors.
Its wrapper prints `0 verified, 0 errors` because no verification-condition
report was emitted; that line is not success. Full diagnostics are in
`.verus_agent/cache/destination-inventory-native.log`.
The annotation overlay was then reversed exactly. The pre-existing
`SavedStateUnit` annotation and every other intake change remain intact.

### Isolated interface evidence

`lock-admission.rs` isolates just `*lock.lock()` for `Mutex<u64>`, with
no registry, name map, invariant, postcondition, or temporary trust.
It independently produces the same four missing external interfaces
(exit 1, 0.55 seconds), recorded in
`.verus_agent/cache/destination-inventory-lock.log`.
This is a missing library interface, not an SMT timeout or a counterexample
to the desired inventory observation.

The resolved primary implementation is `parking_lot` 0.12.4's
`src/mutex.rs:86,100`, which aliases `lock_api` 0.4.13's real mutex and guard.
In the latter's `src/mutex.rs`, the mutex holds a private `UnsafeCell<T>`
(138-141); `lock` acquires the raw mutex and constructs a guard (205-227);
guard dereference reads that mutex's data (651-656); guard destruction
unlocks it (666-673). The pinned vstd has no `parking_lot`/`lock_api`
interface. Adding a project-owned opaque inventory accessor or a lemma
assuming its result would not discharge this external observation boundary.
The known mutable-reborrow identity repair does not apply: native type
and operation translation fails before such a verification condition exists.

`arc-name.rs::name_string` independently checks the exact name conversion
used by the production closure. The postcondition
`result@ == (**name)@` fails (0 verified, 1 error, 0.40 seconds).
The `str_string` control, using the already-supported `str` receiver, passes
(1 verified, 0 errors, 0.68 seconds). The logs are
`.verus_agent/cache/destination-inventory-{arc-name,str-name}.log`.
The failed Arc postcondition is source-supported but **unproved**:
the pinned Rust `alloc/src/sync.rs:3693-3696` delegates `Arc<T>`'s Display
to `T`'s Display. Pinned `vstd/string.rs:168-186` only connects the generic
ToString postcondition to string contents for `str`, not `Arc<str>`.
This evidence does not justify changing the production receiver or assuming
the desired postcondition.

Conversely, the iterator is **not** shown to need a new trusted interface.
Pinned `vstd/std_specs/btree.rs:1000-1018` already specifies `values` using
an increasing, duplicate-free key sequence covering the exact map domain.
`std_specs/iter.rs` supplies `map` and `collect` contracts. These facts rule
out attributing the present frontend failure to missing BTreeMap order
semantics; using these existing interfaces on the real body remains proof work.

No external declaration, trusted lock/collection interface, invariant,
representation bridge, or caller contract was installed. No frozen change
has been tested or submitted as a valid freeze request. The evidence
identifies the external interface frontier; it does not establish a
particular sound, sufficient extension of the frozen TCB. In particular,
opaque mutex type declarations alone would not establish the required
connection to the actual protected registry. Designing that external
resource/guard interface (or natively proving the dependency outside this
mission's writable paths) must precede an actionable boundary proposal.
The selected proof cannot be handed off as proof or progress in this state.

## Exact observer still owed

Let `S` denote **this invocation's actual protected `units` map**, captured
through its acquired guard. The useful result is a snapshot receipt tied
to that registry acquisition, together with a sequence of IDs `keys` such
that:

- `keys` is strictly increasing, has no duplicates, and covers `S.dom()`;
- the returned vector has `keys.len()` elements, and at each valid index
  its String View equals the Unicode contents of `S[keys[i]].name`.

The empty-map result follows from these clauses. Every observed unit is
included even if saving returns no blob; no blob or callback enters this
observation. Duplicate *names*, if present in an otherwise valid observed
map, would still be enumerated at their distinct IDs. No constructor
invariant has been proved or assumed to rule them out.

Using the already-proved `component_name_id` and membership lemmas, the
identity set of that vector must then equal exactly the image of these
observed names under the **same** accepted encoding. This is the destination
meaning needed by the frozen compatibility equality and inventory projection;
it is not currently an available Verus postcondition of `inventory`.
Moving the common name encoding to a lower-level owner is unnecessary until
the production observer can actually be admitted, and was not attempted.

After guard release, independent `UnitHandle`s can remove entries.
Consequently neither a stable `StateUnits` View nor equality to registry
contents at function return can be inferred from a shared receiver.
The later owner-bundle proof must tie this acquisition to the actual
load-created registry and preserve its registrations through the TOP
interval. This is separate from the native enumeration proof and remains
unproved even after the external interfaces are supplied.

## Reproduction and scope

From the repository root, review and apply `native-admission.patch` only
as a temporary diagnostic overlay, then run:

```bash
PATH="$(dirname "$(verification/tools/find-verus.sh)"):$PATH" \
VERUS_Z3_PATH="$(verification/tools/find-z3.sh)" \
"${ARGUS_SKILL_PYTHON:-python3}" verification/tools/fresh_verification.py \
    focus state_unit -- --verify-root --verify-function 'StateUnits::inventory' \
    --multiple-errors 10 --num-threads 1 --triggers-mode silent
```

Reverse that overlay afterward; do not revert the file's intake changes.
`git apply --check` accepts the preserved overlay. The two small probes use
the same native verifier, `--edition=2024 --crate-type=lib --num-threads 1`;
the lock probe additionally imports the Cargo-selected `parking_lot` rmeta
and its dependency directory. The str control selects
`--verify-root --verify-function str_string`. Neither probe substitutes for
the production implementation or constitutes the selected proof.

The maintained call-graph reader reports missing
`.verus_agent/proof_state.json`. A fresh rust-analyzer caller report was
obtained instead at `.verus_agent/cache/destination-inventory-callers.md`.
It identifies `LoadedVm::save -> inventory`,
`LoadedVm::restore -> validate_inventory -> inventory`, and compiler-inserted
`UnitHandle::drop -> remove_if`. Source inspection confirms TOP's call to
`LoadedVm::restore`. These are reachability evidence, not proof contracts.
The first durable caller command failed runner preflight because its first
executable was an unexpanded environment expression; resubmission with the
resolved authoritative Python completed in 87.6 seconds.

## Current machine evidence and retained debt

The prescribed checks ran once on the intake production sources. All
experimental production annotations were restored to those exact sources;
no build, proof, contract, or manifest change is retained. Accordingly this
current full-crate evidence is reused, not rerun on identical inputs.
Full logs remain in `.verus_agent/cache/checks/<check>/latest.log`.

| Command module (all with `--crate-root .`) | Result | Elapsed |
| --- | --- | --- |
| `argus_verus.tools.checks.make_verify` | Exit 0; 1949 verified, 0 errors | 79.103 s |
| `argus_verus.tools.checks.boundary --baseline-dir .verus_agent check` | Exit 3, INCOMPLETE; 0 permanent violations, 7 temporary locations, 0 assumptions in the scoped scan | 1.173 s |
| `argus_verus.tools.checks.spec_drift --baseline-dir .verus_agent` | Exit 0; no frozen specification drift | 1.383 s |
| `argus_verus.tools.checks.exec_drift --baseline-dir .verus_agent` | Exit 0; no executable drift | 5.161 s |

The full selection is `openvmm_core` and its 14 Verus-enabled packages,
not a full proof of every executable body. Lifetime and trait-conflict
checking were enabled in all verification runs; no rlimit annotation was
added or changed. No timing target or bottleneck claim is inferred.

No temporary marker was added, removed, or moved in the final sources.
The temporary removal of `StateUnits`' datatype opacity during the native
experiment was reversed. The seven scoped debt locations remain:
`InitializedVm`, `LoadedVmInner`, TOP's body, and the four bridges
`initialized_vm_representation`, `decoded_load_restore_request_view`,
`decoded_saved_payload_view`, and `loaded_vm_representation`.
Their respective obligations are construction/loaded representation, real
TOP execution, wrapper request decoding, and named mutable-payload decoding.
None proves destination inventory.

Outside that scoped scan, the existing `StateUnits`, `SavedStateUnit`,
`StopGuard`, mesh mpsc `Sender`/`Receiver`, `Rpc`, and
`MicrovmSnapshotBoundaryRequest` datatype markers still defer registry,
payload, channel, and lifetime representations. The saved-name and
request-time proofs remain intact. The pre-existing compatibility
declarations in `openvmm_core/src/verus_compat.rs` (anyhow error/type helpers
and Arc clone) are still unsanctioned by the frozen TCB; the scoped boundary
result does not authorize them. Their precise source locations, consumers,
and removal obligations remain in `verification/restore/UNINTERP.json`.
