# 供应商列表「启用」列改为开关

## Goal

把供应商列表表格中只读的「启用 / 停用」文本列，改为可直接在行内切换的 `HSwitch` 开关，用户无需进入编辑对话框即可启停供应商。

## Background

- 当前 `src/pages/ProvidersPage.vue` 中 enabled 列单元格仅渲染文本：
  `{{ (row as Provider).enabled ? "启用" : "停用" }}`（L296-297）。
- 后端已提供整行更新的 `update_provider` 命令（`src-tauri/src/domain/provider.rs:111`，
  payload 为 `{ id, name, base_url, api_key, enabled }`，校验名称、Base URL 非空），
  前端封装见 `src/api/tauri.ts:179` 的 `updateProvider`。
- `HSwitch` 已在 happier-ui 中导出且支持 `v-model`、`size`、`disabled`、`ariaLabel`，
  props 形态与 `HCheckbox` 一致（`modelValue` + `update:modelValue`）。
- 项目规范（`spec/frontend/component-guidelines.md` 3.1）明确：**布尔启用类用 `HSwitch`/`HCheckbox`**。
- 现有设置页切换模式（`SettingsPage.vue` 的 `toggleOverlay`/`toggleStartupCheck`）是
  乐观更新 + 失败回滚 + `disabled` 防重复的参考样板。

## Requirements

1. 供应商表格 enabled 列单元格改为 `HSwitch`，绑定 `(row as Provider).enabled`，
   切换即触发后端更新。
2. 切换逻辑新建 `toggleProviderEnabled(p: Provider, next: boolean)`：
   - 先乐观更新本地 `items` 中该行的 `enabled`；
   - 调用 `updateProvider`，payload 携带该行现有 `name` / `base_url` / `api_key` 与新 `enabled`；
   - 成功后用返回值同步该行（以服务端为准）；
   - 失败回滚本地 `enabled` 并写入 `error`（沿用 `extractInvokeError`）；
   - 切换进行中的行开关置 `disabled`，防重复点击（可用行级 `Set<number>` ref 记录 in-flight id）。
3. 失败文案必须清晰可行动（中文），与页面既有 error 文案一致。
4. 编辑对话框内的「启用」`HCheckbox` 暂保留不变（编辑流程仍可改 enabled，两处互不冲突）。
5. 不引入新的后端命令，不改数据库 / payload 结构。

## Out of Scope

- 后端 `update_provider` 命令与 payload 结构（仍为整行更新）。
- 对话框内启用复选框、删除/编辑按钮、粘贴快速添加等既有功能。
- 引入 `setEnabled` 之类的部分字段更新命令（无必要，YAGNI）。

## Acceptance Criteria

- [ ] 供应商列表 enabled 列显示 `HSwitch` 开关，反映该行真实 enabled 状态。
- [ ] 点击开关即切换并保存到后端，无需打开编辑对话框。
- [ ] 切换进行中的行开关禁用，不可重复点击；成功后恢复可点。
- [ ] 切换失败时本地状态回滚到切换前值，页面 `error` 显示中文错误文案。
- [ ] `npm run build`（`vue-tsc --noEmit && vite build`）通过，无类型错误。
- [ ] `npm run lint` 通过。
- [ ] 编辑对话框启用复选框行为不受影响。

## Notes

- 轻量级前端改动，仅 `src/pages/ProvidersPage.vue`。PRD-only，无需 `design.md` / `implement.md`。
- 校验命令：`npm run build`、`npm run lint`。
- 参考实现：`src/pages/SettingsPage.vue` 中 `toggleOverlay` 的乐观更新 + 回滚 + disabled 模式。
