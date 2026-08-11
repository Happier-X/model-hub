# Design：按天聚合表修复统计锁死

## 背景与问题

`request_logs` 是明细表，受保留策略约束：`LOG_RETENTION_DAYS = 7`（超 7 天删除）+ `LOG_MAX_ROWS = 10000`（仅保留最新 1 万条），且写日志后立即 purge。首页"总计"是**全历史累计**语义，却从明细现算 → 触顶后 +1/-1 相抵不涨，7 天窗口滚动导致老数据批量消失、数字下降。

**核心矛盾**：明细可裁剪（体积有界）与总计需完整（历史累计）是两种不同生命周期的数据，不应存在同一张表上。解法：明细保持可裁剪，新增**按天聚合表**承载累计统计。

## Schema

在 `db/migrate.rs` 新增 `ensure_daily_request_stats_table`（幂等，加入 `migrate()` 调用链）：

```sql
CREATE TABLE IF NOT EXISTS daily_request_stats (
  day_start_unix INTEGER NOT NULL,        -- 本地自然日 00:00 的 unix 秒（与 local_day_start_unix 一致）
  model_name      TEXT NOT NULL DEFAULT '',-- 明细的 upstream_model，费用按模型单价现算所需
  requests        INTEGER NOT NULL DEFAULT 0,
  input_tokens    INTEGER NOT NULL DEFAULT 0,
  output_tokens   INTEGER NOT NULL DEFAULT 0,
  use_time_ms     INTEGER NOT NULL DEFAULT 0,
  PRIMARY KEY (day_start_unix, model_name)
);
```

- 主键带 `model_name`：费用必须按模型单价现算（既有设计"改价可重算历史"），纯天粒度无法算费用。规模估计：日均请求 ~50 模型 × 365 天 ≈ 2 万行/年，可忽略。
- **不落费用**：保持"统计时按 `model_pricing` 现算"，改价可重算历史，与现状一致。
- 只统计成功口径（2xx 且 error 为空），与现有所有统计一致。

## 写入链路（insert_log 内，同事务）

`insert_log` 的 `with_conn` 闭包内，INSERT 明细成功后追加：

```rust
// is_success_log: status_code 200..=299 && error.trim().is_empty()
if is_success_log(&log) {
    let day = local_day_start_unix(time); // time = Utc::now().timestamp()
    conn.execute(
        "INSERT INTO daily_request_stats (day_start_unix, model_name, requests, input_tokens, output_tokens, use_time_ms)
         VALUES (?1, ?2, 1, ?3, ?4, ?5)
         ON CONFLICT(day_start_unix, model_name) DO UPDATE SET
           requests = requests + 1,
           input_tokens = input_tokens + excluded.input_tokens,
           output_tokens = output_tokens + excluded.output_tokens,
           use_time_ms = use_time_ms + excluded.use_time_ms",
        params![day, log.upstream_model, log.input_tokens, log.output_tokens, log.use_time_ms],
    )?;
}
```

要点：
- 与明细 INSERT 同锁（`with_conn` Mutex）、同闭包：要么都成功要么都失败，天然一致，无需额外事务。
- `notify_changed()` 位置不变（仍在闭包外，成功后触发）。
- dual-write 旧列分支与聚合无关，聚合在分支后统一执行。

## 回填（旧库兼容，幂等）

**位置**：`Stores::new` 末尾 best-effort 调用 `backfill_daily_stats()`。所有入口（`runtime.ensure_stores`、`commands::stores`、测试 `setup`）都经 `Stores::new`，单点覆盖。

**算法**（`with_conn` 锁内）：
1. `SELECT COUNT(*) FROM daily_request_stats`；> 0 直接返回（幂等跳过）。
2. `unchecked_transaction` 内：`DELETE FROM daily_request_stats`（清掉极端并发下可能已写入的行，保证重建干净）→ 扫描 `request_logs` 成功口径行的 `(time, upstream_model, input_tokens, output_tokens, use_time_ms)` → 内存 HashMap 按 `(day_start_unix, model_name)` 累加 → 批量 upsert → commit。
3. 任一步失败整体回滚，表保持空，下次启动重试；失败仅 `tracing::warn`，不阻断启动。

