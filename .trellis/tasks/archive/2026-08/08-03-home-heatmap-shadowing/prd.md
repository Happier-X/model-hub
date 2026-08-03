# 修复首页热力图变量遮蔽导致不展示

## Goal

修复首页「每日请求量（近一年）」热力图完全不渲染的问题。根因是 `HomePage.vue` 中 `heatmapData` 计算属性内部用 `const daily = daily.value` 遮蔽外层 `ref`，触发暂时性死区（TDZ）运行时错误，计算属性一求值就抛 `ReferenceError`。

## 根因（已确认）

```ts
const daily = ref<RequestDailyCounts | null>(null);

const heatmapData = computed<HHeatmapData>(() => {
  const daily = daily.value; // TDZ：内部 const 遮蔽外层 daily，求值时 ReferenceError
  ...
});
```

构建产物也证实自引用：`const G = G.value`。

## 范围内

- 修改 `src/pages/HomePage.vue` 的 `heatmapData` 计算属性：内部局部变量改名，消除与外层 `daily` ref 的遮蔽。
- 保持既有 365 天全网格补全逻辑不变（`end_unix` 向前 365 天、按 `day_start_unix` 查值、无记录填 0）。
- 不改后端、不改 `HHeatmap`、不改 IPC。

## Out of Scope

- 不改 `request_daily_counts` / 日志保留策略。
- 不改 `happier-ui` 的 `HHeatmap` 组件。
- 不引入热力图天数可配置。
- 不处理窄屏下热力图横向裁切（`HCard overflow:hidden` 相关，另开任务）。

## Requirements

1. `heatmapData` 求值不再抛 `ReferenceError`。
2. `daily` 为 `null` 时返回 `[]`。
3. `daily` 有值时返回 365 天完整 `HHeatmapData`（含 value=0 的空日）。
4. 有记录日的 `value` 仍按 `day_start_unix` 正确映射。

## Acceptance Criteria

- [ ] `heatmapData` 内部局部变量不再与外层 `daily` 同名
- [ ] `pnpm typecheck` 通过
- [ ] `pnpm lint` 通过
- [ ] 构建产物中不再出现 `const G=G.value` 这类自引用
- [ ] 首页热力图在有/无请求记录时都能渲染 365 天网格

## Notes

- 轻量级 bugfix，PRD-only，无需 `design.md` / `implement.md`。
- 修复后建议在 Phase 3 用 `trellis-break-loop` 或 `trellis-update-spec` 把「computed/回调内禁止与外层 ref 同名」写进 frontend 规范，避免同类 TDZ。
