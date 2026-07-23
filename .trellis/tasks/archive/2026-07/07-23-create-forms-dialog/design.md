# 技术设计

## 组件边界

新增轻量通用组件 `src/components/AppDialog.vue`，不引入第三方 UI 库。组件只负责对话框外壳与可访问性交互，具体表单和保存逻辑继续归属页面。

## AppDialog 合同

- Props：`open: boolean`、`title: string`、`size?: "default" | "wide"`、`closeDisabled?: boolean`。
- Emits：`close`。
- 默认插槽承载表单内容。
- 使用 `Teleport` 挂载到 `body`，遮罩覆盖应用区域。
- `open=true` 时显示语义化 `role="dialog"`、`aria-modal="true"`，标题通过 `aria-labelledby` 关联。
- 打开后聚焦对话框容器；关闭后由页面将焦点恢复到触发按钮。
- Escape、遮罩和右上角关闭按钮触发 `close`；点击面板内部不关闭。
- 保存期间通过 `closeDisabled` 禁止关闭，避免异步提交中途重置状态。
- `default` 用于供应商表单；`wide` 用于分组表单，面板最大高度约 `90vh`，内容内部滚动。

## 页面状态流

### 供应商页

- 新增 `dialogOpen`、`saving` 和稳定的 `editingProviderId: number | null`。
- `openCreate()`：重置默认值后打开。
- `startEdit(provider)`：复制数据、记录 id 后打开。
- `closeDialog()`：非保存态关闭并重置。
- `save()`：快照编辑 id；id 非空只更新，null 才创建。成功刷新并关闭，失败保留输入和 Dialog。

### 分组页

- 保留现有稳定 `editingGroupId` 与 `saving` 合同，新增 `dialogOpen`。
- `openCreate()`：调用表单重置后打开。
- `startEdit(group)`：复制完整队列后打开。
- `closeDialog()`：非保存态关闭并重置。
- 成功保存后关闭，失败保留 Dialog 和所有队列编辑状态。
- Dialog 打开本身只处理本地状态，不调用 `fetchProviderModels` 或榜单接口。

## 兼容性与风险

- 后端 IPC 和数据库无变化。
- 页面中的错误提示应位于 Dialog 内，避免保存失败信息显示在遮罩下方。
- 分组表单较长，使用内部滚动避免页面背景滚动干扰。
- 关闭无需脏表单确认，这是明确的产品决策。
