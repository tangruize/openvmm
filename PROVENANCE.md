# Provenance and design decisions

## Current source

The production checkout was cloned from `nanvix/openvmm` `main` after resolving the remote head on October 15, 2026. The exact baseline is `7f225e2cd635bbd1e1839e587e659996433396c8`; task changes are committed through `f127b8c455cabb04125eb582772c6eea326b5cc6` only on the local `system-proof/snapshot-save-restore-seed` branch.

The Git remote was removed after the baseline revision was fetched. This prevents the task from silently fetching or switching to another source revision.

The earlier archived revision `70dde4c65b16de084744b0fa85aeb581f0b5c6cd` was used only to recognize the useful begin/apply/finish restore structure. It is not the task code baseline.

## Restore abstraction reused

The retained restore design came from:

- `/home/ruize/openvmm/human-howto-spec.txt`
- `/home/ruize/openvmm/verification/restore/SPEC.md`
- `/home/ruize/openvmm/verification/restore/COMPONENTS.md`

Those materials define the previously discussed boundary: the successful `InitializedVm::load(Some(_))` return after state restoration and time adjustment, with VPs stopped and before guest execution; `SavedState` is not the complete snapshot; prepared RAM, stable VP identity, deferred device state, external resources and clock samples remain explicit.

The relevant abstraction is now hosted once in the common dependency `openvmm_defs/src/snapshot.spec.rs`; the adjacent core `dispatch.spec.rs` re-exports the same nominal model. Historical files are not copied into the production checkout and their prior proof status does not transfer.

## Save/restore synthesis reused

The direct 1008 `snapshot_correspondence.spec.rs` supplied the useful shared capture/restore shape. The current seed expresses it as relations rather than equality with a projected whole VM state:

```text
published_snapshot.captures(source_at_capture_cut, capture_boundary)
published_snapshot.restores_into(environment, restored_at_return)
```

The component model distinguishes complete runtime state from serializable snapshot state. Components are keyed by their state-unit names with separate active and pending maps, while the inventory retains exact registration order. Capture records `ComponentSnapshotStateView`; restore overlays it onto freshly initialized `ComponentStateView`, retaining destination runtime context. The contracts bind that ordered inventory to the concrete `SavedState.inventory` and `SnapshotSaveResponse.state_unit_names` fields and isolate protobuf encoding in a narrow production helper. Component payload semantics remain an explicit grounding obligation. The October 12 model was inspected but not copied because its large abstract TOP was not consumed by the actual entry points.

## System Proof guidance applied

The task follows the current guidance in:

- `vertical/argus_system_proof/skills/shared/top-level-spec-grounding.md`
- `vertical/argus_system_proof/skills/shared/specification-judgment.md`
- `vertical/argus_system_proof/skills/engineer/specification-and-view.md`
- `vertical/argus_system_proof/knowledge/verus/spec-design.md`
- `vertical/argus_system_proof/knowledge/verus/view-design.md`

The important consequence is that the seed model is not the theorem by itself. The real save controller, artifact preparation, restore worker constructor, `load`, `save`, `restore`, `restore_from_snapshot`, and `quiesce_for_snapshot` declarations already consume its relations, initially through `external_body` markers. Completion requires removing or narrowly justifying those markers, grounding the representation functions, closing the dependency path from publication through `InitializedVm::load(Some(_))`, covering success/failure/frame behavior, and passing independent Audit.

## Production refactors

The latest source already had `begin_snapshot_restore` and `finish_snapshot_restore`, but the real `load(Some(_))` branch still assembled the three-step sequence inline. The task adds `LoadedVm::restore_from_snapshot` and makes `InitializedVm::load(Some(_))` call it. No order, error context or success boundary changed.

The restore-worker trait implementation now forwards to an inherent `VmWorker::new(VmWorkerParameters)` containing the original construction body. This behavior-preserving extraction allows the real restore worker TOP to carry a direct function contract despite the current `verus_spec` limitation on a static trait-impl method.

This is intentionally narrower than the old September proof branch, which mixed many unrelated helper extractions and proof instrumentation into `dispatch.rs`.

## Validation and status

- `cargo fmt --all -- --check`: passed.
- `RUSTUP_TOOLCHAIN=1.98.1 cargo check -p openvmm_defs -p openvmm_entry -p openvmm_core --quiet`: passed after restoring the repository-managed packages.
- Official stable `cargo-verus focus -p openvmm_defs` verified the shared model and both composition lemmas with `2 verified, 0 errors`.
- Official stable `cargo-verus focus -p openvmm_entry` parsed the save and preparation TOP contracts, then hit the existing `vir/src/prune.rs:464:46` `no entry found for key` panic before solver obligations.
- Official stable `cargo-verus focus -p openvmm_core` parsed the worker, load, save, and restore contracts, then hit the same existing prune panic before solver obligations.
- Validation caches (`target`, `.packages`, `flowey-persist`, and `flowey-out`) were removed after validation.

No container, service, Argus project, verification worktree or campaign has been launched. No commit has been pushed.
