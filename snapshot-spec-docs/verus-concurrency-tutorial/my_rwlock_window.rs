use std::sync::Arc;
use vstd::prelude::*;
use vstd::rwlock::*;
use vstd::thread::*;

verus! {

////////////////////////////////////////////////////////////////////////////////
// The same window [lo, hi] as my_atomic_two_vars.rs, with the same operations
// (`widen_down`, `widen_up`), but protected by a lock instead of two atomics.
//
// What the lock gives us here: we can read BOTH fields in one critical section, so the
// final `lo <= hi` is a fact about values the program actually holds (exec-visible),
// not only about ghost permissions. (With the atomics we could only state it in proof
// mode, because two atomics cannot be read in one atomic step.)
//
// `reset_to` is an EXTRA operation that the atomic version does not have, and it is
// not a widening: it replaces the window. Inside its critical section the invariant is
// temporarily broken (see the comments there). That is fine under a lock, because the
// invariant only has to hold at release. With two separate atomics each write is its
// own step, so this is not safe as two independent writes when callers run concurrently.
//
// The permission is hidden: vstd's `RwLock` keeps it inside the lock.
// `acquire_write` hands us the data itself and `release_write` takes it back. The
// lock's predicate plays the role of the invariant: assume it after acquiring,
// re-prove it before releasing.
////////////////////////////////////////////////////////////////////////////////

pub struct Window {
    pub lo: u64,
    pub hi: u64,
}

pub struct LoLeHi;

impl RwLockPredicate<Window> for LoLeHi {
    open spec fn inv(self, w: Window) -> bool {
        w.lo <= w.hi
    }
}

type Lock = RwLock<Window, LoLeHi>;

// lo = min(lo, n)
pub fn widen_down(lock: &Lock, n: u64) {
    let (mut w, handle) = lock.acquire_write(); // blocks until we hold the lock
    if n < w.lo {
        w.lo = n;
    }
    handle.release_write(w); // unlike std's lock, the release is an explicit call
}

// hi = max(hi, n)
pub fn widen_up(lock: &Lock, n: u64) {
    let (mut w, handle) = lock.acquire_write();
    if n > w.hi {
        w.hi = n;
    }
    handle.release_write(w);
}

// Widen the window so that it contains n. Here a plain if / else-if is fine: the lock makes
// "read, decide, write" one critical section, so the decision cannot go stale. (With atomics,
// the fetch_min / fetch_max instructions play this role; see my_atomic_two_vars.rs.)
pub fn widen(lock: &Lock, n: u64) {
    let (mut w, handle) = lock.acquire_write();
    if n < w.lo {
        w.lo = n; //widen down
    } else if n > w.hi {
        w.hi = n; //widen up
    }
    handle.release_write(w);
}

// Replace the window with [a, b].
pub fn reset_to(lock: &Lock, a: u64, b: u64)
    requires
        a <= b,
{
    let (mut w, handle) = lock.acquire_write();
    w.lo = a;
    // lo <= hi is BROKEN here if a > the old hi (e.g. [20, 30] -> lo = 50 gives [50, 30]).
    // Under the lock nobody else can see this state, so it is fine.
    w.hi = b;
    // lo = a <= b = hi: the invariant holds again, and release_write checks exactly that.
    handle.release_write(w);
}

fn spawn_widen_down(lock: &Arc<Lock>, n: u64) -> (h: JoinHandle<()>) {
    let lock = lock.clone();
    spawn(move || -> (r: ()) { widen_down(&lock, n) })
}

fn spawn_widen_up(lock: &Arc<Lock>, n: u64) -> (h: JoinHandle<()>) {
    let lock = lock.clone();
    spawn(move || -> (r: ()) { widen_up(&lock, n) })
}

fn spawn_widen(lock: &Arc<Lock>, n: u64) -> (h: JoinHandle<()>) {
    let lock = lock.clone();
    spawn(move || -> (r: ()) { widen(&lock, n) })
}

fn spawn_reset_to(lock: &Arc<Lock>, a: u64, b: u64) -> (h: JoinHandle<()>)
    requires
        a <= b,
{
    let lock = lock.clone();
    spawn(move || -> (r: ()) { reset_to(&lock, a, b) })
}

fn run_threads() -> (out: (u64, u64))
    ensures
        out.0 <= out.1,
{
    let lock = Arc::new(Lock::new(Window { lo: 50, hi: 50 }, Ghost(LoLeHi)));

    let d0 = spawn_widen_down(&lock, 40);
    let d1 = spawn_widen_down(&lock, 10);
    let d2 = spawn_widen_down(&lock, 30);

    let u0 = spawn_widen_up(&lock, 60);
    let u1 = spawn_widen_up(&lock, 90);
    let u2 = spawn_widen_up(&lock, 70);

    let w0 = spawn_widen(&lock, 5);
    let w1 = spawn_widen(&lock, 95);

    // Not widening: these replace the window, e.g. [50, 60] or [20, 30].
    let r0 = spawn_reset_to(&lock, 50, 60);
    let r1 = spawn_reset_to(&lock, 20, 30);

    let _ = d0.join();
    let _ = d1.join();
    let _ = d2.join();
    let _ = u0.join();
    let _ = u1.join();
    let _ = u2.join();
    let _ = w0.join();
    let _ = w1.join();
    let _ = r0.join();
    let _ = r1.join();

    // Both fields are read in one critical section, so they are consistent.
    let handle = lock.acquire_read();
    let w = handle.borrow();
    let result = (w.lo, w.hi);
    handle.release_read();
    result
}

fn main() {
    let (lo, hi) = run_threads();
    assert(lo <= hi);
}

} // verus!
