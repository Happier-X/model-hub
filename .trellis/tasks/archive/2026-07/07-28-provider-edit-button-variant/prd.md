# 供应商操作栏「编辑」按钮视觉调整为 outline

## Goal

把供应商列表操作栏「编辑」按钮的 `variant` 从 `ghost` 改为 `outline`，
让它在静态/未交互状态下有边框形态感，与右侧红色 `danger-soft` 删除按钮
在"中性操作 / 危险操作"语义上更对称，避免用户误以为按钮"没样式"。

## Background

- 当前 `src/pages/ProvidersPage.vue` 操作栏单元格（actions 列）：
  - 编辑：`<HButton variant="ghost" size="sm">` —— ghost 底色透明、普通文字色，
    仅 `:active` 时给 `surface-secondary` 底色，无 hover、无边框，静态下显得太平庸。
  - 删除：`<HButton variant="danger-soft" size="sm">` —— 浅红底 + 红字，按钮形态明显。
- `HButton` 已支持 `variant: 'primary' | 'secondary' | 'tertiary' | 'outline' | 'ghost' | 'danger' | 'danger-soft'`
  （见 `node_modules/happier-ui/dist/components/HButton.vue.d.ts`）。
- `outline` 变体样式：边框 + 透明底，hover/active 有底色（见 `happier-ui/dist/styles.css`），
  与 `danger-soft` 形成"中性 / 危险"对称。
- 编辑按钮 `@click="startEdit(row as Provider)"` 功能与逻辑不变，只改视觉 `variant`。

## Requirements

1. 把操作栏「编辑」按钮的 `variant` 由 `"ghost"` 改为 `"outline"`
   （`size="sm"`、`type="button"`、`@click` 等其余属性与文案保持不变）。
2. 删除按钮 `variant="danger-soft"` 不动。
3. 不改 `startEdit` / `remove` 等逻辑、不改对话框、不改其他页面。
4. 不引入新组件、不新增 happy-ui issue 跟踪（ghost 无 hover 是库的设计，本仓库业务侧调整即可）。

## Out of Scope

- 删除按钮的 variant 与样式。
- 「编辑」按钮文案、行为、事件绑定。
- 其他页面（分组页 / 日志页 / 设置页）的 ghost 按钮调整。
- happy-ui 库侧 ghost 变体改造。

## Acceptance Criteria

- [ ] 供应商列表操作栏「编辑」按钮 `HButton variant="outline"`，静态下可见边框形态。
- [ ] 「删除」按钮 `variant="danger-soft"` 行为与外观不变。
- [ ] 「编辑」点击仍触发 `startEdit`，对话框逻辑不变。
- [ ] `npm run build` 通过，无类型错误。
- [ ] `npm run lint` 通过。

## Notes

- 轻量级前端改动，仅 `src/pages/ProvidersPage.vue` 一行属性。PRD-only，
  无需 `design.md` / `implement.md`。
- 校验命令：`npm run build`、`npm run lint`。
- 选 `outline` 而非 `secondary`：outline 的透明底 + 边框更轻盈，与 danger-soft
  的柔和色块风格在权重上更协调，不喧宾夺主。
