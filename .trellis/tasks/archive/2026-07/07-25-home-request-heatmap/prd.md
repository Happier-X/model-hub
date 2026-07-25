# 首页每日请求量热力图

## Goal

在首页新增一块「每日请求量热力图」，使用 `happier-ui@0.0.6` 新提供的 `HHeatmap`（GitHub 贡献图风格），按本地自然日聚合过去一年的请求日志总量并可视化，帮助用户一眼看到代理的活跃程度与波动。

## Background

- `happier-ui` 已从 `0.0.3` 升级到 `0.0.6`，新增 `HHeatmap` 组件，输入 `HHeatmapData = { timestamp: number(ms); value?: number | null }[]`。
- 现有首页 (`src/pages/HomePage.vue`) 只有「今日请求」聚合卡（`get_request_stats` → `request_stats_today`）与「最近成功请求」，没有历史趋势视角。
- 后端 `request_logs` 表逐条落盘请求，`time` 字段为本地 unix 秒（写入时 `chrono::Local::now().timestamp()`）。

## Requirements

### 功能

1. 首页在「今日请求」卡片下方（或作为独立卡片）新增「每日请求量」区块，展示 `HHeatmap`。
2. 数据窗口：最近约 12 个自然月（≥ 365 天，避免热力图渲染稀疏）。
3. 聚合口径：本地自然日 00:00 - 次日 00:00（复用 `request_stats_today` 已在用的时区语义），`value` = 该日请求总条数（含成功/失败/故障转移，不再拆分）。
4. 无数据的日期不必显式返回；`HHeatmap` 会用 `colors[0]` 渲染空色格。
5. 加载态：热力图数据未就绪时 `HHeatmap :loading="true"`。
6. 刷新：复用「今日请求」现有的「刷新统计」按钮，一并刷新热力图数据（一次点击刷两个指标）。
7. 错误：接口失败时在卡片内展示错误文案，参照现有 `statsError` 的样式。

### 数据契约

后端提供新的聚合 IPC，避免前端拉全表：

- 命令名：`get_request_daily_counts`
- 入参：`{ days: u32 }`（可选；不传则默认 365；上限 400，防止一次拉太多桶）
- 返回：`RequestDailyCounts { days: Vec<DailyCount>, start_unix: i64, end_unix: i64 }`
  - `DailyCount { day_start_unix: i64, count: i64 }`
  - `start_unix` = 窗口首日 00:00 本地 unix 秒
  - `end_unix` = 今日 24:00（次日 00:00）本地 unix 秒（半开区间）
  - `days` 只返回 `count > 0` 的日期（前端稀疏渲染即可）

### 非功能

- 后端聚合走单条 SQL 或应用层分桶，总耗时目标 < 100ms（默认 30 天日志保留下，行数一般 ≤ 数万）。
- 沿用 `request_logs.time` 现有索引；本次不加新索引。
- 时区：跟随本机 Local 时区（与既有 `request_stats_today` / `request_stats_between` 一致）。

## Constraints

- 不改 `request_logs` 表结构，不加新迁移。
- 不引入前端图表库；只用 `HHeatmap`。
- 不改变热力图色带默认值（组件自带 `colors` 已够用）。
- Web 场景（本项目仅 Tauri Desktop）无需 SSR 兼容。

## Non-Goals

- 不拆分「成功/失败/故障转移」的多色热力图；只做总量。
- 不做鼠标悬停日期 tooltip 定制（组件默认交互即可，如组件未提供 tooltip 则本轮不补）。
- 不做时间窗口切换器（30 天 / 90 天 / 365 天），保持默认 365 天。
- 不做导出与筛选。

## Acceptance Criteria

- [ ] 后端 `LogStores` 提供 `request_daily_counts(days: u32) -> Result<RequestDailyCounts, AppError>`
- [ ] `commands::get_request_daily_counts` 已注册进 `invoke_handler`
- [ ] 前端 `api/tauri.ts` 暴露 `getRequestDailyCounts(days?: number)` 与对应 TS 类型
- [ ] 首页新增热力图卡片；365 天默认窗口；无数据日空色格
- [ ] 「刷新统计」按钮同时刷新今日聚合与热力图
- [ ] `cargo test` 全绿；新增至少 1 个后端单测（构造多条跨日 log，验证聚合桶）
- [ ] `pnpm lint && pnpm typecheck && pnpm test:unit` 全绿

## Open Questions

- 无。默认 365 天窗口 + 单色总量热力图为本轮 MVP。
