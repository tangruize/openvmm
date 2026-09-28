# Snapshot publication and restore access meaning

This file records the shared snapshot View for the current `snapshot.publish` investigation. It separates two different kinds of evidence: restore-open model feedback that helped shape the View, and source-bound publication fragments for the same save path. Run receipts and exact evidence paths belong in `review.md` and the graph, not in this shared input file.

## Restore-open model feedback

The restore-side proof entrypoint is `proofs/snapshot.rs`, function `opened_snapshot_open_view`. It keeps the useful restore-access meaning: a successful open should expose retained directory, manifest, state, and memory handles; state length agrees with the manifest; the observed memory generation length agrees with the manifest; and intended-capture membership is only propagated from a publication or caller proof.

That result is not source-body feedback for `OpenedSnapshot::open`. The body comparison shows the proof body begins with Boolean inputs such as `input.directory_opened`, while the production body begins by executing `OpenedSnapshotDirectory::open(dir)`. The registered candidate `snapshot-opened-generation-v1` is therefore classified as a model. Its restore-access meaning remains useful specification feedback, but it does not discharge the body obligation for `openvmm/openvmm_helpers/src/snapshot/restore.rs`.

The restore-side representation dependencies remain open:

- `OpenedSnapshotDirectory` and relative artifact opens must be shown to retain one directory generation and regular opened artifact handles on each supported platform.
- The filesystem must maintain the ordinary meaning of opened file handles and file metadata across the checked interval.
- A publication or caller proof must establish that the structurally opened generation belongs to the intended capture. Versions 3 through 5 intentionally do not authenticate same-length mutations of `state.bin` or `memory.bin`.
- Composition through `prepare_snapshot_restore_for_config`, `duplicate_memory_file_for_mapping`, resume claims, copy-on-write memory mapping, and worker lifetime guards remains later work.

## Publication fragment feedback

The intended publication body is `write_snapshot_with_memory_publication` in `openvmm/openvmm_helpers/src/snapshot/publish.rs`. Its source control flow distinguishes staging failure, publication failure with rollback, committed publication followed by parent-sync failure, and successful return. A Verus harness preserving that body was attempted and retained in `research/system-proof/candidates/publication-orchestrator-limitation.txt`, but the configured frontend rejected the source-shaped `and_then(|()| staging.publish(dir))` closure because it mutably captures `staging`.

The non-model publication candidate is therefore narrowed to the same-path callee `StagingDirectory::publish`. The binding file is `research/system-proof/candidates/staging-publish-binding.json`. The correspondence check matches the executable fragment between the production method body and the proof method body, while leaving type, callee, macro, formatting/context conversion, and filesystem/resource semantics explicit.

The proof-side View for this fragment is `staging_commit_view`, supported by the explicit `rename_no_replace_view` boundary in `proofs/snapshot.rs`. It says that `StagingDirectory::publish` reports success exactly when the no-replace rename is treated as committed and the staging owner retires its path before returning `Ok(())`. The proof-side `Context::with_context` interface preserves the `Ok`/`Err` shape of the rename result, and the publish postcondition states that an error return leaves the staging owner path unchanged. The filesystem state and races that decide whether `rename_no_replace` succeeds are not parameters of `StagingDirectory::publish`, so two calls with the same `Path` values need not have the same result. This is an intended environment boundary unless a later candidate introduces an explicit filesystem/resource state interface.

The production rename helper also leaves a platform dependency visible. In `openvmm/openvmm_helpers/src/snapshot/fs.rs`, the Linux-GNU implementation checks same-parent names and calls `renameat2` with `RENAME_NOREPLACE`; the fallback implementation delegates to `fs_err::rename`. The current fragment therefore supports the source-shaped publication body and the Linux-GNU no-replace intent, but it does not prove portable no-replace semantics, substituted type behavior, or filesystem resource effects. This is a useful publication boundary, but it is not a proof of the full orchestrator. The caller-level facts below remain open:

- `stage_snapshot` must establish a complete staging generation, including manifest rewriting, state bytes, memory publication policy, optional scratch verification, and staging-directory sync.
- `write_snapshot_with_memory_publication` must map staging errors through rollback to `BeforeCommit` or `CleanupUncertain`.
- A failed `ensure_path_absent` or `StagingDirectory::publish` must leave the final destination uncommitted under the relevant platform filesystem no-replace semantics.
- A failed parent-directory sync after publication must be classified as `SnapshotWriteError::Committed`, not absence of the snapshot.
- Full success must connect the committed generation to the caller’s intended capture only under stopped-source, exact-memory-handle, source-file quiescence, and access-control assumptions.

## Rollback fragment feedback

The rollback candidate is `snapshot-staging-rollback-v1`, bound to `StagingDirectory::rollback` in `openvmm/openvmm_helpers/src/snapshot/publish.rs` and `proofs/snapshot.rs`. The correspondence check reports an exact executable fragment and parameter match. The preserved body keeps the production distinction: when `source_memory_alias` is true and `remove_staging_directory` fails, rollback returns `SnapshotWriteError::CleanupUncertain`; otherwise it returns `SnapshotWriteError::BeforeCommit`, warning but not changing the classification for independent cleanup failure. The proof-side `rollback_classification_view` records the limited consequence needed by the publication path: both rollback outcomes are pre-commit and do not mean the final destination was published.

This fragment is not yet verified as a source-body proof. The exact production-shaped method needs Rust 2024 let-chain parsing and a `mut self` receiver, and those currently stop the configured frontend/checker before the rollback classification can be proved. The cleanup callee, tracing macro, `PathBuf` clone/expect behavior, private-method path invariant, and filesystem removal/sync effects remain explicit dependencies rather than hidden assumptions.
