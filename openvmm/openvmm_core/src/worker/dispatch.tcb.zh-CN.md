# 初始 snapshot 信任边界

提供的合同故意不把 OpenVMM 自己拥有的 snapshot 生命周期放进信任边界。实验必须证明或明确处理 capture cut 选择、state-unit quiesce/save/restore、inventory 验证、RAM flush 与 backing 兼容性、时间调整、VP 数量选择、错误传播、清理、发布顺序，以及 `InitializedVm::load(Some(_))` 到 `LoadedVm::restore_from_snapshot` 的连接。

初始 TCB 可以包含 Verus、Rust 编译语义、所选 hypervisor 和操作系统 API 的文档化接口边界、在明确假设后的文件系统 durability primitive、按规范使用的密码学 primitive，以及受支持配置依赖的硬件行为。Opaque component payload 和外部资源需要显式合同；仅仅 opaque 并不会使 OpenVMM orchestration 自动成为 trusted。

`dispatch.proof.rs` 中的 uninterpreted representation function 是 seed boundary，不是已经完成的 correspondence proof。在端到端结果声称实现对应关系之前，必须用 source-grounded View 对它们进行细化、约束或替换。

非语义 logging 和 diagnostic formatting 可以集中封装在 `verus_keep_ghost` wrapper 后。具有语义作用的条件、参数求值、状态变化、结果分类、清理、恢复和返回路径必须继续对验证可见。
