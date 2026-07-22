# 概览聚合统计：今日请求与失败

## Goal

在概览页展示基于 `request_logs` 的**本地自然日**聚合：总请求、成功、失败、故障转移次数，便于一眼判断代理健康。

## Requirements

- R1：后端 `get_request_stats`（或等价）按本地日历「今天 00:00–明日 00:00」统计 unix `time`。
- R2：指标：`total`、`success`（2xx）、`failure`（status≥400 或 error 非空）、`failover`（from/to 任一非空）。
- R3：概览页卡片展示上述数字；随代理状态刷新一并刷新；可单独刷新。
- R4：空日志时全 0；中文标签。
- R5：domain 单测覆盖分类计数。

## Acceptance Criteria

- [x] 概览可见今日四项统计。
- [x] 成功/失败/故障转移语义与日志页筛选一致。
- [x] 单测通过；`pnpm typecheck` / `lint` 通过。

## Out of Scope

- 多日趋势图、按分组拆分、导出。
