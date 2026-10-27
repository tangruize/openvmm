# Verus patches

Local fixes to Verus needed by the snapshot specification work. Each patch
applies to Verus commit `426d8b0` ("Support /= and %= for signed integers
(#3013)"), the version that matches the pinned `vstd`
(`=0.0.0-2026-10-04-0306`).

To apply and rebuild (from the Verus checkout), in order:

```sh
git apply /path/to/openvmm/verus-patches/0001-async-fn-unnamed-return-keeps-ensures.patch
git apply /path/to/openvmm/verus-patches/0002-async-fn-ghost-params.patch
git apply /path/to/openvmm/verus-patches/0003-external-trait-own-assoc-bounds-and-hrtb.patch
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

## 0003-external-trait-own-assoc-bounds-and-hrtb.patch

Three bugs found while declaring mesh_protobuf's `Protobuf` trait to
Verus; the first two kept it from being declared, and so kept the contract off `SavedStateBlob::new` / `parse`, whose
bound is `T: SavedStateRoot` (`SavedStateRoot: DescribedProtobuf:
Protobuf`).

**Bug 1.** A trait whose supertrait bound names its own associated type
cannot be declared with `external_trait_specification`. `Protobuf` is
`trait Protobuf: DefaultEncoding<Encoding = <Self as Protobuf>::Encoding>`.
In the proxy trait, naming `<Self as Protobuf>::Encoding` overflows rustc's
trait solver, and naming the proxy's own `<Self as ExProtobuf>::Encoding`
fails Verus's check that the proxy's bounds match the external trait's
exactly ("external_trait_specification trait bound mismatch"). No proxy can
pass.

**Cause 1.** The bound check (`rust_verify/src/rust_to_vir_trait.rs`)
instantiates the proxy's predicates with the external trait's `Self`, but
leaves projections onto the proxy's own associated types as they are, so
`<X as ExProtobuf>::Encoding` never equals `<X as Protobuf>::Encoding`.

**Fix 1.** Before comparing, rewrite projections onto the proxy's
associated types into projections onto the external trait's associated
types of the same name (a `TypeFolder`, `ProxyAssocToExternal`). Verus
already treats the proxy as the external trait everywhere else.

**Bug 2.** Verus panics (inside rustc: "has escaping bound vars, so it
cannot be wrapped in a dummy binder") when it imports an external trait
impl whose associated type is bounded by a higher-ranked trait bound, as
`Protobuf::Encoding: for<'a> MessageDecode<'a, Self, NoResources>`.

**Cause 2.** `translate_assoc_type` (`rust_verify/src/rust_to_vir_impl.rs`)
looks up the impl satisfying each bound with `codegen_select_candidate`,
passing the bound's trait reference with its binder skipped, which leaves
the late-bound region escaping.

**Fix 2.** Instantiate the bound's late-bound regions with erased regions
(`instantiate_bound_regions_with_erased`) instead of skipping the binder;
`codegen_select_candidate` expects erased regions anyway.

**Bug 3.** Verus panics (`vir/src/recursive_types.rs`,
`check_well_founded`) instead of reporting an error when a declared
external trait's bound names a type not declared to Verus (mesh_protobuf's
`NoResources`, before it was declared).

**Cause 3.** The well-foundedness check walks every datatype node of the
type graph, which includes types only referenced in bounds, and
`check_well_founded` panics on a path it has no datatype for.

**Fix 3.** Before checking a datatype node, report an error naming the
type ("type `X` is used but not declared to Verus; declare it, for example
with #[verifier::external_type_specification]") if it is not declared.

**Tests.** Three cases added to `rust_verify_test/tests/external_traits.rs`:
`external_trait_bound_names_own_assoc_type` (a `trait P: Enc<E = <Self as
P>::E>` declared through a proxy, used as a bound),
`external_trait_impl_assoc_type_higher_ranked_bound` (an impl whose
associated type has a `for<'a>` bound, used through a generic function),
and `external_trait_bound_names_undeclared_type` (a bound naming an
undeclared type; expects the error). Without the fixes they fail with the
three errors above (the last one by panicking); with them they pass.

**Test results.** With all three patches: `external_traits` 42 passed,
`traits` 216 (2 ignored), `assoc_type_impls` 45, `traits_dyn` 26,
`external_fn_specification` 78 (3 ignored), `recursive_types` 28; 0
failed. The full suite has not been run.

**Found by.** Attaching the `SavedStateBlob::new` / `parse` contract
(`vm/vmcore/src/save_restore_t.rs`).

