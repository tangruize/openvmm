# 统一 snapshot save/restore 合同

## 状态

这是针对最新 `nanvix/openvmm` task checkout 的初始规范候选。它故意保留未决问题，在证明工作将其视为权威规范前，必须根据真实 caller 和 component implementation 进行 judgment。

该合同复用了此前的 restore 抽象：decoded snapshot 加上 prepared RAM 和外部资源表示一个逻辑 VM 状态；返回的 `LoadedVm` 独立表示一个逻辑 VM 状态；成功 restore 在 guest 尚未执行的受保护返回边界关联两者。新增的关键点是 save 与 restore 使用完全相同的逻辑状态，并通过唯一的 snapshot interpretation 组合。

## 共同逻辑状态

`LogicalVmState` 包含开发者需要保留的状态：解释 snapshot 所需的 machine/profile identity、GPA 到 byte 的 RAM 内容、稳定有序的 VP identity 和状态、component identity/payload/deferred private state、明确观察边界上的 clock/timer state，以及所选模式要求保留的 generation ID 和 entropy。

Host address、file descriptor number、lock、task、channel 和 handle 不按 identity 比较，但重建后的资源仍必须实现相同的 RAM、CPU、component 和资源逻辑状态。

## 完整 snapshot view

`SavedState` 只是 component envelope。`SnapshotArtifactsView` 还包括准确的 RAM generation/backing、manifest interpretation、capture-time clock metadata 和所选模式需要的其他 artifact。

`snapshot_view` 必须连接 decoded artifact 和 memory-file range；`project_vm` 必须连接真实 runtime resource 和 observer。二者都不能使用独立填充、只重复期望值的 witness。

## Save 关系

必须存在一个统一 capture cut `t_cut`：

```text
snapshot_view(published_snapshot) = project_vm(source_vm_at_t_cut)
```

真实路径必须建立 state unit/VP 停止、输入与 retained-state mutation 排除、inventory 稳定、component/time cut 对齐、发布 RAM bytes 与该 cut 一致，以及文件系统只提交一个完整 generation 或失败。

## Restore 关系

边界是 `InitializedVm::load(Some(_))` 成功返回：`LoadedVm::restore_from_snapshot` 已应用 component state、时间策略并获得临时 VP stop guard，但 guest 尚未执行且 external-ready 尚未发布。

```text
project_vm(returned_vm) = allowed_restore_transform(snapshot_view(snapshot), environment)
```

允许的变化只能是显式 runtime-resource reconstruction、声明的 downtime transform、明确策略允许的 generation ID/entropy 替换，以及保持稳定 VP identity 语义的受支持 prefix 选择。不能静默丢弃 payload、RAM、VP state 或 deferred state。

真实调用链为：

```text
InitializedVm::load(Some(saved_state))
    -> LoadedVm::restore_from_snapshot(saved_state)
       -> begin_snapshot_restore(&saved_state)
       -> LoadedVm::restore(saved_state)
       -> finish_snapshot_restore(profile_span)
```

只证明 helper 而不证明真实 `load(Some(_))` 消费它以及返回值连接，不足以完成任务。

## 组合、兼容与失败

成功 save 后在兼容 destination restore，应得到源 cut 状态经过明确 restore policy 的结果。零 downtime 和 identity policy 下应精确保持所选逻辑状态；非 identity policy 必须逐项列出允许差异，并保持其他字段不变。

Compatibility 必须来自真实 caller validation 和支持配置，至少涵盖 machine/profile、RAM layout/size、inventory/component identity、VP/capability、device feature/capacity、clock/time ABI 和 typed external resource reconnect policy。

Save 失败不得发布成功 snapshot generation。Restore 失败不得返回或发布成功恢复的 VM，也不得允许 guest 执行。无需保证完整 rollback，但必须保留 caller 丢弃部分修改资源所需的 ownership/cleanup 语义。

## 改进义务

- 对具有语义内容的 component，用 decoded relation 替换抽象 payload bytes。
- 当前源码匹配时复用此前关于 prepared RAM、deferred virtio、VP identity、reconnect policy 和 clock sample 的详细 restore 设计。
- 从当前 production code 确定 overflow/wrapping、timer expiration、capture cut 和 publication commit point。
- 建立真实 `snapshot_view`/`project_vm` bridge，而不是 proof-only observation。
- 将最终 TOP contract 绑定到真实 save/capture route 和 `InitializedVm::load(Some(_))`。
