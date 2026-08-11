# 首页统计改为按天聚合表：修复总计被 7 天/1 万条保留策略锁死、数字不涨反降

## Goal

首页四张统计卡片显示的全历史"总计"被 `request_logs` 保留策略（`LOG_RETENTION_DAYS = 7` 天 / `LOG_MAX_ROWS = 10000` 条）锁死：实测库内恰好 10000 条，每写一条新日志 purge 就删掉最老一条，总计 +1/-1 相抵不涨，且 7 天窗口滚动导致老数据批量消失、数字不涨反降。

方案：新增按天聚合表 `daily_request_stats`（每天一行，只统计成功请求），写日志时同事务 upsert 当天聚合；首页总计/热力图/时间序列统计改读聚合表。明细表保留策略不变，统计数字从此只增不减、实时准确；旧库启动时幂等回填。

## Confirmed Facts

- `request_logs` 保留常量：`LOG_RETENTION_DAYS = 7`、`LOG_MAX_ROWS = 10000`（`src-tauri/src/domain/log.rs`）。
- `insert_log_best_effort` 每次写入后触发 `purge_expired_logs_best_effort`；`ensure_stores`、`list_logs`、应用启动也会 purge。
- 真实库（`%APPDATA%/com.modelhub.desktop/gateway/data/data.db`）当前 10000 条，今日 6273 条、08-10 1246 条、08-07 18 条、08-06 2463 条；总计已触顶，且 08-06 老记录正被新请求逐条挤掉。
- 统计口径：仅成功请求（2xx 且 error 为空）计入；费用不落库，统计时按 `model_pricing` 单价现算（改价可重算历史，既有设计）。
- 前端接口形状：`get_request_overview` → `{total, today}`；`get_request_daily_counts` → `{days, start_unix, end_unix}`；`get_timeseries_stats` → `{daily, hourly}`。今日小时序列目前只能由明细现算（今日明细在 10000 上限下始终完整保留，见 design.md）。
- 历史背景（journal session 43/52）：用户明确要求卡片显示全历史"总计"（不做"今日增量行"）；实时刷新已由事件驱动（`stats-changed`）+ 5s 轮询 + 恢复可见刷新保障，本轮不改变前端刷新链路。
- 当前工作区另有 `08-11-pricing-auto-sync-remove-settings`（planning）任务，本任务不触碰其范围。

## Requirements

- R1：新增按天聚合表 `daily_request_stats`（每天一行：`day_start_unix`、`requests`、`input_tokens`、`output_tokens`、`use_time_ms`），仅累计成功请求（2xx 且 error 为空），与现有统计口径一致。
- R2：`insert_log` 成功且属成功口径时，在同一 `with_conn` 事务内 upsert 当天聚合行（同一天累加，跨天新建），保证明细与聚合一致。
- R3：首页统计查询改读聚合表：`request_overview` 的 `total`/`today`、`request_daily_counts`、`request_daily_stats`（含费用现算：SUM(token) × 当前单价 / 1e6）；`request_hourly_stats` 保持从今日明细现算（今日明细恒在库内）。
- R4：接口形状与返回结构完全不变，前端（StatsCards/Heatmap/StatsChart/HomePage）无需改动，刷新链路（事件 + 轮询）不变。
- R5：旧库兼容：启动时幂等回填聚合表（表空时在锁内从现存 `request_logs` 扫描重建），不重复累加；被 purge 已删除的历史无法恢复，从回填时刻起只增不减。
- R6：`request_logs` 保留策略（7 天 / 10000 条）与 purge 触发点不变，明细库体积保持有界。
- R7：purge 之后统计数字不得减少（核心回归：写日志 + purge 后 total 仍单调递增）。

## Acceptance Criteria

- [ ] AC1：`daily_request_stats` 表随库创建（`CREATE TABLE IF NOT EXISTS` 迁移模式）；成功请求写入后当天聚合行累加，失败/带 error 请求不累计。
- [ ] AC2：同一日多条成功请求 → 该日聚合行 `requests/tokens/use_time` 累加正确；跨日写入 → 自动新建次日行。
- [ ] AC3：`request_overview().total` 从聚合表 SUM，数值等于"现存明细 + 已 purge 历史"的全历史累计；purge 后 total 不降（回归测试覆盖）。
- [ ] AC4：`request_daily_counts` / `request_daily_stats` 返回与现实现等价的按日数据（窗口/升序/空日补 0 语义不变），费用按当前单价现算且数值正确。
- [ ] AC5：`request_hourly_stats` 行为不变（今日 24 小时，空小时补 0）。
- [ ] AC6：旧库（无聚合表或聚合表为空）首次查询/启动时回填正确且幂等（两次启动不翻倍）。
- [ ] AC7：`cargo test` 全量通过（含新增聚合/回填/ purge 回归测试）；`cargo build` 通过。
- [ ] AC8：前端零改动；运行应用后首页总计在请求期间只增不减，热力图/图表正常。

## Out of Scope

- 不改变 `LOG_RETENTION_DAYS` / `LOG_MAX_ROWS` 数值与 purge 策略。
- 不改变"费用按单价现算"设计（不做历史价格快照、不落费用到聚合表）。
- 不调整前端刷新链路（事件/轮询/可见刷新）、卡片布局或文案。
- 不优化"每条日志都 purge"的性能问题（可列为后续项）。
- 不触碰 `08-11-pricing-auto-sync-remove-settings` 任务涉及的价格同步/设置页范围。
- 不恢复已被 purge 删除的历史数据（不可恢复）。

## Risks / Deferred Items

- 回填只能覆盖现存明细；已 purge 的历史累计缺口无法弥补，需在交付说明中告知用户（自修复起数字只增不减）。
- `daily_request_stats` 写入与回填的并发安全依赖 `with_conn` 互斥锁；回填采用"锁内清空 + 扫描重建"保证原子幂等。
- 若未来某日请求量在单日内即超过 10000 条，今日小时统计的明细来源仍完整（purge 保留最新 N 条），不受影响。
