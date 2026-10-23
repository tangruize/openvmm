# OpenVMM snapshot 规格：Argus 实验前版本

本分支收集了 2026 年 10 月 15 日 Argus 实验启动前定制的规格与源码、当时的原始任务输入，以及更早的自然语言 restore 规格设计说明。它不是 Agent 后来生成的证明结果。英文 `SNAPSHOT-SPEC.md` 为权威说明，本文件为中文镜像。

## 阅读入口

- [INPUT.md](INPUT.md)：原始实验输入，包含 save/restore projection correspondence 的目标。
- [human-howto-spec.txt](human-howto-spec.txt)：更早的自然语言说明，描述如何为 `InitializedVm::load(Some(_))`、snapshot 输入和实际返回的 VM 资源建立规格。这是面向 restore 的历史设计输入，不替代统一 save/restore 任务。
- [goal.txt](goal.txt) 与 [targets.txt](targets.txt)：原始目标和四个真实入口。

原始输入中的 `/workspace/task/production` 是实验容器路径。本分享分支对应的源码直接位于仓库中。

## 正式规格在哪里

主要模型位于 [openvmm/openvmm_defs/src/snapshot.spec.rs](openvmm/openvmm_defs/src/snapshot.spec.rs)。建议从 `LogicalVmState`、`PublishedSnapshotView::captures`、`PublishedSnapshotView::restores_into` 和 `save_prepare_worker_composes` 开始。Save 与 restore 共用 snapshot-relevant state，不要求完整运行时对象相等。

[openvmm/openvmm_core/src/worker/dispatch.spec.rs](openvmm/openvmm_core/src/worker/dispatch.spec.rs) 重新导出这个共享模型，不是另一套 restore 规格。具体表示的支撑定义在相邻的 [dispatch.proof.rs](openvmm/openvmm_core/src/worker/dispatch.proof.rs)。

真实入口合同附着于：

- [vm_controller/microvm.rs](openvmm/openvmm_entry/src/vm_controller/microvm.rs) 中的 `VmController::handle_guest_snapshot_request`。
- [snapshot_restore/prepare.rs](openvmm/openvmm_entry/src/snapshot_restore/prepare.rs) 中的 `prepare_snapshot_restore_for_config`。
- [worker/dispatch.rs](openvmm/openvmm_core/src/worker/dispatch.rs) 中的 inherent `VmWorker::new(VmWorkerParameters)` 和 `InitializedVm::load`。

这是允许进一步修正的初始规格。提供的入口函数体初始带有 `external_body`；存在这些合同和抽象组合 lemma 不代表实现已经证明完成。

## 来源

源码基线是 `nanvix/openvmm` 的 `7f225e2cd635bbd1e1839e587e659996433396c8`。实验前的定制源码为 `f127b8c455cabb04125eb582772c6eea326b5cc6`，也保存在 `system-proof/snapshot-save-restore-seed` 分支上。

`INPUT.md`、`goal.txt` 和 `targets.txt` 与 `system-proof/snapshot-save-restore-task-20261015` 分支的任务归档提交 `031ac3c43c881409e191b32968fd4cd50d8ded29` 逐字节一致。该归档的 `production` gitlink 指向相同的 `f127b8c`。自然语言说明原样复制自历史本地文件 `human-howto-spec.txt`。

这次发布仅增加阅读说明和任务输入，不修改实验前源码，也不包含 campaign 凭据、环境文件、状态或对话记录。
