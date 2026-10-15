# OpenVMM 统一 snapshot 任务

该目录用于准备 OpenVMM snapshot save/restore lifecycle 的 System Proof 任务。Production checkout 基于 `nanvix/openvmm` revision `7f225e2cd635bbd1e1839e587e659996433396c8`；当前本地任务 revision 为 `f127b8c455cabb04125eb582772c6eea326b5cc6`。

目标端到端性质是：source VM 捕获的 snapshot-relevant projection 与经过 allowed restore transforms 后的 restored VM projection 等价。`goal.txt` 给出简洁目标，`INPUT.md` 是权威任务意图，`targets.txt` 标识真实 lifecycle entry points。

Production source 在 `openvmm/openvmm_defs/src/snapshot.spec.rs` 中包含 initial shared specification，并在所列 targets 上包含直接的 initial contracts。Initial specification 可以在 source grounding 要求时修正或细化，但必须保留 lifecycle objective。

当前尚未启动 LXD container、service、Argus project、verification worktree 或 campaign。Shared specification 及其 composition lemmas 得到 `2 verified, 0 errors`；entry 和 core crates 的验证在解析 supplied contracts 后到达已知的 Verus `vir/src/prune.rs:464` frontend panic。

本 README 与 `README.zh-CN.md` 是面向人的状态文档，`PROVENANCE.md` 是面向人的来源记录。之后的实验 setup 必须只将 `production/`、`goal.txt`、`INPUT.md`、`targets.txt` 以及 `campaign.env` 中必需的设置复制到 LXD task workspace。