**时机安全**：`open_db`（含 `migrate` 建表）先于 `Stores::new`；`ensure_stores` 在代理监听端口之前调用，此时无请求写入；即便有并发，也全部在 `with_conn` 锁内串行。

**语义缺口**：回填只能覆盖现存明细（7 天/1 万条），已被 purge 的历史无法恢复——从回填时刻起数字只增不减，交付说明中告知用户。

## 读链路改造（domain/log.rs）

| 方法 | 改造 |
|---|---|
| `request_overview` / `overview_row` | `total`、`today` 均改从 `daily_request_stats` 聚合，`LEFT JOIN model_pricing`（JOIN 条件与现实现逐字一致：`s.model_name = p.model_name OR p.model_name LIKE '%/' \|\| s.model_name`）算费用；`range` 过滤用 `day_start_unix >= ?1 AND day_start_unix < ?2`（与本地日边界对齐） |
| `request_daily_counts` | `SELECT day_start_unix, SUM(requests) FROM daily_request_stats WHERE day_start_unix >= ?1 AND day_start_unix < ?2 GROUP BY day_start_unix ORDER BY day_start_unix`（窗口即日边界，直接可比） |
| `request_daily_stats` | 聚合表按日 GROUP BY + LEFT JOIN model_pricing 算费用；空日补 0 / 升序 / 恰好 days 行语义不变 |
| `request_hourly_stats` | **保持明细现算**（今日 24 小时需要小时粒度，聚合表只有天粒度；今日明细在 10000 上限下恒完整——purge 按 id DESC 保留最新，今日新增永不先被删） |
| `request_stats_today` / `request_stats_between` | **不改**（LogsPage 今日分类统计含失败/failover 维度，聚合表不承载；今日明细恒在库内） |
| `last_success_request`、`list_logs`、`clear_logs` | 不改 |

费用计算等价性：现有实现是 `逐行 token × 单价再 SUM`；新实现是 `SUM(token) × 单价`（按模型分组后再 SUM）。乘加线性运算，浮点舍入差异在 1e-6 量级，可接受。

## 不变项

- `request_logs` 保留策略、purge 触发点（写日志后/启动/list_logs）全部不变，明细库体积保持有界。
- 前端四个接口的形状与语义完全不变：`get_request_overview`、`get_request_daily_counts`、`get_timeseries_stats`、`get_request_stats`；StatsCards/Heatmap/StatsChart/HomePage 零改动。
- 事件驱动（stats-changed）+ 5s 轮询 + 恢复可见刷新的刷新链路不动。
- `clear_logs` 只清明细（保留聚合历史）——这是预期行为：清空日志列表不应抹掉累计统计。需在测试中明确此语义。

## 测试计划

新增（log.rs tests + migrate.rs tests）：
1. `migrate_creates_daily_request_stats_table_idempotently` — 建表幂等。
2. `insert_log_accumulates_daily_stats_for_success_only` — 成功累加；404/error 不计。
3. `daily_stats_accumulates_same_day_and_creates_new_day` — 同日多条累加；跨日（手插 time）新建行。
4. `overview_total_survives_purge`（核心回归）— 插入多条 → `purge_expired_logs` 删明细 → `request_overview().total` 不降（等于聚合值）。
5. `backfill_rebuilds_from_details_and_is_idempotent` — 手插明细后新建 Stores（不经过 insert_log）→ 回填正确；再建一次不翻倍。
6. `daily_counts_and_daily_stats_match_details` — 聚合读与明细现算等价（同一批数据两边对账）。
7. `overview_cost_computed_from_pricing` — pricing 行存在时费用正确（token × 单价 / 1e6）。
8. 既有全部测试保持通过（`request_hourly_stats`、`request_stats_between`、purge 系列等）。

## 风险

- 回填缺口不可恢复（已在 PRD 注明，交付时告知用户）。
- JOIN 重复计数隐患（pricing 同时存在短名/长名时翻倍）当前数据未触发，保持现状，不做本次范围外修复。
- 浮点舍入差异（费用等价性）在 1e-6 量级，测试断言用近似比较。
