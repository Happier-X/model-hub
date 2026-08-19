# 供应商页删除确认改用 AppDialog

## Goal

移除 `src/pages/ProvidersPage.vue` 中删除供应商时使用的浏览器原生 `confirm()`，改用项目统一封装的 shadcn-vue 对话框组件 `AppDialog`，保持与其他业务弹窗一致的交互风格。

## Requirements

- 删除供应商前必须二次确认，确认文案保持「确认删除该供应商？」语义。
- 复用现有 `AppDialog` 组件，不引入新依赖（原生 `confirm` 是阻塞式的，`AppDialog` 是异步的，需用 reactive 状态驱动）。
- 不影响现有「编辑/新建供应商」的 `AppDialog` 用法。
- 删除动作的异步行为（`deleteProvider` → 失败时 `error` 回显）保持不变。

## Acceptance Criteria

- [ ] `src/pages/ProvidersPage.vue` 中不再出现 `confirm(` / `window.confirm`。
- [ ] 点击「删除」按钮弹出 `AppDialog` 二次确认框，标题与文案为删除确认语义。
- [ ] 确认后调用 `deleteProvider` 并刷新列表；取消则关闭弹窗且不触发删除。
- [ ] 删除失败时仍沿用现有 `error` 回显机制（不改变现有错误展示方式）。
- [ ] 不影响「新建/编辑供应商」弹窗及开关启停、立即同步等既有功能。
- [ ] 类型检查 / lint 通过。

## Notes

- 轻量任务，PRD-only，无需 design.md / implement.md。
- 仅改动 `src/pages/ProvidersPage.vue` 一个文件（状态 + 模板）。