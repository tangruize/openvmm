# OpenVMM snapshot specification: pre-Argus seed

This branch packages the customized specification and source supplied before the October 15, 2026 Argus experiment, together with its original task inputs and the earlier natural-language restore design. It is not the agent's later proof result.

## Start here

- [INPUT.md](INPUT.md): the original experiment input, including the intended save/restore projection correspondence.
- [human-howto-spec.txt](human-howto-spec.txt): the earlier natural-language instructions for specifying `InitializedVm::load(Some(_))`, its snapshot inputs, and its actual returned resources. This restore-oriented brief is historical design input, not a replacement for the unified save/restore task.
- [goal.txt](goal.txt) and [targets.txt](targets.txt): the original objective and four entry points.

The `/workspace/task/production` path in the original input is the experiment's container path. On this review branch, the corresponding source is the repository itself.

## Where the formal specification lives

The main model is [openvmm/openvmm_defs/src/snapshot.spec.rs](openvmm/openvmm_defs/src/snapshot.spec.rs). Start with `LogicalVmState`, `PublishedSnapshotView::captures`, `PublishedSnapshotView::restores_into`, and `save_prepare_worker_composes`. Save and restore use a common snapshot-relevant state rather than demanding equality of complete runtime objects.

[openvmm/openvmm_core/src/worker/dispatch.spec.rs](openvmm/openvmm_core/src/worker/dispatch.spec.rs) re-exports that shared model; it is not a separate restore specification. Representation support lives in the adjacent [dispatch.proof.rs](openvmm/openvmm_core/src/worker/dispatch.proof.rs).

The real entry-point contracts are attached to:

- `VmController::handle_guest_snapshot_request` in [vm_controller/microvm.rs](openvmm/openvmm_entry/src/vm_controller/microvm.rs).
- `prepare_snapshot_restore_for_config` in [snapshot_restore/prepare.rs](openvmm/openvmm_entry/src/snapshot_restore/prepare.rs).
- The inherent `VmWorker::new(VmWorkerParameters)` and `InitializedVm::load` in [worker/dispatch.rs](openvmm/openvmm_core/src/worker/dispatch.rs).

This is an initial, refinable specification. The supplied entry-point bodies begin as `external_body`; having these contracts and abstract composition lemmas is not a completed proof of the implementation.

## Provenance

The source baseline is `nanvix/openvmm` commit `7f225e2cd635bbd1e1839e587e659996433396c8`. The customized pre-experiment source is commit `f127b8c455cabb04125eb582772c6eea326b5cc6`, also preserved on `system-proof/snapshot-save-restore-seed`.

The original `INPUT.md`, `goal.txt`, and `targets.txt` are preserved byte-for-byte from task archive commit `031ac3c43c881409e191b32968fd4cd50d8ded29` on `system-proof/snapshot-save-restore-task-20261015`. Its `production` gitlink identifies the same `f127b8c` source. The natural-language brief is copied unchanged from the historical local `human-howto-spec.txt`.

This publication adds only review documents and task inputs. It does not modify the pre-Argus source or include campaign credentials, environment files, state, or transcripts.
