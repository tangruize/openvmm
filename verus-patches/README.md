# Verus patches

Local fixes to Verus needed by the snapshot specification work. Each patch
applies to Verus commit `426d8b0` ("Support /= and %= for signed integers
(#3013)"), the version that matches the pinned `vstd`
(`=0.0.0-2026-10-04-0306`).

To apply and rebuild (from the Verus checkout), in order:

```sh
git apply /path/to/openvmm/verus-patches/0001-async-fn-unnamed-return-keeps-ensures.patch
git apply /path/to/openvmm/verus-patches/0002-async-fn-ghost-params.patch
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

## 0002-async-fn-ghost-params.patch

**Bug.** Annotation-style ghost parameters (`#[verus_spec(with Tracked(x):
..., Ghost(y): ...)]`) on an `async fn` failed with "verus internal error:
ill-formed unwrap_parameter header", for any ghost or tracked parameter.

**Cause.** The `verus_spec` macro turns `with Tracked(x)` into a parameter
`verus_tmp_x` and a header in the body that unwraps it into `x`. Verus finds
the parameter by the `HirId` of the variable the header reads
(`rust_verify/src/pre_header.rs`, `unwrap_param_extract_idents`). In an
`async fn`, rustc moves each parameter into the coroutine with a new binding
(`let verus_tmp_x = verus_tmp_x;`), so the header reads that binding, whose
`HirId` is not the parameter's, and the lookup failed.

**Fix.** If no parameter has that `HirId`, look the parameter up by name
(the moved binding has the parameter's name). Simple bindings only.

**Tests.** Two cases added to `rust_verify_test/tests/syntax_attr.rs`:
`test_async_fn_with_ghost_params` (a tracked `&mut` and a ghost input to an
`async fn`, passed with `proof_with!` and awaited later; the callee's
postcondition about them holds after the `.await`) and
`test_async_fn_with_ghost_params_fail` (a wrong ghost argument fails the
precondition). Both fail without the fix and pass with it.

**Test results.** With both patches: `syntax_attr` 80 passed; and, as for
0001, `async_functions` 15, `functions` 10, `mut_refs` 117, `mut_refs_old`
27, `opaque_types` 23, `regression` 83 (1 ignored), `return` 13,
`returns_postcondition` 14; 0 failed. The full suite has not been run.

**Not fixed.** Ghost *outputs* (`with ... -> out: Tracked<T>`) of an `async
fn` still do not work at call sites: the macro destructures the call's
result as a tuple, but for an `async fn` that is the future, not its
output. Tracked `&mut` inputs cover the need.

**Found by.** Verifying `StateUnits::run_op` in annotation style: its
helpers are `async fn`s that need ghost parameters.

