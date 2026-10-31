// Plain Rust (NOT Verus): the same window [lo, hi] as my_rwlock_window.rs, protected by
// std::sync::RwLock. Shown in chapter 25 only to compare the two lock APIs.
use std::sync::{Arc, RwLock};
use std::thread;

struct Window {
    lo: u64,
    hi: u64,
}

fn widen_down(lock: &RwLock<Window>, n: u64) {
    let mut w = lock.write().unwrap(); // guard: blocks until we hold the lock
    if n < w.lo {
        w.lo = n;
    }
} // the guard is dropped here, and that releases the lock automatically

fn widen_up(lock: &RwLock<Window>, n: u64) {
    let mut w = lock.write().unwrap();
    if n > w.hi {
        w.hi = n;
    }
}

fn main() {
    let lock = Arc::new(RwLock::new(Window { lo: 50, hi: 50 }));

    let a = lock.clone();
    let b = lock.clone();
    let t0 = thread::spawn(move || widen_down(&a, 10));
    let t1 = thread::spawn(move || widen_up(&b, 90));
    t0.join().unwrap();
    t1.join().unwrap();

    let w = lock.read().unwrap(); // a read guard
    assert!(w.lo <= w.hi);
    println!("window = [{}, {}]", w.lo, w.hi);
}
