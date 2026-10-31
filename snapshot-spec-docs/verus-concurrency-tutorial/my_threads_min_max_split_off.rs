use vstd::prelude::*;
use vstd::thread::*;

verus! {

////////////////////////////////////////////////////////////////////////////////
// Threads that share nothing.
//
// Two threads each scan their OWN half of a list of numbers and report the smallest
// and the largest value they saw. The parent combines the two answers.
// Nothing is shared between the threads, so ownership is enough.
////////////////////////////////////////////////////////////////////////////////

// "Every element of v lies between lo and hi."
pub open spec fn within(v: Seq<u64>, lo: u64, hi: u64) -> bool {
    forall|i: int| 0 <= i < v.len() ==> lo <= #[trigger] v[i] && v[i] <= hi
}

// "[lo, hi] is the tightest window of v": every element lies in it, and both ends
// are actual elements of v. So lo is the smallest element and hi is the largest.
pub open spec fn tight(v: Seq<u64>, lo: u64, hi: u64) -> bool {
    within(v, lo, hi)
    && (exists|i: int| 0 <= i < v.len() && v[i] == lo)
    && (exists|i: int| 0 <= i < v.len() && v[i] == hi)
}

// Scan one piece. Returns the tightest window of its elements.
fn scan(v: &Vec<u64>) -> (out: (u64, u64))
    requires
        v.len() > 0,
    ensures
        tight(v@, out.0, out.1),
{
    let mut lo = v[0];
    let mut hi = v[0];
    let mut i = 1;
    proof {
        // the first element is both the smallest and the largest of the first 1 elements
        assert(v@.take(1)[0] == lo);
    }
    while i < v.len()
        invariant
            1 <= i <= v.len(),
            tight(v@.take(i as int), lo, hi),
        decreases v.len() - i,
    {
        let x = v[i];
        let ghost old_lo = lo;
        let ghost old_hi = hi;
        if x < lo {
            lo = x;
        }
        if x > hi {
            hi = x;
        }
        i += 1;
        proof {
            // the first i elements are the old first (i - 1) elements, plus v[i - 1]
            let old = v@.take((i - 1) as int);
            let new = v@.take(i as int);
            assert(new =~= old.push(v@[i - 1]));
            // lo and hi are still elements: either the old ones, or the new element v[i - 1]
            if lo == old_lo {
                let j = choose|j: int| 0 <= j < old.len() && old[j] == old_lo;
                assert(new[j] == lo);
            } else {
                assert(new[i - 1] == lo);
            }
            if hi == old_hi {
                let j = choose|j: int| 0 <= j < old.len() && old[j] == old_hi;
                assert(new[j] == hi);
            } else {
                assert(new[i - 1] == hi);
            }
        }
    }
    assert(v@.take(v.len() as int) =~= v@);
    (lo, hi)
}

// Split the list in two owned halves; one thread scans each half.
fn scan_with_two_threads(v: Vec<u64>) -> (out: Result<(u64, u64), ()>)
    requires
        v.len() >= 2,
    ensures
        out is Ok ==> tight(v@, out.unwrap().0, out.unwrap().1),
{
    let ghost whole = v@;
    let mid = v.len() / 2;
    let mut left = v;
    let right = left.split_off(mid);
    let ghost left_seq = left@;
    let ghost right_seq = right@;

    // Each closure OWNS one half: it was moved in. The closure's `ensures` is the
    // only thing the parent learns about the thread's work.
    let handle_left = spawn(
        move || -> (r: (u64, u64))
            ensures
                tight(left_seq, r.0, r.1),
            { scan(&left) },
    );
    let handle_right = spawn(
        move || -> (r: (u64, u64))
            ensures
                tight(right_seq, r.0, r.1),
            { scan(&right) },
    );

    if let (Ok(l), Ok(r)) = (handle_left.join(), handle_right.join()) {
        let lo = if l.0 < r.0 {
            l.0
        } else {
            r.0
        };
        let hi = if l.1 > r.1 {
            l.1
        } else {
            r.1
        };
        proof {
            // v = left ++ right, so a window that contains both halves contains all of v
            assert(whole =~= left_seq + right_seq);
            assert forall|i: int| 0 <= i < whole.len() implies lo <= #[trigger] whole[i]
                && whole[i] <= hi by {
                if i < left_seq.len() {
                    assert(whole[i] == left_seq[i]);
                } else {
                    assert(whole[i] == right_seq[i - left_seq.len()]);
                }
            }
        }
        Ok((lo, hi))
    } else {
        Err(())
    }
}

fn main() {
    let mut v: Vec<u64> = Vec::new();
    v.push(50);
    v.push(10);
    v.push(90);
    v.push(30);
    let ghost s = v@;
    assert(s =~= seq![50u64, 10, 90, 30]);
    if let Ok((lo, hi)) = scan_with_two_threads(v) {
        // lo is an element and every element is >= lo, so lo is the smallest element
        assert(lo == 10 && hi == 90);
    }
}

} // verus!
