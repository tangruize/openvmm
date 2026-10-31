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
