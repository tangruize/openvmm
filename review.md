# Snapshot publication and restore-side open specification

Checkpoint before the Tianyu-method migration: the bounded native supervisor completed two reviewed partial investigations and ended its third mission with a replan request. The current shared proof unit is blocked; the earlier successful publication-fragment result is historical, not a current proof of this aggregate. No API or module is declared complete.

Production behavior is unchanged. The current proof work is in `proofs/snapshot.rs`, `proofs/specdet.toml`, `specs/snapshot/meaning.md`, `research/system-proof/candidates/`, `research/system-proof/nodes/opened-snapshot-open.json`, and `research/system-proof/nodes/staging-rollback.json`.

## Corrected restore-open classification

The previous `OpenedSnapshot::open` result is retained as model and specification feedback, not as source-body feedback. The current binding comparison is `.system-proof/evidence/restore-body-connection.json`; it reports `model_or_transformed` because the proof body starts from Boolean inputs such as `input.directory_opened`, while the production body starts by executing `OpenedSnapshotDirectory::open(dir)`.

The registered candidate `snapshot-opened-generation-v1` is now revision 12 and is classified as a model. It is rebound to the current shared inputs, including the exact rollback proof body. The current correctness receipt `.system-proof/evidence/verification-e5e8a426a63942acada4892051895b69/correctness.json` fails before verification because the public verifier command still parses `proofs/snapshot.rs` without the Rust 2024 edition flag needed for the rollback let-chain. The current supplied checker receipt `.system-proof/evidence/specdet-4365a3c7aa2f4970/completeness.json` uses `--edition=2024`, selects `opened_snapshot_open_view`, and then fails when Verus rejects the exact rollback method's `mut self` receiver later in the same proof file. This preserves the restore-access meaning as current model feedback, but it does not prove the production source body in `openvmm/openvmm_helpers/src/snapshot/restore.rs`.

The restore-open View remains useful as a dependent meaning: successful structural open gives retained handles, state length agreement, memory generation length agreement, and intended-capture propagation from an upstream publication or caller proof. The representation of retained directory generations and the intended-capture composition proof remain open.

## Publication source-bound fragment

The intended body remains `write_snapshot_with_memory_publication` in `openvmm/openvmm_helpers/src/snapshot/publish.rs`. A proof attempt preserving its staging, publication/rollback, committed parent-sync failure, and successful-return control flow is retained in `research/system-proof/candidates/publication-orchestrator-limitation.txt`. The configured Verus frontend rejected the source-shaped closure:

```text
ensure_path_absent(dir, "snapshot destination").and_then(|()| staging.publish(dir))
```

because the closure mutably captures `staging`. The private diagnostic is `.system-proof/evidence/publication-orchestrator-verus-limitation.txt`. This leaves the orchestrator open rather than converting it into a rewritten Boolean model.

The registered non-model fallback is the same-path callee `StagingDirectory::publish`, candidate `snapshot-staging-publish-v1` revision 10. Its `source_binding` names exact symbols:

- source: `openvmm/openvmm_helpers/src/snapshot/publish.rs`, `StagingDirectory::publish`
- proof: `proofs/snapshot.rs`, `StagingDirectory::publish`

The current correspondence receipt `.system-proof/evidence/staging-publish-connection.json` reports `exact_fragment`: the executable body and parameter syntax match, while type, callee, macro, formatting/context, filesystem, and resource semantics remain explicit dependencies.

The rename/context representation account is now recorded in `research/system-proof/candidates/rename-resource-account.txt` and reflected in `proofs/snapshot.rs`. The proof-side `Context::with_context` interface preserves the `Ok`/`Err` shape of the underlying rename result, `rename_no_replace_view` names the success-as-commit and error-before-commit boundary, and `StagingDirectory::publish` now states that an error return leaves the staging owner path unchanged. The source basis is `openvmm/openvmm_helpers/src/snapshot/publish.rs` for the early-return-before-retirement body and `openvmm/openvmm_helpers/src/snapshot/fs.rs` for the Linux-GNU `renameat2(..., RENAME_NOREPLACE)` implementation. The fallback `fs_err::rename` implementation remains a platform semantics dependency rather than a closed no-replace proof.

The current correctness receipt `.system-proof/evidence/verification-957983d9a8ec4802970a9bde21dc9440/correctness.json` fails before reaching the publish proof because the public verifier command omits the Rust 2024 edition flag and the exact rollback body later in the shared proof file uses a let-chain. The current completeness receipt `.system-proof/evidence/specdet-b1cf8ada206244a6/completeness.json` selects `snapshot.rs::StagingDirectory::publish` with `--edition=2024`, then fails when Verus rejects the rollback method's exact `mut self` receiver in the same proof file. The earlier publish proof and resource-determinism feedback are retained as historical learning in `research/system-proof/candidates/rename-resource-account.txt`, but the current registered candidate is blocked, not supported.

