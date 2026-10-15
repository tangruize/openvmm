# OpenVMM snapshot save/restore proof input

## Source

The production source is at `/workspace/task/production`. It contains the `nanvix/openvmm` implementation and a supplied initial unified snapshot save/restore specification attached to the real lifecycle functions.

Use this checkout rather than fetching, switching to another revision, or replacing it with another source tree.

## Target

Prove the real snapshot lifecycle for the supported Linux-host, x86_64-guest microVM configuration:

- `VmController::handle_guest_snapshot_request` for snapshot capture and publication;
- `prepare_snapshot_restore_for_config` for validation and preparation of the published artifacts;
- the `parameters.saved_state.is_some()` branch of `VmWorker::new` for worker construction and restore;
- `InitializedVm::load` with `saved_state = Some(_)` as the internal restore composition boundary.

The source contains a supplied initial top-level formalization for this lifecycle:

- `openvmm/openvmm_defs/src/snapshot.spec.rs` defines the shared `LogicalVmState`, capture relation, allowed restore transform, save/prepare/worker TOP relations, and their composition;
- `openvmm/openvmm_core/src/worker/dispatch.spec.rs` re-exports that common nominal model, and the adjacent core and entry `.proof.rs` files contain supporting specification definitions;
- all four functions in `targets.txt`, together with `LoadedVm::save`, `LoadedVm::restore`, `LoadedVm::restore_from_snapshot`, and `LoadedVm::quiesce_for_snapshot`, carry supplied `verus_spec` contracts and begin as `external_body`.

Reuse the supplied formalization as the starting point. It may be corrected, refined, or extended when source grounding requires, but do not weaken the intended lifecycle or replace it with a separate model theorem.

The central intended claim is the following projection correspondence:

```text
restored_projection(returned_vm)
    == allowed_restore_transform(
        captured_projection(source_vm_at_capture_cut),
        environment,
    )
```

The supplied initial specification decomposes that end-to-end claim into a save-side capture relation and a restore-side relation over the same published snapshot:

```text
published_snapshot.captures(source_vm_at_capture_cut, capture_boundary)
published_snapshot.restores_into(environment, returned_vm)
```

Equivalence is required only for snapshot-relevant state under the allowed restore transforms; it must not be strengthened to equality of complete runtime objects or destination-local operational context. `SavedState` must not be assumed to be the complete snapshot: determine from the production lifecycle which state and artifacts participate in capture and restore, and repair the initial decomposition if source grounding requires it.

The restored projection is observed at the successful `InitializedVm::load(Some(_))` return before guest execution.

No guarantee is required from `InitializedVm::load(None)`.

## Verifier limitations

Treat verifier limitations as repairable engineering problems rather than reasons to weaken the goal. Prefer small behavior-preserving source rewrites when they retain the intended semantics and do not change the architecture or public behavior. When a bounded, well-understood Verus defect or compiler crash blocks the real target, repair and rebuild Verus with focused regression coverage when feasible; do not bypass the affected code or weaken the specification merely to obtain a passing result. Escalate before broad or architectural source or compiler changes.

## Result discipline

Keep the final top-level specification recognizably about the supplied snapshot capture, publication, preparation, and restore lifecycle. Necessary formalization repairs are allowed, but replacing it with a materially different, substantially more complex, or easier objective is not completion.

The final report must state whether the supplied TOP was proved as-is, repaired while preserving the same lifecycle, only partially proved, or found unresolved or invalid.
