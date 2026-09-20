use vstd::prelude::*;

verus! {

enum Request {
    Reset(u64),
}

fn apply<F: FnOnce(u64) -> Request>(f: F, input: u64) -> (result: Request)
    requires call_requires(f, (input,)),
    ensures call_ensures(f, (input,), result),
{
    f(input)
}

fn test(input: u64) {
    let result = apply(Request::Reset, input);
    assert(result == Request::Reset(input));
}

}

fn main() {}
