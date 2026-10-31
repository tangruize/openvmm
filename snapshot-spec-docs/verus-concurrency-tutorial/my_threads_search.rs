use vstd::prelude::*;
use vstd::thread::*;

verus! {

////////////////////////////////////////////////////////////////////////////////
// Threads that share nothing.
//
// Search an unordered list for a value. The list is cut in two halves, and one thread
// searches each half. The parent combines the two answers.
// Nothing is shared between the threads, so ownership is enough.
////////////////////////////////////////////////////////////////////////////////

// What a correct answer to "where is k in s?" looks like:
// `Some(i)` means s[i] == k; `None` means k does not occur in s.
pub open spec fn search_result(s: Seq<u64>, k: u64, r: Option<usize>) -> bool {
    match r {
        Some(i) => i < s.len() && s[i as int] == k,
        None => forall|i: int| 0 <= i < s.len() ==> s[i] != k,
    }
}

// Search one piece, from the front.
fn search(v: &Vec<u64>, k: u64) -> (r: Option<usize>)
    ensures
        search_result(v@, k, r),
{
    let mut i = 0;
    while i < v.len()
        invariant
            i <= v.len(),
            forall|j: int| 0 <= j < i ==> v[j] != k,
        decreases v.len() - i,
    {
        if v[i] == k {
            return Some(i);
        }
        i += 1;
    }
    None
}

// Split the list in two owned halves; one thread searches each half.
fn search_with_two_threads(v: Vec<u64>, k: u64) -> (out: Result<Option<usize>, ()>)
    ensures
        out is Ok ==> search_result(v@, k, out.unwrap()),
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
        move || -> (r: Option<usize>)
            ensures
                search_result(left_seq, k, r),
            { search(&left, k) },
    );
    let handle_right = spawn(
        move || -> (r: Option<usize>)
            ensures
                search_result(right_seq, k, r),
            { search(&right, k) },
    );

    if let (Ok(l), Ok(r)) = (handle_left.join(), handle_right.join()) {
        // v = left ++ right: an index in the right half is shifted by the length of the left
        proof {
            assert(whole =~= left_seq + right_seq);
        }
        match (l, r) {
            (Some(i), _) => Ok(Some(i)),
            (None, Some(j)) => Ok(Some(mid + j)),
            (None, None) => {
                proof {
                    assert forall|i: int| 0 <= i < whole.len() implies whole[i] != k by {
                        if i < left_seq.len() {
                            assert(whole[i] == left_seq[i]);
                        } else {
                            assert(whole[i] == right_seq[i - left_seq.len()]);
                        }
                    }
                }
                Ok(None)
            },
        }
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
    if let Ok(Some(i)) = search_with_two_threads(v, 90) {
        assert(s[i as int] == 90);
    }
}

} // verus!
