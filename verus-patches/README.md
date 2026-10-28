# Verus patches (and one OpenVMM patch)

This directory holds two kinds of patch:

| Patch | Applies to | What |
|---|---|---|
| `0001`–`0005` (`*.patch` without a prefix) | **Verus**, commit `426d8b0` | local fixes to Verus needed by the snapshot specification work |
| `openvmm-0001-virtio-save-keeps-captured-device-state.patch` | **OpenVMM `origin/main`** (`7a2b21747`) | a change to OpenVMM's own Rust code, for OpenVMM developers to review; this branch already contains it (see "OpenVMM patches" below) |

Local fixes to Verus needed by the snapshot specification work. Each patch
applies to Verus commit `426d8b0` ("Support /= and %= for signed integers
(#3013)"), the version that matches the pinned `vstd`
(`=0.0.0-2026-10-04-0306`).

To apply and rebuild (from the Verus checkout), in order:

```sh
git apply /path/to/openvmm/verus-patches/0001-async-fn-unnamed-return-keeps-ensures.patch
git apply /path/to/openvmm/verus-patches/0002-async-fn-ghost-params.patch
git apply /path/to/openvmm/verus-patches/0003-external-trait-own-assoc-bounds-and-hrtb.patch
git apply /path/to/openvmm/verus-patches/0004-async-fn-in-trait.patch
git apply /path/to/openvmm/verus-patches/0005-trait-method-ghost-params-and-async-defaults.patch
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


## 0004-async-fn-in-trait.patch

Lets a trait declare an `async fn` with a contract, as `StateUnit` does
(`vmm_core/state_unit/src/lib.rs`): its seven methods are all `async fn`,
so no unit's `save` / `restore` could carry a contract.

**Bug.** Verus rejects a trait (its own, or one declared with
`external_trait_specification`) with an `async fn` method: "Verus does not
yet support Opaque types in trait def" (`check_no_opaque_types_in_trait`,
`rust_verify/src/verifier.rs`).

**Cause.** rustc gives each `async fn` in a trait (and each `-> impl
Trait` method) a hidden associated type, an RPITIT ("return-position impl
Trait in trait") type: the trait declares it, bounded by `Future<Output =
...>`, and each impl defines it as its method's opaque future type. The
method returns a projection onto it. Verus did not translate these
associated types: they have no name, are not items in HIR, and the method's
return type is a projection, not the opaque type Verus expects of an
`async fn`. So the check refused them.

**Fix.** Translate RPITIT associated types like other associated types:

- Name them `VERUS_RPITIT__<method>` (`rust_to_vir_base::assoc_item_name`,
  used wherever an associated item's name was taken), the same in the trait
  and in each impl.
- The trait declares them with their bounds (`rust_to_vir_trait.rs`); each
  impl defines them as its method's opaque type (`rust_to_vir_impl.rs`).
  The impl gets no impl paths for the bounds: the compiler-generated future
  meets them, and resolving them would reveal the opaque type before
  Verus's erasure is set up (it panicked: "VerusErasureCtxt has not been
  initialized").
- An async method's return type may be the RPITIT projection
  (`async_fn_output_typ`, `rust_to_vir_func.rs`); the method and its
  specification stub are compared by their futures' output types.
- The opaque types' declarations go before the associated-type impls in the
  solver input (`verifier.rs`), which now refer to them.
- The trait-conflict check leaves RPITIT types out (`trait_conflicts.rs`):
  it cannot express opaque types, and they cannot make impls conflict.
- The check now refuses only non-async methods returning `impl Trait`,
  which are untested (`verifier.rs`).

**Tests.** Six cases in `rust_verify_test/tests/async_functions.rs`, on a
trait `Counter` with a `spec fn value` and an `async fn bump` with
`requires` / `ensures`: a verified impl with a caller on the impl's type
(`async_fn_in_trait_concrete_caller`) and a generic caller
(`async_fn_in_trait_generic_caller`) pass; an impl that breaks the
`ensures`, a false assertion after the call (concrete and generic), and a
call that breaks the `requires` each fail as expected. Without the patch
Verus refuses each trait with the error above, before verifying anything
(not rerun without the patch).

Also checked on `StateUnit` itself: declared with
`external_trait_specification` and an `external_trait_extension` spec
function, a contract on `save` holds for a generic caller and is enforced
on a verified impl (a probe, not committed).

**Test results.** With all four patches: `async_functions` 21 passed,
`opaque_types` 23, `opaque_reveal` 24 (4 ignored), `adts_opaque` 10,
`traits` 216 (2 ignored), `external_traits` 42, `assoc_type_impls` 45,
`traits_dyn` 26, `external_fn_specification` 78 (3 ignored),
`recursive_types` 28; 0 failed. The full suite has not been run.

**Not covered.** Non-async `-> impl Trait` trait methods (still refused);
`async fn` in a trait used through `dyn` (not object safe anyway).

## 0005-trait-method-ghost-params-and-async-defaults.patch

Lets a trait method take ghost inputs (`#[verus_spec(with Tracked(..))]`),
including an `async fn` and a default method, so that `StateUnit`'s
contract can pass the FracGhost halves of the request protocol through its
methods (see "Production units: the contract on `StateUnit`" in
`SNAPSHOT-SPEC-DESIGN.md`). Three fixes.

**Bug 1.** `#[verus_spec(with ...)]` on a trait method declaration is
refused: "`with` does not support trait" (`builtin_macros/src/attr_rewrite.rs`).

**Cause 1.** For a function, `with` makes the macro emit two functions:
the original, for unverified callers, with its body replaced by `requires
false; unimplemented!()` in verification builds; and a verified copy,
`_VERUS_VERIFIED_<name>`, with the ghost inputs and the specification,
which `proof_with!` calls. A trait method needs both copies as trait
methods, and each impl both as impl methods; the macro did neither for
traits, and an impl's original copy got `requires false`, which a trait
impl method cannot declare.

**Fix 1.** A trait method declared with `with` becomes two trait methods:
the original, unchanged and without specification, and the verified copy
with the ghost inputs and the specification (`split_trait_method_with`;
also in erase mode, since impls emit both copies there too). The methods of
a trait impl under `#[verus_verify]` are marked as such
(`verus_trait_impl_method_marker`), and their original copy gets no
`requires false` (verified callers call the verified copy anyway). An impl
must provide the verified copy, so an impl without `with` does not compile
in verification builds: every impl is checked.

**Bug 2.** An `async fn` with a default body in a trait (patch 0004 covered
only methods without one): an internal error ("ill-typed AIR code: use of
undeclared function TYPE%...opaque0"), and, with that fixed, generic callers
did not get the method's `ensures`.

**Cause 2.** A default method returns the projection onto its RPITIT type
(0004); the default body's future is that type's default, an opaque type
that Verus never declared. And inside the default method, rustc normalizes
the projection to that opaque type, so the method's contract was stated on
the opaque type, while a generic caller's future has the projection type.

**Fix 2.** Declare the default body's opaque type (`check_fn_opaque_ty`);
do not normalize an RPITIT projection to a trait default body's opaque type
(`mid_ty_to_vir`), so the contract is stated on the projection; accept the
projection as an async function's return type; and for an impl that does
not override the method, define its RPITIT type as the default body's
opaque type for the impl (`rust_to_vir_impl.rs`), as rustc does, so that
callers on the impl's type, whose future rustc gives the opaque type, get
the contract too.

**Bug 3.** In a `#[verus_verify]` trait impl, a method whose specification
is written `#[cfg_attr(verus_keep_ghost, verus_spec(...))]` (as OpenVMM
writes every specification, so that normal builds do not see it) fails:
"Multiple #[verus_spec] attributes are not allowed".

**Cause 3.** `#[verus_verify]` on an impl adds an empty `#[verus_spec]` to
each method that has none (`add_verus_spec_if_needed`), and it runs before
the method's `cfg_attr` is expanded, so it does not see that one.

**Fix 3.** Count a `cfg_attr` that contains `verus_spec` as a
specification.

**Tests.** In `rust_verify_test/tests/syntax_attr.rs`:
`test_trait_impl_cfg_attr_verus_spec` (two impls of a trait method with a
ghost input, both written with `cfg_attr`: one verifies, the other breaks
the `ensures` and fails); `test_trait_method_with_ghost_params` (a trait method with a tracked input,
an impl, a generic caller, and an unverified caller of the original),
`test_async_trait_method_with_ghost_params` (the same for `async fn`, and an
async default method with a ghost input), `test_trait_signature` (replaced:
it expected the old error; now ghost inputs and a ghost output on a trait
method, an impl, and a generic caller that receives the output), and three
that must fail and do (an impl breaking the `ensures`, a caller breaking
the `requires`, a false assertion after the call). In
`rust_verify_test/tests/async_functions.rs`, async default methods: a
generic caller and a caller on a non-overriding impl, an overriding impl,
and three that must fail and do (a default body breaking its `ensures`, a
false assertion after the call, generic and concrete).

**Test results.** With all five patches: `syntax_attr` 86 passed,
`async_functions` 26, `traits` 216 (2 ignored), `external_traits` 42,
`assoc_type_impls` 45, `traits_dyn` 26, `external_fn_specification` 78 (3
ignored), `recursive_types` 28, `opaque_types` 23, `opaque_reveal` 24 (4
ignored), `adts_opaque` 10, `functions` 10, `mut_refs` 117, `mut_refs_old`
27, `regression` 83 (1 ignored), `return` 13, `returns_postcondition` 14; 0
failed. The full suite has not been run. All OpenVMM spec crates verify
(`scripts/verify-snapshot-spec.sh`).

**Not covered.** Ghost outputs (`-> out`) of an async trait method (as for
any `async fn`, see 0002); `with` on methods of traits declared with
`external_trait_specification`.

**OpenVMM's build.** Fixes 1 and 3 are in Verus's macros
(`builtin_macros`), which OpenVMM takes from crates.io
(`verus_builtin_macros 0.0.0-2026-09-20-0158`, pinned by `vstd`), not from
the local checkout. So the repo carries a copy of that published crate
with the fix applied, `verus-patches/verus_builtin_macros`, used through
`[patch.crates-io]` in the workspace `Cargo.toml`; its diff from the
published crate is `0005-verus_builtin_macros-published.patch` (the same
change as this patch's `attr_rewrite.rs` hunks, at other line offsets).
Fix 2 is in the verifier and comes from the local checkout, as
for the other patches.

## OpenVMM patches

These change OpenVMM's executable Rust code (not Verus). This branch
already contains each change (as a code change in
`SNAPSHOT-SPEC-CODE-CHANGES.md`); the patch here is the same change against
`origin/main`, to show OpenVMM developers. Apply from an OpenVMM checkout of
`origin/main` with `git apply`.

### openvmm-0001-virtio-save-keeps-captured-device-state.patch

**Behavior before.** A virtio device's private state lives in its device
task. When the device stops, the transport captures that state; `save` then
*takes* it out (`take_device_state`, `captured_device_state.take()`). So
`save` changes the transport, and a second `save` before the next `start`
fails with "device state was not captured". (Added by commit `c2f1ed7c9` on
`main`; the commit does not say why the state is taken rather than copied.)

**Why change it.** The snapshot specification states, for every
`SaveRestore` device, that `save` does not change the device and that what
it returns is determined by the device's state. With `take`, the virtio
transports (MMIO and PCI) cannot meet that.

**Fix.** `save` copies the captured state (`saved_device_state`, which
clones the `SavedStateBlob`) and keeps the capture until the next `start`
or reset, which clear it as before. A failed capture is still returned, and
dropped, by the first `save`.

**Effect.** Same result for the first `save` after a `stop`. A second
`save` before the next `start` now returns the same state instead of
failing. `save` without a preceding `stop` still fails. Cost: one copy of
the device's private-state blob per `save`, and the capture is kept in
memory until the next `start`.

**Files.** `vm/devices/virtio/virtio/src/transport/core.rs` (the method,
renamed from `take_device_state`), `mmio.rs` and `pci.rs` (their `save`
calls it). Checked on `main`: builds, rustfmt clean, `cargo test -p virtio`
140 passed. On this branch the same change is in
`transport/core/restore.rs` (the file was later split; the method is
`captured_device_state` there).