The rename/context representation account remains the intended open resource boundary. The source method takes `&Path` values, not an explicit filesystem/resource state, so the same paths may encounter destination absence, destination presence, parent-open failure, or a race. That is not a reason to assume publication always succeeds or to claim the full orchestrator is proved.

## Rollback source-bound fragment

The current rollback candidate is `snapshot-staging-rollback-v1` revision 6. Its `source_binding` names exact symbols:

- source: `openvmm/openvmm_helpers/src/snapshot/publish.rs`, `StagingDirectory::rollback`
- proof: `proofs/snapshot.rs`, `StagingDirectory::rollback`

The correspondence receipt `.system-proof/evidence/staging-rollback-connection.json` reports `exact_fragment`: the executable body and parameter syntax match the production method. That body keeps the important branch distinction. If a source-memory alias exists and `remove_staging_directory` fails, rollback returns `SnapshotWriteError::CleanupUncertain` with the staging path, original publication error, and cleanup error. Otherwise rollback returns `SnapshotWriteError::BeforeCommit`; an independent cleanup failure is logged but does not become `CleanupUncertain`. In the shared View, both rollback outcomes are pre-commit and do not imply that the final destination was published.

The candidate is registered but blocked, not supported. The current correctness receipt `.system-proof/evidence/verification-135649be06a5463abbd4dc6c19c02eb5/correctness.json` fails before verification because the public verifier command still omits the Rust 2024 edition flag needed for the production let-chain in the exact body. The supplied checker receipt `.system-proof/evidence/specdet-89dc4bba501a4bc1/completeness.json` uses `--edition=2024` from `proofs/specdet.toml`, selects `snapshot.rs::StagingDirectory::rollback`, and then fails when Verus rejects the exact `mut self` receiver. The direct diagnostic `.system-proof/evidence/staging-rollback-verus-edition-2024.txt` confirms the same `mut self` limitation with the project-local Verus executable. This preserves a real source-shaped candidate and a useful frontend diagnostic, but it is not a verified production proof.

## Current proof map

The persistent graph was updated through:

```text
system-proof --project /home/ruize/argus-system-proof-agent/projects/openvmm graph put --file research/system-proof/nodes/opened-snapshot-open.json --reason 'Refresh snapshot graph after final restore model rebind and rollback-shared frontend blockers'
system-proof --project /home/ruize/argus-system-proof-agent/projects/openvmm graph put --file research/system-proof/nodes/staging-rollback.json --reason 'Refresh rollback blocker node with revision 6 receipts'
```

The graph now treats `snapshot.publish.staging_directory_publish` as blocked in the current registry, not supported by the older passing receipt: exact source correspondence exists, but both current tracks stop on the rollback method in the shared proof file before the publish proof is reached. It treats `snapshot.publish.rollback_classification` as blocked rather than supported: exact source correspondence exists for `StagingDirectory::rollback`, but both public tracks stop on Verus frontend/configuration support for the exact Rust 2024 body. It treats `snapshot.restore.opened_generation_view` as current model feedback with blocked tool tracks, not stale source-body evidence, and it keeps `snapshot.publish.write_snapshot_with_memory_publication` open with the retained closure limitation.

The current `system-proof check` result is incomplete, but not because the restore candidate is stale. The current issues are:

```text
Tool errors/unsupported reports alone do not establish a reviewable result
No current exact-source-fragment body feedback; declaring a Boolean summary source_derived is insufficient
```

That result is the remaining obstacle. Repairing it inside the current writable scope would require either changing the verifier configuration that omits `--edition=2024` from the public correctness command or changing the exact rollback method shape that currently gives source-fragment correspondence. The first file is outside this mission's writable paths, and the second would destroy the selected source-bound body rather than proving it.

## Open dependencies

The next publication work should target the full orchestrator once the Verus closure limitation is resolved or an equivalent source-preserving translation is supported. Until then, the following dependencies remain open:

- `stage_snapshot` must establish a complete staging generation.
- `StagingDirectory::rollback` has exact source correspondence for the branch classification, but proof and completeness tracks are blocked by the current Verus handling of the exact Rust 2024 body and `mut self` receiver.
- `rename_no_replace`, path ownership, platform no-replace behavior, and filesystem resource effects must be connected to an explicit resource interface; the local `with_context` result-shape and failure-owner frame have been repaired in the current proof-side account.
- `remove_staging_directory`, path ownership during cleanup, source-memory alias lifetime, and cleanup filesystem effects remain explicit rollback dependencies.
- The committed parent-sync failure branch must remain distinct from both absence and full success.
- Intended-capture membership still requires the caller’s stopped-source boundary, exact memory handle, source-file quiescence, and access-control assumptions.
- Restore-side retained directory/file generation representation remains separate from the model result.

Human review remains pending. These are retained partial results and blocked attempts, not a release-ready source proof of `write_snapshot_with_memory_publication` or the whole snapshot module.
