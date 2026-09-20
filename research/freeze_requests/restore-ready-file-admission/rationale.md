# Admit the carried restore-ready file type

## Concrete boundary conflict

The unchanged production `LoadedVm` contains
`restore_ready_sink: Option<std::fs::File>` in
`openvmm/openvmm_core/src/worker/dispatch.rs:840`.
Its TOP postcondition uses `pre_execution_representation`, which reads
`restored_from_snapshot`, `restore_start_guard`, and `running` from the same
struct. The real restore body also reads and writes its fields.

With only the TOP's `external_body` removed, the existing focused command
`make verify MODULE=restore` rejects the production struct:
`std::fs::File is not supported`. This is a frontend rejection, not a failed
restore postcondition. The complete diagnostic is in
`.verus_agent/cache/admission/production-body.log`; its source delta is exactly
the TOP annotation changing from
`#[cfg_attr(verus_keep_ghost, verus_verify(external_body))]` to
`#[cfg_attr(verus_keep_ghost, verus_verify)]`. That diagnostic edit was reverted.

This is not a claim that every other admission error requires new trust.
In particular, the pinned verifier supports `Arc<dyn Trait>` with a verified
trait declaration, as illustrated by its `traits_dyn.rs` tests. Project types
and implementations should be admitted from their real sources, not replaced
by opaque external specifications.

## Why local proof annotations cannot supply this declaration

The pinned frontend translates every field of a transparent struct
(`rust_to_vir_adts.rs`, `check_variant_data`). It does not omit an unused field
because a selected function does not access it. `File` belongs to the compiled
Rust standard library, not project-owned source, and the pinned vstd has no
declaration for it.

Making the containing struct `external_body` instead removes its fields from
the verifier. Field projections then fail with
`disallowed: field expression for an opaque datatype`. That cannot preserve the
TOP's concrete pre-execution representation or admit the production body.
Trying a transparent `external_type_specification` for `File` instead fails
with `private fields not supported for transparent datatypes`. Genericizing or
removing the file field would change frozen executable types; an opaque
accessor assumption would merely move the trust.

These alternatives are isolated by this standalone frontend probe, not by a
replacement restore implementation:

```rust
use std::fs::File;
use vstd::prelude::*;
verus! {
#[cfg(file_declaration)]
#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExFile(File);
#[cfg(transparent_file_declaration)]
#[verifier::external_type_specification]
pub struct ExFile(File);
#[cfg_attr(opaque_container, verifier::external_body)]
struct Carrier {
    ready_sink: Option<File>,
    running: bool,
}
fn stop(carrier: &mut Carrier)
    ensures !final(carrier).running,
{
    carrier.running = false;
}
fn main() {}
}
```

The ready-to-run source is
`.verus_agent/cache/admission/std_file_admission.rs`. From the repository root,
run the following separately for `baseline`, `opaque_container`,
`transparent_file_declaration`, and `file_declaration`:

```bash
VERUS_Z3_PATH="$PWD/toolchain/verus-src/source/z3" \
  toolchain/verus-src/source/target-verus/release/verus \
  .verus_agent/cache/admission/std_file_admission.rs \
  --cfg baseline --multiple-errors 2 --num-threads 1 --triggers-mode silent
```

The opaque file declaration permits the probe's ordinary field-assignment
obligation; the other configurations produce the diagnostics above. This
isolates the type-admission obstruction only. It does not establish any
production restore obligation.

## Smallest proposed frozen change

Add one opaque `std::fs::File` type declaration, `ExRestoreReadyFile`, in the
existing restore proof module, with exact symbol-and-marker TCB entries for
`external_type_specification` and `external_body`. No file operation, function
contract, file contents, OS effect, layout, or restore fact is assumed. The
type's private standard-library representation is the sole new opaque leaf.
No project-owned state is made opaque.

`freeze.patch` also records the current, unchanged scope manifest because the
frozen branch lacks the manifests that are already present on the working
branch. It does not change the TOP, its vocabulary, scope, or frozen branch.
It retains the existing `InitializedVm::load` sanction. `run.patch` makes the
same declaration and TCB change against the working branch. The type is
required even with the TOP's body-exclusion annotation retained: the original
focused frontend also rejects `File` in the transparent `LoadedVm` declaration.
Neither patch changes executable behavior or removes existing proof debt.

This request resolves one necessary admission boundary, not the whole
frontend. `HvlitePartition`, `LoadedVmInner`, `SavedState`, `StateUnits`,
`StopGuard`, `anyhow::Context`, and other carried types still need admission
work. The four representation bridges and the TOP body proof remain owed.
Do not treat this proposal, its reduced probe, or patch consistency checks as
verification of `LoadedVm::restore_snapshot_state`.
