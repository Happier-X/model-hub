# Implement：按天聚合表修复统计锁死

## 前置

- 当前任务 `stats-daily-aggregate`（planning → start 后实施）。
- 工作区另有 `08-11-pricing-auto-sync-remove-settings`（planning，未开始）——本任务不触碰其文件；若实施期发现其未提交改动涉及相同文件，先 `git stash` 或暂停本任务，勿混改。

## 实施步骤（按序）

### 1. 迁移：建聚合表
- [ ] `src-tauri/src/db/migrate.rs`：新增 `ensure_daily_request_stats_table(conn)`（DDL 见 design.md），在 `migrate()` 中 `ensure_model_pricing_table` 之后调用。
- [ ] 新增测试 `migrate_creates_daily_request_stats_table_idempotently`（migrate 两次不报错、表存在、可插入行）。

### 2. 写入链路：insert_log 同步聚合
- [ ] `src-tauri/src/domain/log.rs`：新增 `fn is_success_log(log: &NewRequestLog) -> bool`（`(200..=299).contains(&log.status_code) && log.error.trim().is_empty()`）。
- [ ] `insert_log` 的 `with_conn` 闭包内、两个 INSERT 分支之后：成功口径时 upsert 当日聚合行（SQL 见 design.md，用 `local_day_start_unix(time)` 分桶，`time` 即本函数内的 `Utc::now().timestamp()`）。
- [ ] `notify_changed()` 位置与行为不变。

### 3. 回填：Stores::new 幂等重建
- [ ] `src-tauri/src/domain/mod.rs`：`Stores::new` 末尾 best-effort 调 `backfill_daily_stats()`（失败 `tracing::warn` 不阻断）。`Stores::new` 签名不变。
- [ ] `src-tauri/src/domain/log.rs`：实现 `backfill_daily_stats`（锁内：表非空即返回 → 事务内 DELETE + 扫描 request_logs 成功行累加 HashMap + 批量 upsert → commit）。
- [ ] 确认所有 `Stores::new` 调用点（runtime.rs `ensure_stores`、commands.rs `stores()`、测试 setup）无需改动即可获得回填。

### 4. 读链路：统计改读聚合表
- [ ] `overview_row(range)`：SQL 改为从 `daily_request_stats s` 聚合 + `LEFT JOIN model_pricing p`（JOIN 条件逐字保持现有写法），`range` 用 `s.day_start_unix >= ?1 AND s.day_start_unix < ?2`；返回结构不变。
- [ ] `request_daily_counts(days)`：改读聚合表（`GROUP BY day_start_unix`，窗口用 start/end 日边界），`start_unix/end_unix` 返回不变。
- [ ] `request_daily_stats(days)`：改读聚合表（按日 GROUP BY + JOIN pricing 算费用），空日补 0、升序、恰好 days 行语义不变。
- [ ] `request_hourly_stats()`、`request_stats_between`、`request_stats_today`、`last_success_request`、`list_logs`、`clear_logs`：不改。

### 5. 测试
- [ ] 新增（见 design.md 测试计划 1-7，落地到 `log.rs` tests / `migrate.rs` tests）：
  - `migrate_creates_daily_request_stats_table_idempotently`
  - `insert_log_accumulates_daily_stats_for_success_only`
  - `daily_stats_accumulates_same_day_and_creates_new_day`
  - `overview_total_survives_purge`（核心回归）
  - `backfill_rebuilds_from_details_and_is_idempotent`
  - `daily_counts_and_daily_stats_match_details`
  - `overview_cost_computed_from_pricing`
- [ ] `cargo test --lib` 全量通过（既有测试不许回归，注意 `request_hourly_stats` / `request_stats_between` 相关用例）。

### 6. 验证（手工 + 构建）
- [ ] `cargo build` 通过。
- [ ] 对真实库做只读对账（不写库）：sqlite3 对 `%APPDATA%/com.modelhub.desktop/gateway/data/data.db` 对比「明细现算 SUM」vs「聚合表预期值」，确认回填结果一致（可先备份 db 文件再跑一次应用验证）。
- [ ] 运行应用：首页总计/费用在发请求期间只增不减；热力图、折线图、日志页正常；重启后数字不回退、不翻倍。
- [ ] 前端零改动确认：`git status` 无 src/ 下前端文件变更。

### 7. 收尾
- [ ] 更新 `.trellis/spec/backend/database-guidelines.md`：新增 `daily_request_stats` 表说明 + 统计语义（统计不随明细裁剪，聚合表只增）。
- [ ] journal 记录 session（根因 + 方案 + 回填缺口说明）。
- [ ] 提交（含任务 artifacts），archive 任务。

## 验证命令

```bash
cd src-tauri && cargo test --lib && cargo build
cd /c/code/model-hub && git status  # 确认无前端改动
```

## 回滚点

- 步骤 1-4 每步完成后 `cargo test --lib` 全绿再进下一步（先测后进）。
- 若 5 出现既有测试回归：回查该步 SQL/逻辑差异，优先恢复该步再排查。
- 若实施中发现聚合与明细对不上：聚合表可整体 `DELETE`（应用重启即回填重建），这是内置的自我修复通道；不要手改明细。

## 完成标准

- 全部 AC（prd.md AC1-AC8）满足：cargo test 全绿、构建通过、真实库对账一致、运行观察数字只增不减。
