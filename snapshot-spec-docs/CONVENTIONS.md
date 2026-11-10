# OpenVMM snapshot spec: conventions

The style rules the snapshot spec follows: how spec functions, views, and
files are organized, and how types are named. New spec code should follow
them.

## Style

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
- **A spec function has the shape of its intent, not of the code's
  algorithm** (the user, 2026-11-08). When the intent is a step-by-step
  process, define it recursively, one step at a time, rather than as the
  result of a search with nested quantifiers ("there exists ... such that,
  for all ..."). Example: the access policy's "the nearest denied or
  allowed path that contains a path decides" is a walk up from the path,
  stopping at the first rule (`first_rule_upward`,
  `virtiofs/src/microvm/policy.spec.rs`); a first version described the
  result of the code's search over all rules (`filter`, `max_by_key`) with
  nested quantifiers, which was correct but hard to read. Nested
  quantifiers also make proofs slower.
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
  - Both are included from `<file>.rs` under `#[cfg(verus_keep_ghost)]`
    (one exception: a `.spec.rs` file whose types a normal build needs,
    because production code names one of them, e.g. ghost state passed as
    a trait's type parameter, as virtio-fs's `Fuse<G>`, is included without
    `cfg`; a normal build then keeps only its type definitions):
    `<file>.spec.rs` as `mod spec`, and `<file>.proof.rs` as `mod proof`
    *inside* `spec`, so that lemmas can unfold the spec's closed views (a
    closed body is visible in its module and the modules within it).
  - `<name>_t.rs`: trusted code (`external_body` wrappers and the
    assumptions they state), as in verified-storage. Each such file begins
    with what it assumes.
  - Proof code that must stay in a production function's body (`proof { }`
    blocks, loop invariants) only calls lemmas from `<file>.proof.rs` and
    states what it needs; the reasoning itself lives in the lemmas.

- **Errors.** A contract says not only what holds on Ok but also **when
  the function may fail** (or, equivalently, when it must succeed), e.g.
  `result is Err ==> (some unit's Stop failed || the deadline passed)`.
  Otherwise a function that always returns an error satisfies its
  contract. The conditions are typically external failures (the
  hypervisor, a mesh channel, a deadline) or bad inputs. Which kind of
  error is returned need not be specified in detail, unless callers act
  on it (e.g. `SnapshotQuiesceError`'s `Rejected`, `RollbackSafe` and
  `Uncertain`, which decide whether a rollback is possible).
  - **The reasons come from what the function is meant to do, not from
    the code's checks** (the user, 2026-11-01). First list the reasons
    the design allows (a requirement, an external failure, an
    out-of-scope area), then compare with the code's error checks. A
    check that no reason justifies is a finding, not a new reason: a
    bug, or a check that can never fail. A copied buggy check would be
    allowed by the spec. Example: the guest-requested snapshot checked
    "the VM boots Linux with an MP table", which is always true for a
    microVM (its configuration is validated when the worker starts); the
    fact went into `LoadedVm::wf()` instead of the error list (design
    doc, "Errors: when a save may fail").

## Checking the conventions

`scripts/verus-style-check.py` checks the rules above that a script can
check, and lists each place that breaks one, with its file and line: free
spec functions and lemmas; spec functions other than `view` and `wf()` on
concrete types; views not named `FooView`; open views that are more than a
mapping of fields; spec and proof functions in production files; and, for
a human to judge, large open spec functions, long proof blocks in
production files, types declared to Verus in production files, and concrete
types without a `wf()`. Run it after writing spec or proof code
(`scripts/verus-style-check.py [PATH ...]`, or `--rev REV` for a git
revision), and refactor until it reports no FIX. A hit that is right as it
is gets a `STYLE OK:` comment with its reason.

## Lessons

The kinds of mistakes the proof agent found in our spec, and the rule
for avoiding each, are in [LESSONS.md](LESSONS.md). Check a new contract
against them.

## Naming

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
- **A plain-value type gets no `View`, and no type is its own view**
  (the user, 2026-11-09). When the spec means exactly a type's value (a
  record of plain fields that the spec reads as they are), the spec uses
  the value itself, as verified-storage does with `SetupParameters`,
  `PersistentMemoryConstants`, and its keys and items (`AtomicKvStore`'s
  `Map<K, (I, Seq<L>)>`). A type is never its own view (`impl View for Foo
  { type V = Foo; }`, `view() = *self`): either it has no `View`, or its
  view is a different, abstract type, which is where the spec means
  something other than the bits (`SaturatingU64` -> `int`, its unbounded
  value; `ConcreteJournalEntry` -> `JournalEntry`, with `u64` -> `int` and
  `Vec<u8>` -> `Seq<u8>`). verified-storage has no `View` impl of the first
  kind.

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
