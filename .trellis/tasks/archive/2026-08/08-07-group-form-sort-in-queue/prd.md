# 排序按钮移入故障转移队列卡片

## Goal

「按模型能力排序」按钮从页面顶部工具行移入右栏「故障转移队列」卡片 header，与「清空」并排。

## Requirements

1. 右栏 HCard #header 右侧新增「按模型能力排序」按钮（`sortQueueByCapability`，disabled 条件沿用 `items.length < 2 || leaderboardLoading`）
2. 页面顶部工具行移除排序按钮；「强制刷新榜单」与状态文本保留（榜单是排序数据源辅助）
3. 队列为空时排序按钮禁用；点击行为不变

## Acceptance Criteria

- [ ] AC1：排序按钮位于故障转移队列卡片 header 内，清空按钮旁
- [ ] AC2：点击仍触发按 llm_benchmark 榜单排序，行为与文案不变
- [ ] AC3：`pnpm typecheck` / `pnpm lint` / `pnpm build` 全绿
