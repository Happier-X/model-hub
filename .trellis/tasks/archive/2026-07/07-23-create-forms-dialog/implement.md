# 实施计划

1. 新增 `src/components/AppDialog.vue`，实现 Teleport、遮罩、Escape、关闭按钮、焦点和 default/wide 尺寸。
2. 修改 `src/pages/ProvidersPage.vue`：移除常驻表单，增加新建入口；新建和编辑统一使用 Dialog；补充保存中状态与关闭逻辑。
3. 修改 `src/pages/GroupsPage.vue`：移除常驻表单，增加新建入口；将完整队列表单迁移到 wide Dialog；保留稳定编辑 id、保存分支和用户点击拉取模型行为。
4. 检查错误、成功提示和取消路径，确认失败不关闭、关闭重置、成功刷新。
5. 运行 `pnpm test:unit`、`pnpm typecheck`、`pnpm lint`。
6. 使用 `trellis-check` 做组件可访问性、状态流和上游访问约束验收。
