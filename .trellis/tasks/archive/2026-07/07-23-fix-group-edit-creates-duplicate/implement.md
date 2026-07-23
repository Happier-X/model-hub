# 实施计划

1. 前端 `GroupsPage.vue`：将 `editing` 替换为 `editingGroupId`；保存分支快照 id；增加 `saving` 防重复；更新按钮文案/编辑提示。
2. 前端测试：提取 `getGroupSaveMode(editingGroupId)` 等纯函数，覆盖编辑/新建分支与 id 稳定性。
3. 后端 `domain/group.rs`：补 update 添加第二供应商模型的回归测试，确认不新增 groups。
4. 检查分组页面所有异步/添加/排序路径不清空编辑 id；不触发自动上游请求。
5. 运行 `cargo test`、`pnpm test:unit`、`pnpm typecheck`、`pnpm lint`。
6. 更新 frontend spec（编辑表单必须使用稳定 id，保存失败保留编辑态）。
