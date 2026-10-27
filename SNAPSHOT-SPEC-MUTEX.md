# Verified mutex with a lock invariant

A trusted Verus specification of `parking_lot::Mutex`, so that verified
code can rely on a property of the value a mutex protects, while the
runtime lock stays the real `parking_lot::Mutex`.

Code: `vmm_core/state_unit/src/mutex_t.rs` (trusted). Its first user is
`StateUnits`' registry (`InnerMutex = Mutex<Inner, InnerPred>` in
`vmm_core/state_unit/src/lib.rs`).

## The mental model: a box with a rule

The mutex is a box with a rule written on it (the lock invariant). Whenever
the box is closed (unlocked), its contents satisfy the rule.

- Taking the lock opens the box: you get the contents and may assume the
  rule.
- While you hold it, you may break the rule temporarily.
- Releasing the lock closes the box: you must put back contents that
  satisfy the rule again, and Verus checks it.

This is what vstd's `RwLock` offers (its `RwLockPredicate`), and what the
concurrency tutorial's atomic invariants offer for a single atomic step.
vstd has no `Mutex`; this fills that gap for `parking_lot::Mutex`.

A lock invariant only gives a property that holds whenever the lock is
free, not exact knowledge of the contents: others may change them between
your acquisitions. For exact knowledge, hold a FracGhost half outside the
lock (see `SNAPSHOT-SPEC-DESIGN.md`).

## How to use it

### 1. Name the rule

```rust
enum MyPred {}   // a type that names the rule; never constructed

impl MutexPredicate<MyData> for MyPred {
    open spec fn inv(v: MyData) -> bool {
        // the rule on the contents
    }
}

type MyMutex = Mutex<MyData, MyPred>;
```

The rule is static: a function of the contents only (see "Design
choices").

### 2. Create it

```rust
let m = MyMutex::new(data);   // requires MyPred::inv(data)
```

### 3. In verified code: acquire, then release

```rust
let (mut data, handle) = m.acquire();   // MyPred::inv(data) holds
... read or change `data` ...
handle.release(data);                   // requires MyPred::inv(data)
```

`acquire` moves the contents out while the lock is held; `release` moves
them back and unlocks.

### 4. In unverified code: lock as usual

```rust
let mut guard = m.lock();   // an ordinary guard (`MappedMutexGuard`)
guard.field = ...;
```

`lock` is hidden from Verus, so verified code cannot call it. Unverified
code is trusted to keep the rule. This is what let `StateUnits` switch to
the wrapper without changing any of its existing `.lock()` call sites.

## API

| Item | Contract |
|---|---|
| `trait MutexPredicate<V> { type Key; spec fn key(v: V) -> Key; spec fn inv(v: V) -> bool; }` | the lock invariant, named by a type, and the key |
| `Mutex::<V, Pred>::new(v)` | requires `Pred::inv(v)`; ensures `m.key() == Pred::key(v)` |
| `m.acquire() -> (V, MutexHandle)` | ensures `Pred::inv(value)` and `Pred::key(value) == m.key()`; holds the lock until `release` |
| `handle.release(v)` | requires `Pred::inv(v)` and the key unchanged; unlocks |
| `m.lock()` (unverified code only) | an ordinary guard; no contract |

## What is trusted

Everything in `mutex_t.rs` is trusted (also listed in
`SNAPSHOT-SPEC-TRUSTED.md`):

- **The lock is a lock**: `parking_lot::Mutex` gives mutual exclusion, so
  the contents seen by `acquire` are those left by the last `release` (or
  unverified `lock`).
- **Unverified code keeps the rule**: anything changed through `lock()` is
  assumed to satisfy `Pred::inv` when the guard is dropped.

## Design choices

- **Runtime: `parking_lot::Mutex<Option<V>>`.** `acquire` takes the value
  out (`Option::take`) and `release` puts it back, so verified code owns the
  value while holding the lock, as with vstd's `RwLock::acquire_write`. A
  handle dropped without `release` leaves `None`, so the next `lock` or
  `acquire` panics: safe, never a value that breaks the rule. Unverified
  code's `lock()` maps the guard to the value (`MutexGuard::map`), so it
  sees a plain `V`.
- **Why not switch to vstd's `RwLock`.** It is a spinlock built on atomics,
  and its write handle is not released when dropped; switching would change
  OpenVMM's runtime behavior and every early-return path at the lock sites.
- **A static rule (`MutexPredicate`), not a ghost value.** vstd's `RwLock`
  takes its predicate as a ghost value at construction, so the rule can
  mention per-instance constants (such as ghost ids). Here the rule is named
  by a type, so construction needs no ghost argument and the code compiles
  unchanged in a normal build; the cost is that the rule cannot mention
  per-instance constants. A per-instance tie is made through a **key**
  instead: `Pred::key(v)` is a constant of the value that every holder
  keeps (`acquire` gives it, `release` requires it), and `m.key()` is the
  mutex's. `StateUnits` uses it to tie its registry's ghost state to its
  own (the key is the id of a ghost record shared by the two).
- **Normal builds.** In a non-Verus build the trait has no items, and a
  blanket impl makes every type satisfy it, so `Mutex<V, Pred>` works for
  any `Pred`.

## Status and limitations

- **Tested**: in the prototype (`verus-patches/verus-rpc-prototype`,
  `mutex_unit.rs`): a registry whose units leave FracGhost halves in the
  mutex verifies; dropping a precondition breaks the rule and fails; runtime
  tests cover `acquire`/`release` and the unverified `lock()`. In
  `state_unit`, the registry uses it (`collect_halves` verifies with it);
  the `state_unit` tests pass.
- **No `try_lock`, no condition variables**; only what `StateUnits` needed.
- **Where it should live.** A vstd version would specify
  `std::sync::Mutex` rather than `parking_lot`, would likely take its rule
  as a ghost value like `RwLock` (for per-instance constants), and would
  leave out the unverified `lock()` escape hatch, which exists here only so
  that unverified OpenVMM code could keep its call sites unchanged.
