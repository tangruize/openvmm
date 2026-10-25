# Verus patches

Local fixes to Verus needed by the snapshot specification work. Each patch
applies to Verus commit `426d8b0` ("Support /= and %= for signed integers
(#3013)"), the version that matches the pinned `vstd`
(`=0.0.0-2026-10-04-0306`).

To apply and rebuild (from the Verus checkout):

```sh
git apply /path/to/openvmm/verus-patches/0001-async-fn-unnamed-return-keeps-ensures.patch
cd source
source ../tools/activate   # or put tools/vargo/target/release on PATH
vargo build --release
```

## 0001-async-fn-unnamed-return-keeps-ensures.patch

**Bug.** An `async fn` without a named return value loses its postconditions
at `.await`. For example, after `set_zero(&mut x).await` with
`ensures *final(x) == 0`, the caller does not learn `x == 0`. Declaring the
return as `-> (ret: ())` avoided it. The bug loses facts (incompleteness); it
does not prove false ones.

**Cause.** Verus states an async function's postconditions as
`awaited(future) ==> ...`, typed by the function's return parameter. With a
named return, the `verus!`/`verus_spec` macro supplies the future's output
type (`get_future_output_type`). Without one, the return parameter keeps the
opaque future type (`impl Future`), so the postconditions are stated at the
wrong type and do not match the facts that `.await` provides.

**Fix.** In `rust_verify/src/rust_to_vir_func.rs` (`finish_function`), when an
async function has no named return, set the return parameter's type to the
`Output` type taken from the opaque return type's `Future` bound (new helper
`async_fn_output_typ`). This makes the unnamed case match the named one.

**Tests.** Four cases added to `rust_verify_test/tests/async_functions.rs`:
an unnamed unit return with `final(x)`, a `&mut self` method awaited in a
loop, an unnamed non-unit return, and a negative case. The three positive
cases fail without the fix and pass with it; the negative case fails as
expected in both.

**Test results.** With the patch applied (Verus `426d8b0`, `vargo test
--release`), every `rust_verify_test` file that uses `async` or tests
return values passes: 302 passed, 0 failed, 1 ignored.

| Test file               | Passed | Failed | Ignored |
|-------------------------|--------|--------|---------|
| `async_functions`       | 15     | 0      | 0       |
| `functions`             | 10     | 0      | 0       |
| `mut_refs`              | 117    | 0      | 0       |
| `mut_refs_old`          | 27     | 0      | 0       |
| `opaque_types`          | 23     | 0      | 0       |
| `regression`            | 83     | 0      | 1       |
| `return`                | 13     | 0      | 0       |
| `returns_postcondition` | 14     | 0      | 0       |

The full `rust_verify_test` suite has not been run.

**Found by.** The async state-unit prototype (`verus-rpc-prototype`,
`two_units_async.rs`): the unit's request loop could not keep its invariant
after awaiting `handle(&mut self, ...)`.
