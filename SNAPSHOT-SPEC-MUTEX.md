# Verified locks with lock invariants

Trusted Verus specifications of the two locks on the snapshot path, so
that verified code can rely on a property of the value a lock protects,
while the runtime locks stay the real ones: `parking_lot::Mutex` (below)
and OpenVMM's `CloseableMutex` (its own section, at the end).

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
    type Param = Loc;   // what each mutex gives the rule (e.g. a ghost's id)

    open spec fn inv(id: Loc, v: MyData) -> bool {
        // the rule on the contents, for a mutex with parameter `id`
    }
}

type MyMutex = Mutex<MyData, MyPred>;
```

The rule's form is given by the type; each mutex has its own parameter,
chosen when it is created, so the rule can name that mutex's own ghost
state (see "Design choices").

### 2. Create it

```rust
let m = MyMutex::new_verified(data, Ghost(id));   // requires MyPred::inv(id, data)
// m.param() == id
```

### 3. In verified code: acquire, then release

```rust
let (mut data, handle) = m.acquire();   // MyPred::inv(m.param(), data) holds
... read or change `data` ...
handle.release(data);                   // requires MyPred::inv(m.param(), data)
```

`acquire` moves the contents out while the lock is held; `release` moves
them back and unlocks.

### 4. In unverified code: lock as usual

```rust
let mut guard = m.lock();   // an ordinary guard (`MappedMutexGuard`)
guard.field = ...;
```

`lock` is hidden from Verus, so verified code cannot call it. Unverified
code is assumed to keep the rule (its users are to be verified). Likewise
unverified code may create a mutex with `new(v)`, whose parameter is then
unknown. This is what let `StateUnits` switch to
the wrapper without changing any of its existing `.lock()` call sites.

## API

| Item | Contract |
|---|---|
| `trait MutexPredicate<V> { type Param; spec fn inv(param: Param, v: V) -> bool; }` | the lock invariant's form, named by a type, for each mutex's parameter |
| `m.param()` | the mutex's own parameter, given when it was created |
| `Mutex::<V, Pred>::new_verified(v, Ghost(p))` | requires `Pred::inv(p, v)`; ensures `m.param() == p` |
| `m.acquire() -> (V, MutexHandle)` | ensures `Pred::inv(m.param(), value)`; the handle has the same parameter; holds the lock until `release` |
| `handle.release(v)` | requires `Pred::inv(handle.param(), v)`; unlocks |
| `Mutex::new(v)`, `m.lock()` (unverified code only) | create / lock without a contract |

## What is trusted

Everything in `mutex_t.rs` is trusted (also listed in
`SNAPSHOT-SPEC-TRUSTED.md`):

- **The lock is a lock**: `parking_lot::Mutex` gives mutual exclusion, so
  the contents seen by `acquire` are those left by the last `release` (or
  unverified `lock`).
- **Unverified code keeps the rule** (an assumption about our code, to
  prove when those users are verified): anything changed through `lock()`
  satisfies the rule when the guard is dropped, and a mutex created with
  `new` holds a value satisfying it.

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
- **Each mutex's own rule, as in vstd** (since code change 94; decided by
  the user, 2026-10-31). vstd's `RwLock` takes its predicate as a ghost
  value at construction, so the rule can mention per-instance constants
  (such as ghost ids). Here the rule's form is named by a type (`Pred`, so
  `Mutex<V, Pred>` keeps its runtime shape), and each mutex gets its own
  ghost **parameter** at construction (`new_verified(v, Ghost(p))`), which
  the rule takes: the same power, split into a form and a value.
  `StateUnits` uses it to tie its registry's ghost state to its own: the
  registry's parameter is the id of a ghost record shared by the two.
  (Until code change 94 the rule was static, a function of the contents
  only, with a **key**, a constant of the value every holder kept, standing
  in for the parameter.)
- **Normal builds.** In a non-Verus build the trait has no items, and a
  blanket impl makes every type satisfy it, so `Mutex<V, Pred>` works for
  any `Pred`.

## Status and limitations

- **Tested**: in `state_unit`, the registry uses it (`collect_halves`
  verifies with it); the `state_unit` tests pass.
- **No `try_lock`, no condition variables**; only what `StateUnits` needed.
- **Where it should live.** A vstd version would specify
  `std::sync::Mutex` rather than `parking_lot`, would take its rule as a
  ghost value like `RwLock` (as this one now takes its parameter), and would
  leave out the unverified `lock()` escape hatch, which exists here only so
  that unverified OpenVMM code could keep its call sites unchanged.

## The other lock: `CloseableMutex` (chipset devices)

Each chipset device sits behind OpenVMM's own lock, `CloseableMutex`
(`support/closeable_mutex`), reached by the device's unit and by vCPU
threads. Its trusted specification
(`support/closeable_mutex/src/closeable_mutex_t.rs`) follows the same
model as the box above, with two differences.

**The rule's form comes from the value's type.** `CloseableMutex<T>` has
no type parameter for a predicate, so the rule's form is given by `T`
(`trait LockInvariant { type Param; spec fn inv(&self, param) }`); each
mutex still gets its own parameter when it is created, as above
(`new_verified(value, Ghost(param))`, `m.param()`). For a device, the
parameter is the id of the device's own fractional ghost (and its
`omit`), so each device's lock says "this slot holds this device's
ghost" (code change 94).

**It can be closed.** Besides locking, its holder can **close** it: a
device's unit does, for each state operation (stop, save, restore, …), to
have the device to itself; closing gives the value until the closed
guard is reopened. While it is closed, `lock` **panics**, and so does
closing it again. The specification states those panics as
preconditions (code change 95), the way vstd states a `Vec` index bound:

| Item | Contract |
|---|---|
| `CloseableMutex::new_verified(value, Ghost(param))` | requires `value.inv(param)`; returns the mutex and its **close token** (a `FracGhost<()>` with id `m.close_id()`), whole |
| `CloseableMutex::close_verified(&m, token)` | requires the **whole** close token; gives a closed guard with the value (satisfying the rule), which keeps the token |
| closed guard: `borrow`, `borrow_mut`, `release()` | read / change the value; `release` requires the rule, reopens the mutex, and gives the whole token back |
| `m.lock_verified(&token)` | requires a **piece** of the close token; gives an open guard with the value (satisfying the rule) |
| open guard: `borrow_mut`, `release()` | change the value; `release` requires the rule |
| `lock`, `lock_if_open`, `close`, `new` (unverified code only) | as before, without contracts |

Whoever holds a piece of the close token knows nobody holds the whole, so
nobody has the mutex closed: `lock_verified` cannot panic, nor can
`close_verified` (it needs the whole). Who holds which piece when (the
unit, the vCPUs) is for the proof of the code that locks a device; it
follows from the order in which units stop and start.

What is trusted, as above: the lock is a lock (mutual exclusion; closing
excludes everyone else until reopened), and unverified code keeps the
rule. A closed guard dropped without `release` leaves the mutex closed
(later `lock`s panic), and an open guard dropped without `release` leaves
it locked: safe, not unsound.
