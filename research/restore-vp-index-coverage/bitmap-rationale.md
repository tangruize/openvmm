# Native Boolean update for the restore-index bitmap

The proposed frozen change is confined to
`vmm_core/src/partition_unit/vp_set.rs`: replace
`std::mem::replace(slot, true)` with a private
`mark_restore_vp_present(slot)` whose body copies the Boolean, assigns true
to that same reference, and returns the saved Boolean. This is a
behavior-preserving source normalization, not a correction to the validator's
runtime behavior.

## Concrete frozen boundary

`research/restore-vp-index-coverage/native-accessor-consumer.log` records
the real validator's unsupported `core::mem::replace` call after the native
VpIndex representation and accessor were supplied. The source evidence in
`native-core-source.log` follows the library implementation to
`read_via_copy` and `write_via_move`; `probe-intrinsics.log` records their
bodyless compiler declarations. The smaller `probe-raw-intrinsics.log`
isolates rejection of both intrinsic calls without the initial reference
coercion problem. This established rejection is reused here.

Annotating the library source cannot prove bodyless compiler-intrinsic
leaves. Adding their suggested `assume_specification` interfaces would add
trust, and changing the verifier is outside the frozen boundary. The
existing swap control shows a different available library interface but
neither proves the validator nor obliges this proposal to use a trusted
library operation. The proposed Boolean-specific helper needs neither
library operation nor intrinsic contract.

## Exact operation and caller behavior

The candidate helper has no precondition and the postcondition
`previous == *old(slot), *final(slot)`. Its one real body reads `*slot`,
unconditionally writes true, and returns the saved value. The ordinary
exclusive-reference rules limit the mutation to that slot. For initial
false it returns false and leaves true; for initial true it returns true
and leaves true. The second case still performs the assignment before the
caller enters its duplicate-error branch. There is no external body,
assumption, admission, or alternative verification-only implementation.

Factoring these three native operations into a private helper makes exactly
the operation needed by this caller independently verifiable while the
validator's unrelated declarations remain unresolved. An inline spelling
would save a function boundary but would leave this operation's body proof
blocked behind those declarations. No general bitmap abstraction or new
caller requirement is introduced.

The checked `get_mut` and unknown-index error still precede this operation.
The helper takes the exact reference previously passed to `replace`; it
cannot inspect or change the index stream or any other slot. The original
`if` now tests the same saved value returned by the helper. Both error
variants and their formatting expressions, all early returns, iterator
consumption order, and the final `position` scan are textually unchanged.
For a Boolean the original replacement and the proposed copy/assignment
invoke no user-defined clone, destructor, or conversion, and neither
operation can panic for a valid mutable reference.

Consequently an unknown index still stops before mutation, a duplicate
still stops after setting its slot, and exhaustion still uses the same
first-missing-index scan. Empty inventories, unsorted valid inventories,
and errors in dormant entries retain the existing behavior. This is an
equivalence argument about the changed operation, not a Verus exact-coverage
theorem for arbitrary iterators.

## Independent patches and production evidence

`freeze.patch` contains only the helper and its call-site substitution.
`run.patch` independently contains that same executable normalization,
native annotations for the helper's body contract, and the already-retained
VpIndex View/accessor work and its crate metadata/dependency. The latter is
not a changed theorem: the existing accessor still returns its private
scalar View without a precondition or representation assumption. Including
it makes the candidate self-contained relative to the working branch tip
without discarding the uncommitted production proof.

The isolated archive uses the actual workspace and production validator.
`research/restore-vp-index-coverage/bitmap-candidate-vp_set.rs` retains its
exact proposed source. The preparation and focused commands are
`prepare-bitmap-candidate.command`, `check-bitmap-helper.command`, and
`check-bitmap-validator.command`. The helper is selected from its real
production module with lifetime checking enabled; `bitmap-helper.log`
records its body proof, not a standalone copy.

`bitmap-validator-admission.patch` is an experiment-only annotation overlay:
it exposes the existing validator and enables the same macro feature used
by the earlier consumer probe. `bitmap-validator.log` records the actual
candidate validator reaching six declaration/library failures instead of
the original seven. The replacement helper is declared and has a proved
contract; neither `mem::replace` nor its intrinsic boundary is reported.
The overlay is removed after that measurement and is not part of the
request, since the validator body does not yet verify.

The remaining six failures are `RestoreError`, slice-iterator `position`,
`anyhow::Error`, `Error::msg`, `__private::must_use`, and
`__private::format_err`. No interfaces for them are added. No validator
verification conditions are claimed discharged. The generic IntoIterator
input still lacks a proved binding to the consumed contents; exhaustion,
bitmap invariants and successful exact coverage remain proof work.

`check-bitmap-rust.command` checks the exact candidate with ordinary Rust,
then applies only `bitmap-behavior.patch` for regression tests on the real
functions. The tests cover both initial Boolean values and preservation of
other slots, exact duplicate/unknown/missing error values and text, early
iteration termination, empty inventories, unsorted input and the first
missing slot. Existing selector and restore-index tests run alongside them.
The ordinary compiler and test outputs are `bitmap-rust.log` and
`bitmap-tests.log`. The test overlay does not alter the validator algorithm
and is removed afterward.

## Scope and approval boundary

The real `select_instantiated_vp_states` supplies its mapped slice iterator
to the validator before filtering. `VpSet::restore` uses the selected pairs
to send their payloads to `StateEvent::Restore`. The retained LSP report
`callers.log` and the production call sites establish this connection, not
a proof of it. This operation supports the future exact-coverage
intermediate needed on the route to `restore_vp_projection`.

Exact coverage, prefix selection, payload installation and the TOP theorem
remain unproved. The TOP body marker, four uninterpreted representation
bridges, and existing project declaration debt are untouched. There is no
new temporary marker, sanctioned declaration, rlimit annotation, manifest
change, or verifier change. The current restore admission and both earlier
normalization packages are preserved.

Nothing is installed on either authoritative branch. This package requests
assessment and explicit Human approval of the executable-source
normalization; package validation or candidate proof is not authorization
to apply either patch. The existing worker-only boundary scan and its
TCB-provenance limitation are not a whole-workspace trust certificate.
