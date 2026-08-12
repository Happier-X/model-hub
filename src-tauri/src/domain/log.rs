use rusqlite::params;
use serde::{Deserialize, Serialize};

use crate::error::AppError;

use super::Stores;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestLog {
    pub id: i64,
    pub time: i64,
    pub group_name: String,
    pub provider_name: String,
    pub upstream_model: String,
    pub status_code: i64,
    pub use_time_ms: i64,
    pub error: String,
    pub failover_from: String,
    pub failover_to: String,
    pub failover_reason: String,
}

#[derive(Debug, Clone, Default)]
pub struct NewRequestLog {
    pub group_name: String,
    pub provider_name: String,
    pub upstream_model: String,
    pub status_code: i64,
    pub use_time_ms: i64,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub error: String,
    pub failover_from: String,
    pub failover_to: String,
    pub failover_reason: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct LogQuery {
    pub page: i64,
    pub page_size: i64,
    pub group_name: Option<String>,
    /// all | 2xx | 4xx | 5xx | error
    pub status_class: Option<String>,
    pub failover_only: bool,
}

impl Default for LogQuery {
    fn default() -> Self {
        Self {
            page: 1,
            page_size: 50,
            group_name: None,
            status_class: None,
            failover_only: false,
        }
    }
}

/// 默认保留天数（删除更早的 `time`）。
pub const LOG_RETENTION_DAYS: i64 = 7;

/// 默认保留的最大条数（仅保留最新的 N 条，按 id 倒序）。
pub const LOG_MAX_ROWS: i64 = 10000;

#[derive(Debug, Clone, Serialize)]
pub struct LogPage {
    pub items: Vec<RequestLog>,
    /// 当前筛选条件下的条数
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
    /// 库内日志总条数（未筛选）
    pub stored_total: i64,
    /// 当前保留策略天数
    pub retention_days: i64,
    /// 当前保留策略的最大条数
    pub max_rows: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct LogPurgeResult {
    pub deleted: i64,
    pub retained: i64,
    pub retention_days: i64,
    pub max_rows: i64,
    pub cutoff_unix: i64,
}

/// 按日时间序列统计行（成功口径，含费用）。
#[derive(Debug, Clone, serde::Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct DailyStatRow {
    /// 本地自然日 00:00 的 unix 秒。
    pub day_start_unix: i64,
    pub requests: i64,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cost: f64,
}

/// 按小时时间序列统计行（今日，成功口径，含费用）。
#[derive(Debug, Clone, serde::Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct HourlyStatRow {
    /// 本地小时 0..=23。
    pub hour: i64,
    pub requests: i64,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cost: f64,
}

/// get_timeseries_stats 的聚合响应。
#[derive(Debug, Clone, serde::Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct TimeseriesStats {
    /// 近 30 天（含今日）按日统计，升序。
    pub daily: Vec<DailyStatRow>,
    /// 今日 0..=23 按小时统计（空小时补 0）。
    pub hourly: Vec<HourlyStatRow>,
}

/// success_rows_in_range 的中间行（含单价计算后的费用）。
struct SuccessCostRow {
    time: i64,
    input_tokens: i64,
    output_tokens: i64,
    input_cost: f64,
    output_cost: f64,
}

/// 统计总览单行（总计 / 今日共用）。仅成功请求口径（2xx 且 error 为空）。
#[derive(Debug, Clone, Serialize)]
pub struct OverviewRow {
    pub requests: i64,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub use_time_ms: i64,
    /// 输入 token 费用（美元，按模型单价统计时算）。
    pub input_cost: f64,
    /// 输出 token 费用（美元，按模型单价统计时算）。
    pub output_cost: f64,
    /// 总费用 = 输入 + 输出（美元）。
    pub cost: f64,
}

/// 首页统计总览：总计 + 今日。
#[derive(Debug, Clone, Serialize)]
pub struct RequestOverview {
    pub total: OverviewRow,
    pub today: OverviewRow,
}

fn map_log_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<RequestLog> {
    Ok(RequestLog {
        id: row.get(0)?,
        time: row.get(1)?,
        group_name: row.get(2)?,
        provider_name: row.get(3)?,
        upstream_model: row.get(4)?,
        status_code: row.get(5)?,
        use_time_ms: row.get(6)?,
        error: row.get(7)?,
        failover_from: row.get(8)?,
        failover_to: row.get(9)?,
        failover_reason: row.get(10)?,
    })
}

fn escape_like(raw: &str) -> String {
    raw.replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

/// 返回 (WHERE 子句含 WHERE 或空, 可选 group LIKE 绑定值)。
fn build_filters(query: &LogQuery) -> Result<(String, Option<String>), AppError> {
    let mut clauses: Vec<String> = Vec::new();
    let mut group_like: Option<String> = None;

    if let Some(name) = query
        .group_name
        .as_ref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
    {
        clauses.push("group_name LIKE ?1 ESCAPE '\\'".into());
        group_like = Some(format!("%{}%", escape_like(name)));
    }

    let status = query
        .status_class
        .as_deref()
        .unwrap_or("all")
        .trim()
        .to_ascii_lowercase();
    match status.as_str() {
        "" | "all" => {}
        "2xx" => clauses.push("status_code BETWEEN 200 AND 299".into()),
        "4xx" => clauses.push("status_code BETWEEN 400 AND 499".into()),
        "5xx" => clauses.push("status_code BETWEEN 500 AND 599".into()),
        "error" => {
            clauses.push("(status_code >= 400 OR (error IS NOT NULL AND length(error) > 0))".into())
        }
        other => {
            return Err(AppError::Business(format!(
                "不支持的状态筛选：{other}（可选 all/2xx/4xx/5xx/error）"
            )));
        }
    }

    if query.failover_only {
        clauses.push(
            "((failover_from IS NOT NULL AND length(failover_from) > 0) OR (failover_to IS NOT NULL AND length(failover_to) > 0))"
                .into(),
        );
    }

    let where_sql = if clauses.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", clauses.join(" AND "))
    };
    Ok((where_sql, group_like))
}

fn request_logs_has_column(conn: &rusqlite::Connection, name: &str) -> Result<bool, AppError> {
    conn.query_row(
        "SELECT EXISTS(
            SELECT 1 FROM pragma_table_info('request_logs') WHERE name = ?1
        )",
        params![name],
        |row| row.get(0),
    )
    .map_err(|e| AppError::Database(format!("检查 request_logs.{name} 失败: {e}")))
}

/// 成功口径：2xx 且 error 为空（与所有统计查询的 WHERE 判定逐字等价）。
fn is_success_log(log: &NewRequestLog) -> bool {
    (200..=299).contains(&log.status_code) && log.error.is_empty()
}

impl Stores {
    pub fn insert_log(&self, log: NewRequestLog) -> Result<(), AppError> {
        let time = chrono::Utc::now().timestamp();
        self.with_conn(|conn| {
            // 旧 gateway-rust request_logs 含 request_model_name / channel_name /
            // actual_model_name / use_time 等 NOT NULL 列；CREATE IF NOT EXISTS 与
            // ensure_* 加新列后旧列仍在，必须双写否则 INSERT 失败 → UI 无日志。
            let has_request_model_name = request_logs_has_column(conn, "request_model_name")?;
            let has_channel_name = request_logs_has_column(conn, "channel_name")?;
            let has_actual_model_name = request_logs_has_column(conn, "actual_model_name")?;
            let has_use_time = request_logs_has_column(conn, "use_time")?;

            if has_request_model_name || has_channel_name || has_actual_model_name || has_use_time {
                let mut cols = vec![
                    "time",
                    "group_name",
                    "provider_name",
                    "upstream_model",
                    "status_code",
                    "use_time_ms",
                    "input_tokens",
                    "output_tokens",
                    "error",
                    "failover_from",
                    "failover_to",
                    "failover_reason",
                ];
                let mut placeholders = vec![
                    "?1", "?2", "?3", "?4", "?5", "?6", "?7", "?8", "?9", "?10", "?11", "?12",
                ];
                // 旧列与当前语义映射：
                // request_model_name ← group_name（客户端 model）
                // channel_name ← provider_name
                // actual_model_name ← upstream_model
                // use_time ← use_time_ms
                if has_request_model_name {
                    cols.push("request_model_name");
                    placeholders.push("?2");
                }
                if has_channel_name {
                    cols.push("channel_name");
                    placeholders.push("?3");
                }
                if has_actual_model_name {
                    cols.push("actual_model_name");
                    placeholders.push("?4");
                }
                if has_use_time {
                    cols.push("use_time");
                    placeholders.push("?6");
                }
                let sql = format!(
                    "INSERT INTO request_logs ({}) VALUES ({})",
                    cols.join(", "),
                    placeholders.join(", ")
                );
                conn.execute(
                    &sql,
                    params![
                        time,
                        log.group_name,
                        log.provider_name,
                        log.upstream_model,
                        log.status_code,
                        log.use_time_ms,
                        log.input_tokens,
                        log.output_tokens,
                        log.error,
                        log.failover_from,
                        log.failover_to,
                        log.failover_reason
                    ],
                )
                .map_err(|e| AppError::Database(e.to_string()))?;
            } else {
                conn.execute(
                    "INSERT INTO request_logs
                     (time, group_name, provider_name, upstream_model, status_code, use_time_ms, input_tokens, output_tokens, error, failover_from, failover_to, failover_reason)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                    params![
                        time,
                        log.group_name,
                        log.provider_name,
                        log.upstream_model,
                        log.status_code,
                        log.use_time_ms,
                        log.input_tokens,
                        log.output_tokens,
                        log.error,
                        log.failover_from,
                        log.failover_to,
                        log.failover_reason
                    ],
                )
                .map_err(|e| AppError::Database(e.to_string()))?;
            }
            // 按天聚合（仅成功口径）：统计不再从可裁剪的明细现算；明细与聚合同事务写入，天然一致。
            if is_success_log(&log) {
                let day = local_day_start_unix(time);
                conn.execute(
                    "INSERT INTO daily_request_stats
                     (day_start_unix, model_name, requests, input_tokens, output_tokens, use_time_ms)
                     VALUES (?1, ?2, 1, ?3, ?4, ?5)
                     ON CONFLICT(day_start_unix, model_name) DO UPDATE SET
                       requests = requests + 1,
                       input_tokens = input_tokens + excluded.input_tokens,
                       output_tokens = output_tokens + excluded.output_tokens,
                       use_time_ms = use_time_ms + excluded.use_time_ms",
                    params![
                        day,
                        log.upstream_model,
                        log.input_tokens,
                        log.output_tokens,
                        log.use_time_ms
                    ],
                )
                .map_err(|e| AppError::Database(e.to_string()))?;
            }
            Ok(())
        })?;
        // 写入成功后通知变更订阅（用于实时刷新统计）；失败则不通知。
        self.notify_changed();
        Ok(())
    }

    /// 写库失败只记 tracing，避免吞掉错误后 UI 完全无日志却无法排查。
    pub fn insert_log_best_effort(&self, log: NewRequestLog) {
        if let Err(error) = self.insert_log(log) {
            tracing::warn!(%error, "写入请求日志失败");
        } else {
            // 写入成功后偶尔清理过期，避免仅读库时库无限涨。
            // 每条都 purge 成本低（有索引时 DELETE 很快）；失败忽略。
            self.purge_expired_logs_best_effort();
        }
    }

    pub fn list_logs(&self, query: LogQuery) -> Result<LogPage, AppError> {
        // 列表前尽力清理，保证页上「库内总量」贴近保留策略。
        self.purge_expired_logs_best_effort();
        let page = query.page.max(1);
        let page_size = query.page_size.clamp(1, 100);
        let offset = (page - 1) * page_size;
        let (where_sql, group_like) = build_filters(&query)?;

        self.with_conn(|conn| {
            let total: i64 = if let Some(ref like) = group_like {
                let sql = format!("SELECT COUNT(*) FROM request_logs {where_sql}");
                conn.query_row(&sql, params![like], |row| row.get(0))
                    .map_err(|e| AppError::Database(e.to_string()))?
            } else {
                let sql = format!("SELECT COUNT(*) FROM request_logs {where_sql}");
                conn.query_row(&sql, [], |row| row.get(0))
                    .map_err(|e| AppError::Database(e.to_string()))?
            };

            let select = format!(
                "SELECT id, time, group_name, provider_name, upstream_model, status_code, use_time_ms, error, failover_from, failover_to, failover_reason
                 FROM request_logs {where_sql}
                 ORDER BY id DESC LIMIT ? OFFSET ?"
            );

            let mut items = Vec::new();
            if let Some(ref like) = group_like {
                // ?1 = like, ?2 = limit, ?3 = offset
                let sql = select.replace(
                    "LIMIT ? OFFSET ?",
                    "LIMIT ?2 OFFSET ?3",
                );
                // group filter already uses ?1
                let mut stmt = conn
                    .prepare(&sql)
                    .map_err(|e| AppError::Database(e.to_string()))?;
                let rows = stmt
                    .query_map(params![like, page_size, offset], map_log_row)
                    .map_err(|e| AppError::Database(e.to_string()))?;
                for r in rows {
                    items.push(r.map_err(|e| AppError::Database(e.to_string()))?);
                }
            } else {
                let sql = select.replace("LIMIT ? OFFSET ?", "LIMIT ?1 OFFSET ?2");
                let mut stmt = conn
                    .prepare(&sql)
                    .map_err(|e| AppError::Database(e.to_string()))?;
                let rows = stmt
                    .query_map(params![page_size, offset], map_log_row)
                    .map_err(|e| AppError::Database(e.to_string()))?;
                for r in rows {
                    items.push(r.map_err(|e| AppError::Database(e.to_string()))?);
                }
            }

            let stored_total: i64 = conn
                .query_row("SELECT COUNT(*) FROM request_logs", [], |row| row.get(0))
                .map_err(|e| AppError::Database(e.to_string()))?;

            Ok(LogPage {
                items,
                total,
                page,
                page_size,
                stored_total,
                retention_days: LOG_RETENTION_DAYS,
                max_rows: LOG_MAX_ROWS,
            })
        })
    }

    pub fn clear_logs(&self) -> Result<(), AppError> {
        self.with_conn(|conn| {
            conn.execute("DELETE FROM request_logs", [])
                .map_err(|e| AppError::Database(e.to_string()))?;
            Ok(())
        })
    }

    /// 按默认策略清理：删除超过 `LOG_RETENTION_DAYS` 天，或超出最新 `LOG_MAX_ROWS` 条的行。
    /// 时间窗口与条数上限同时生效，满足任一淘汰条件的记录都会被删除。
    pub fn purge_expired_logs(&self) -> Result<LogPurgeResult, AppError> {
        self.purge_logs(LOG_RETENTION_DAYS, LOG_MAX_ROWS)
    }

    /// 仅按时间清理（条数不限）；保留供测试与按天调用。
    pub fn purge_logs_older_than_days(
        &self,
        retention_days: i64,
    ) -> Result<LogPurgeResult, AppError> {
        self.purge_logs(retention_days, i64::MAX)
    }

    /// 组合清理：先删超时行，再删超出最新 `max_rows` 的行。
    pub fn purge_logs(
        &self,
        retention_days: i64,
        max_rows: i64,
    ) -> Result<LogPurgeResult, AppError> {
        let days = retention_days.max(1);
        let rows = max_rows.max(1);
        let now = chrono::Utc::now().timestamp();
        let cutoff = now.saturating_sub(days.saturating_mul(86_400));
        self.with_conn(|conn| {
            // 1) 超过时间窗口的行。
            let deleted_by_time =
                conn.execute("DELETE FROM request_logs WHERE time < ?1", params![cutoff])
                    .map_err(|e| AppError::Database(e.to_string()))? as i64;
            // 2) 时间窗口内仍超出最新 rows 条的行（按 id 倒序保留最新）。
            let deleted_by_rows =
                conn.execute(
                    "DELETE FROM request_logs WHERE id NOT IN (
                        SELECT id FROM request_logs ORDER BY id DESC LIMIT ?1
                    )",
                    params![rows],
                )
                .map_err(|e| AppError::Database(e.to_string()))? as i64;
            let retained: i64 = conn
                .query_row("SELECT COUNT(*) FROM request_logs", [], |row| row.get(0))
                .map_err(|e| AppError::Database(e.to_string()))?;
            Ok(LogPurgeResult {
                deleted: deleted_by_time + deleted_by_rows,
                retained,
                retention_days: days,
                max_rows: rows,
                cutoff_unix: cutoff,
            })
        })
    }

    /// 自动清理失败不阻断主路径。
    pub fn purge_expired_logs_best_effort(&self) {
        if let Err(error) = self.purge_expired_logs() {
            tracing::warn!(%error, "自动清理过期请求日志失败");
        }
    }

    /// 幂等回填聚合表：仅当 `daily_request_stats` 为空时执行（锁内：清空 → 扫描
    /// `request_logs` 成功口径行重建）。失败整体回滚，表保持空，下次启动重试。
    /// 旧库升级后首次启动由 `Stores::new` 调用。
    pub fn backfill_daily_stats(&self) -> Result<(), AppError> {
        self.with_conn(|conn| {
            let count: i64 = conn
                .query_row("SELECT COUNT(*) FROM daily_request_stats", [], |row| row.get(0))
                .map_err(|e| AppError::Database(e.to_string()))?;
            if count > 0 {
                return Ok(());
            }
            // 先在锁内把明细（成功口径）聚合到内存，再开短事务写入，失败整体回滚。
            let mut acc: std::collections::HashMap<(i64, String), (i64, i64, i64, i64)> =
                std::collections::HashMap::new();
            {
                let mut stmt = conn
                    .prepare(
                        "SELECT time, upstream_model, input_tokens, output_tokens, use_time_ms
                         FROM request_logs
                         WHERE status_code BETWEEN 200 AND 299
                           AND (error IS NULL OR length(error) = 0)",
                    )
                    .map_err(|e| AppError::Database(e.to_string()))?;
                let rows = stmt
                    .query_map([], |row| {
                        Ok((
                            row.get::<_, i64>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, i64>(2)?,
                            row.get::<_, i64>(3)?,
                            row.get::<_, i64>(4)?,
                        ))
                    })
                    .map_err(|e| AppError::Database(e.to_string()))?;
                for r in rows {
                    let (time, model, input, output, use_time) =
                        r.map_err(|e| AppError::Database(e.to_string()))?;
                    let day = local_day_start_unix(time);
                    let entry = acc.entry((day, model)).or_insert((0, 0, 0, 0));
                    entry.0 += 1;
                    entry.1 += input;
                    entry.2 += output;
                    entry.3 += use_time;
                }
            }
            let tx = conn
                .unchecked_transaction()
                .map_err(|e| AppError::Database(e.to_string()))?;
            // 清掉极端并发下已写入的行，保证重建干净（表空检查与重建在同一把锁内）。
            tx.execute("DELETE FROM daily_request_stats", [])
                .map_err(|e| AppError::Database(e.to_string()))?;
            for ((day, model), (requests, input, output, use_time)) in acc {
                tx.execute(
                    "INSERT OR REPLACE INTO daily_request_stats
                     (day_start_unix, model_name, requests, input_tokens, output_tokens, use_time_ms)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    params![day, model, requests, input, output, use_time],
                )
                .map_err(|e| AppError::Database(e.to_string()))?;
            }
            tx.commit()
                .map_err(|e| AppError::Database(e.to_string()))
        })
    }

    /// 本地自然日 00:00（含）至次日 00:00（不含）的请求聚合。
    pub fn request_stats_today(&self) -> Result<RequestStats, AppError> {
        let (start_ts, end_ts) = local_day_bounds_unix();
        self.request_stats_between(start_ts, end_ts)
    }

    /// 首页统计总览：总计 + 今日（仅成功请求：2xx 且 error 为空）。
    /// 费用按 `model_pricing` 单价现算（token × 单价 / 1e6）。
    pub fn request_overview(&self) -> Result<RequestOverview, AppError> {
        let (start_ts, end_ts) = local_day_bounds_unix();
        let total = self.overview_row(None)?;
        let today = self.overview_row(Some((start_ts, end_ts)))?;
        Ok(RequestOverview { total, today })
    }

    fn overview_row(&self, range: Option<(i64, i64)>) -> Result<OverviewRow, AppError> {
        // 从按天聚合表读取：不再受 request_logs 保留策略（7 天/1 万条）裁剪影响，
        // 费用按模型单价现算（与既有口径一致：token × 单价 / 1e6）。
        let mut sql = String::from(
            "SELECT
                COALESCE(SUM(s.requests), 0),
                COALESCE(SUM(s.input_tokens), 0),
                COALESCE(SUM(s.output_tokens), 0),
                COALESCE(SUM(s.use_time_ms), 0),
                COALESCE(SUM(CAST(s.input_tokens AS REAL) * COALESCE(p.prompt_price_per_mtok, 0) / 1000000.0), 0),
                COALESCE(SUM(CAST(s.output_tokens AS REAL) * COALESCE(p.completion_price_per_mtok, 0) / 1000000.0), 0)
             FROM daily_request_stats s
             LEFT JOIN model_pricing p
               ON s.model_name = p.model_name
               OR p.model_name LIKE '%/' || s.model_name",
        );
        let mut params: Vec<rusqlite::types::Value> = Vec::new();
        if let Some((start_ts, end_ts)) = range {
            sql.push_str(" WHERE s.day_start_unix >= ?1 AND s.day_start_unix < ?2");
            params.push(rusqlite::types::Value::Integer(start_ts));
            params.push(rusqlite::types::Value::Integer(end_ts));
        }
        self.with_conn(|conn| {
            conn.query_row(&sql, rusqlite::params_from_iter(params), |row| {
                let input_cost: f64 = row.get(4)?;
                let output_cost: f64 = row.get(5)?;
                Ok(OverviewRow {
                    requests: row.get(0)?,
                    input_tokens: row.get(1)?,
                    output_tokens: row.get(2)?,
                    use_time_ms: row.get(3)?,
                    input_cost,
                    output_cost,
                    cost: input_cost + output_cost,
                })
            })
            .map_err(|e| AppError::Database(e.to_string()))
        })
    }

    pub fn request_stats_between(
        &self,
        start_ts: i64,
        end_ts: i64,
    ) -> Result<RequestStats, AppError> {
        self.with_conn(|conn| {
            conn.query_row(
                "SELECT
                    COUNT(*) AS total,
                    COALESCE(SUM(CASE WHEN status_code BETWEEN 200 AND 299 AND (error IS NULL OR length(error) = 0) THEN 1 ELSE 0 END), 0) AS success,
                    COALESCE(SUM(CASE WHEN status_code >= 400 OR (error IS NOT NULL AND length(error) > 0) THEN 1 ELSE 0 END), 0) AS failure,
                    COALESCE(SUM(CASE WHEN (failover_from IS NOT NULL AND length(failover_from) > 0)
                        OR (failover_to IS NOT NULL AND length(failover_to) > 0) THEN 1 ELSE 0 END), 0) AS failover
                 FROM request_logs
                 WHERE time >= ?1 AND time < ?2",
                params![start_ts, end_ts],
                |row| {
                    Ok(RequestStats {
                        total: row.get(0)?,
                        success: row.get(1)?,
                        failure: row.get(2)?,
                        failover: row.get(3)?,
                        day_start_unix: start_ts,
                        day_end_unix: end_ts,
                    })
                },
            )
            .map_err(|e| AppError::Database(e.to_string()))
        })
    }

    /// 全局最近一条成功请求（2xx 且 error 为空）；无则 `None`。
    pub fn last_success_request(&self) -> Result<Option<LastSuccessRequest>, AppError> {
        self.with_conn(|conn| {
            use rusqlite::OptionalExtension;
            conn.query_row(
                "SELECT time, group_name, provider_name, upstream_model, status_code
                 FROM request_logs
                 WHERE status_code BETWEEN 200 AND 299
                   AND (error IS NULL OR length(error) = 0)
                 ORDER BY time DESC, id DESC
                 LIMIT 1",
                [],
                |row| {
                    Ok(LastSuccessRequest {
                        time: row.get(0)?,
                        group_name: row.get(1)?,
                        provider_name: row.get(2)?,
                        upstream_model: row.get(3)?,
                        status_code: row.get(4)?,
                    })
                },
            )
            .optional()
            .map_err(|e| AppError::Database(e.to_string()))
        })
    }

    /// 按本地自然日聚合过去 `days` 天（含今日）的**成功请求**（2xx 且 error 为空）总量。
    /// 仅返回 `count > 0` 的日期，按 `day_start_unix` 升序。全项目统计统一按成功口径。
    pub fn request_daily_counts(&self, days: u32) -> Result<RequestDailyCounts, AppError> {
        let days = days.clamp(1, DAILY_COUNTS_MAX_DAYS);
        let (start_unix, end_unix) = daily_window_bounds(days);
        // 读聚合表：窗口即本地日边界，直接与 day_start_unix 比较；不受明细保留策略裁剪影响。
        self.with_conn(|conn| {
            let mut stmt = conn
                .prepare(
                    "SELECT day_start_unix, SUM(requests) AS requests
                     FROM daily_request_stats
                     WHERE day_start_unix >= ?1 AND day_start_unix < ?2
                     GROUP BY day_start_unix
                     ORDER BY day_start_unix",
                )
                .map_err(|e| AppError::Database(e.to_string()))?;
            let rows = stmt
                .query_map(params![start_unix, end_unix], |row| {
                    Ok(DailyCount {
                        day_start_unix: row.get(0)?,
                        count: row.get(1)?,
                    })
                })
                .map_err(|e| AppError::Database(e.to_string()))?;
            let mut days_out: Vec<DailyCount> = Vec::new();
            for r in rows {
                days_out.push(r.map_err(|e| AppError::Database(e.to_string()))?);
            }
            Ok(RequestDailyCounts {
                days: days_out,
                start_unix,
                end_unix,
            })
        })
    }

    /// 近 `days` 天（含今日）的**成功请求**按日时间序列（含 token 与费用，升序）。
    /// 空日补 0，保证返回恰好 `days` 行，前端可直接画图。
    pub fn request_daily_stats(&self, days: u32) -> Result<Vec<DailyStatRow>, AppError> {
        let days = days.clamp(1, DAILY_COUNTS_MAX_DAYS);
        let (start_unix, end_unix) = daily_window_bounds(days);
        // 读聚合表（已按日+模型聚合），费用按当前单价现算；不受明细保留策略裁剪影响。
        let rows = self.daily_stats_in_range(start_unix, end_unix)?;
        // 同一天可能有多模型行，合并累加。
        let mut buckets: std::collections::HashMap<i64, DailyStatRow> =
            std::collections::HashMap::new();
        for r in rows {
            let bucket = r.day_start_unix;
            let entry = buckets.entry(bucket).or_insert(DailyStatRow {
                day_start_unix: bucket,
                requests: 0,
                input_tokens: 0,
                output_tokens: 0,
                cost: 0.0,
            });
            entry.requests += r.requests;
            entry.input_tokens += r.input_tokens;
            entry.output_tokens += r.output_tokens;
            entry.cost += r.cost;
        }
        // 补全空日（含今日），按日升序。
        let mut out: Vec<DailyStatRow> = Vec::with_capacity(days as usize);
        let mut day = start_unix;
        while day < end_unix {
            out.push(buckets.remove(&day).unwrap_or(DailyStatRow {
                day_start_unix: day,
                requests: 0,
                input_tokens: 0,
                output_tokens: 0,
                cost: 0.0,
            }));
            day += 86_400;
        }
        Ok(out)
    }

    /// 按日聚合行（含费用，按模型单价现算），升序。
    fn daily_stats_in_range(
        &self,
        start_unix: i64,
        end_unix: i64,
    ) -> Result<Vec<DailyStatRow>, AppError> {
        self.with_conn(|conn| {
            let mut stmt = conn
                .prepare(
                    "SELECT s.day_start_unix, s.requests, s.input_tokens, s.output_tokens,
                            CAST(s.input_tokens AS REAL) * COALESCE(p.prompt_price_per_mtok, 0) / 1000000.0,
                            CAST(s.output_tokens AS REAL) * COALESCE(p.completion_price_per_mtok, 0) / 1000000.0
                     FROM daily_request_stats s
                     LEFT JOIN model_pricing p
                       ON s.model_name = p.model_name
                       OR p.model_name LIKE '%/' || s.model_name
                     WHERE s.day_start_unix >= ?1 AND s.day_start_unix < ?2
                     ORDER BY s.day_start_unix",
                )
                .map_err(|e| AppError::Database(e.to_string()))?;
            let rows = stmt
                .query_map(params![start_unix, end_unix], |row| {
                    let input_cost: f64 = row.get(4)?;
                    let output_cost: f64 = row.get(5)?;
                    Ok(DailyStatRow {
                        day_start_unix: row.get(0)?,
                        requests: row.get(1)?,
                        input_tokens: row.get(2)?,
                        output_tokens: row.get(3)?,
                        cost: input_cost + output_cost,
                    })
                })
                .map_err(|e| AppError::Database(e.to_string()))?;
            rows.collect::<Result<Vec<_>, _>>()
                .map_err(|e| AppError::Database(e.to_string()))
        })
    }

    /// 今日 0..=23 的**成功请求**按小时时间序列（含 token 与费用）。
    /// 空小时补 0，返回恰好 24 行。
    pub fn request_hourly_stats(&self) -> Result<Vec<HourlyStatRow>, AppError> {
        let (today_start, tomorrow_start) = local_day_bounds_unix();
        let rows = self.success_rows_in_range(today_start, tomorrow_start)?;
        let mut buckets: [HourlyStatRow; 24] = std::array::from_fn(|hour| HourlyStatRow {
            hour: hour as i64,
            requests: 0,
            input_tokens: 0,
            output_tokens: 0,
            cost: 0.0,
        });
        for r in rows {
            let hour = ((r.time - today_start) / 3600).clamp(0, 23) as usize;
            buckets[hour].requests += 1;
            buckets[hour].input_tokens += r.input_tokens;
            buckets[hour].output_tokens += r.output_tokens;
            buckets[hour].cost += r.input_cost + r.output_cost;
        }
        Ok(buckets.to_vec())
    }

    /// 查询成功请求（2xx 且 error 为空）在范围内的原始行（含 token 与单价费用），供时间序列聚合复用。
    fn success_rows_in_range(
        &self,
        start_unix: i64,
        end_unix: i64,
    ) -> Result<Vec<SuccessCostRow>, AppError> {
        self.with_conn(|conn| {
            let mut stmt = conn
                .prepare(
                    "SELECT l.time, l.input_tokens, l.output_tokens,
                            CAST(l.input_tokens AS REAL) * COALESCE(p.prompt_price_per_mtok, 0) / 1000000.0,
                            CAST(l.output_tokens AS REAL) * COALESCE(p.completion_price_per_mtok, 0) / 1000000.0
                     FROM request_logs l
                     LEFT JOIN model_pricing p
                       ON l.upstream_model = p.model_name
                       OR p.model_name LIKE '%/' || l.upstream_model
                     WHERE l.status_code BETWEEN 200 AND 299
                       AND (l.error IS NULL OR length(l.error) = 0)
                       AND l.time >= ?1 AND l.time < ?2",
                )
                .map_err(|e| AppError::Database(e.to_string()))?;
            let rows = stmt
                .query_map(params![start_unix, end_unix], |row| {
                    Ok(SuccessCostRow {
                        time: row.get(0)?,
                        input_tokens: row.get(1)?,
                        output_tokens: row.get(2)?,
                        input_cost: row.get(3)?,
                        output_cost: row.get(4)?,
                    })
                })
                .map_err(|e| AppError::Database(e.to_string()))?;
            rows.collect::<Result<Vec<_>, _>>()
                .map_err(|e| AppError::Database(e.to_string()))
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LastSuccessRequest {
    pub time: i64,
    pub group_name: String,
    pub provider_name: String,
    pub upstream_model: String,
    pub status_code: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct RequestStats {
    pub total: i64,
    pub success: i64,
    pub failure: i64,
    pub failover: i64,
    /// 统计窗口起点（本地日 00:00，unix 秒）
    pub day_start_unix: i64,
    pub day_end_unix: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct DailyCount {
    /// 本地自然日 00:00 的 unix 秒
    pub day_start_unix: i64,
    pub count: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct RequestDailyCounts {
    /// 仅含 `count > 0` 的日期，升序
    pub days: Vec<DailyCount>,
    /// 窗口首日 00:00 的 unix 秒
    pub start_unix: i64,
    /// 今日次日 00:00 的 unix 秒（半开区间）
    pub end_unix: i64,
}

/// 每日计数默认窗口（天）：约 12 个自然月。
pub const DAILY_COUNTS_DEFAULT_DAYS: u32 = 365;
/// 每日计数允许的最大窗口（天），防一次拉太多桶。
pub const DAILY_COUNTS_MAX_DAYS: u32 = 400;

fn local_day_bounds_unix() -> (i64, i64) {
    use chrono::{Local, TimeZone};
    let today = Local::now().date_naive();
    let tomorrow = today.succ_opt().expect("date overflow");
    let start = today.and_hms_opt(0, 0, 0).expect("midnight");
    let end = tomorrow.and_hms_opt(0, 0, 0).expect("midnight");
    let start_ts = Local
        .from_local_datetime(&start)
        .single()
        .map(|dt| dt.timestamp())
        .unwrap_or_else(|| start.and_utc().timestamp());
    let end_ts = Local
        .from_local_datetime(&end)
        .single()
        .map(|dt| dt.timestamp())
        .unwrap_or_else(|| end.and_utc().timestamp());
    (start_ts, end_ts)
}

/// 把任意 unix 秒归一化到「所在本地自然日 00:00」的 unix 秒。
fn local_day_start_unix(ts: i64) -> i64 {
    use chrono::{Local, TimeZone};
    let Some(dt) = Local.timestamp_opt(ts, 0).single() else {
        return ts;
    };
    let day = dt.date_naive();
    let midnight = match day.and_hms_opt(0, 0, 0) {
        Some(m) => m,
        None => return ts,
    };
    Local
        .from_local_datetime(&midnight)
        .single()
        .map(|d| d.timestamp())
        .unwrap_or(ts)
}

/// 按本地自然日算出 `[days-1 天前 00:00, 今日次日 00:00)` 半开区间。
fn daily_window_bounds(days: u32) -> (i64, i64) {
    use chrono::{Duration, Local, TimeZone};
    let (today_start, tomorrow_start) = local_day_bounds_unix();
    let back = days.saturating_sub(1) as i64;
    let start_unix = Local
        .timestamp_opt(today_start, 0)
        .single()
        .map(|dt| dt.date_naive() - Duration::days(back))
        .and_then(|d| d.and_hms_opt(0, 0, 0))
        .and_then(|nd| Local.from_local_datetime(&nd).single())
        .map(|dt| dt.timestamp())
        .unwrap_or(today_start);
    (start_unix, tomorrow_start)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::open_db;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use tempfile::tempdir;

    fn setup() -> (tempfile::TempDir, Stores) {
        let dir = tempdir().unwrap();
        let db = open_db(&dir.path().join("t.db")).unwrap();
        (dir, Stores::new(db))
    }

    fn seed(stores: &Stores, group: &str, status: i64, err: &str, fo_from: &str, fo_to: &str) {
        stores
            .insert_log(NewRequestLog {
                group_name: group.into(),
                provider_name: "p".into(),
                upstream_model: "m".into(),
                status_code: status,
                use_time_ms: 1,
                error: err.into(),
                failover_from: fo_from.into(),
                failover_to: fo_to.into(),
                failover_reason: if fo_from.is_empty() {
                    String::new()
                } else {
                    "5xx".into()
                },
                input_tokens: 0,
                output_tokens: 0,
            })
            .unwrap();
    }

    #[test]
    fn pagination_total_and_slice() {
        let (_dir, stores) = setup();
        for i in 0..5 {
            seed(&stores, &format!("g{i}"), 200, "", "", "");
        }
        let page1 = stores
            .list_logs(LogQuery {
                page: 1,
                page_size: 2,
                ..Default::default()
            })
            .unwrap();
        assert_eq!(page1.total, 5);
        assert_eq!(page1.items.len(), 2);
        assert_eq!(page1.page, 1);
        assert_eq!(page1.page_size, 2);

        let page3 = stores
            .list_logs(LogQuery {
                page: 3,
                page_size: 2,
                ..Default::default()
            })
            .unwrap();
        assert_eq!(page3.items.len(), 1);
    }

    #[test]
    fn filter_group_name_substring() {
        let (_dir, stores) = setup();
        seed(&stores, "alpha-prod", 200, "", "", "");
        seed(&stores, "beta", 200, "", "", "");
        let page = stores
            .list_logs(LogQuery {
                group_name: Some("alpha".into()),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(page.total, 1);
        assert_eq!(page.items[0].group_name, "alpha-prod");
    }

    #[test]
    fn filter_status_classes() {
        let (_dir, stores) = setup();
        seed(&stores, "g", 200, "", "", "");
        seed(&stores, "g", 404, "no", "", "");
        seed(&stores, "g", 502, "bad", "", "");
        let s2 = stores
            .list_logs(LogQuery {
                status_class: Some("2xx".into()),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(s2.total, 1);
        let s5 = stores
            .list_logs(LogQuery {
                status_class: Some("5xx".into()),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(s5.total, 1);
        let err = stores
            .list_logs(LogQuery {
                status_class: Some("error".into()),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(err.total, 2);
    }

    #[test]
    fn filter_failover_only() {
        let (_dir, stores) = setup();
        seed(&stores, "g", 200, "", "", "");
        seed(&stores, "g", 200, "", "a", "b");
        let page = stores
            .list_logs(LogQuery {
                failover_only: true,
                ..Default::default()
            })
            .unwrap();
        assert_eq!(page.total, 1);
        assert_eq!(page.items[0].failover_from, "a");
    }

    #[test]
    fn insert_log_notifies_change_subscribers() {
        let (_dir, stores) = setup();
        let calls = Arc::new(AtomicUsize::new(0));
        let c = calls.clone();
        stores.subscribe_change(move || {
            c.fetch_add(1, Ordering::Relaxed);
        });
        // 无订阅时写入正常（现有测试已覆盖）；订阅后每次写入成功触发一次回调。
        seed(&stores, "g", 200, "", "", "");
        seed(&stores, "g", 404, "no", "", "");
        assert_eq!(calls.load(Ordering::Relaxed), 2);
    }

    fn insert_at(stores: &Stores, time: i64, status: i64, err: &str, fo_from: &str, fo_to: &str) {
        insert_at_ex(stores, time, status, err, fo_from, fo_to, "m", 10, 20);
    }

    /// 带 token 与模型名的插入（时间序列统计测试用）。
    fn insert_at_ex(
        stores: &Stores,
        time: i64,
        status: i64,
        err: &str,
        fo_from: &str,
        fo_to: &str,
        model: &str,
        input_tokens: i64,
        output_tokens: i64,
    ) {
        stores
            .with_conn(|conn| {
                conn.execute(
                    "INSERT INTO request_logs
                     (time, group_name, provider_name, upstream_model, status_code, use_time_ms, error, failover_from, failover_to, failover_reason, input_tokens, output_tokens)
                     VALUES (?1, 'g', 'p', ?6, ?2, 1, ?3, ?4, ?5, '', ?7, ?8)",
                    params![time, status, err, fo_from, fo_to, model, input_tokens, output_tokens],
                )
                .map_err(|e| AppError::Database(e.to_string()))?;
                Ok(())
            })
            .unwrap();
    }

    /// 直接写聚合表一行（统计读聚合表后的事实来源；测试构造数据用）。
    fn insert_stats_day(
        stores: &Stores,
        day_start_unix: i64,
        model: &str,
        requests: i64,
        input_tokens: i64,
        output_tokens: i64,
        use_time_ms: i64,
    ) {
        stores
            .with_conn(|conn| {
                conn.execute(
                    "INSERT INTO daily_request_stats
                     (day_start_unix, model_name, requests, input_tokens, output_tokens, use_time_ms)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    params![
                        day_start_unix,
                        model,
                        requests,
                        input_tokens,
                        output_tokens,
                        use_time_ms
                    ],
                )
                .map_err(|e| AppError::Database(e.to_string()))?;
                Ok(())
            })
            .unwrap();
    }

    #[test]
    fn insert_log_dual_writes_legacy_request_model_name_columns() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("legacy-logs.db");
        {
            let conn = rusqlite::Connection::open(&path).unwrap();
            conn.execute_batch(
                "CREATE TABLE request_logs (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    time INTEGER NOT NULL,
                    request_model_name TEXT NOT NULL,
                    channel_name TEXT NOT NULL DEFAULT '',
                    actual_model_name TEXT NOT NULL DEFAULT '',
                    input_tokens INTEGER NOT NULL DEFAULT 0,
                    output_tokens INTEGER NOT NULL DEFAULT 0,
                    use_time INTEGER NOT NULL DEFAULT 0,
                    cost REAL NOT NULL DEFAULT 0,
                    error TEXT NOT NULL DEFAULT ''
                );",
            )
            .unwrap();
        }
        let stores = Stores::new(open_db(&path).unwrap());
        stores
            .insert_log(NewRequestLog {
                group_name: "g1".into(),
                provider_name: "p1".into(),
                upstream_model: "m1".into(),
                status_code: 200,
                use_time_ms: 12,
                error: String::new(),
                failover_from: String::new(),
                failover_to: String::new(),
                failover_reason: String::new(),
                input_tokens: 0,
                output_tokens: 0,
            })
            .expect("legacy NOT NULL 列应被双写");

        let page = stores.list_logs(LogQuery::default()).unwrap();
        assert_eq!(page.total, 1);
        assert_eq!(page.items[0].group_name, "g1");
        assert_eq!(page.items[0].provider_name, "p1");
        assert_eq!(page.items[0].upstream_model, "m1");
        assert_eq!(page.items[0].status_code, 200);

        stores
            .with_conn(|conn| {
                let legacy: (String, String, String, i64) = conn
                    .query_row(
                        "SELECT request_model_name, channel_name, actual_model_name, use_time
                         FROM request_logs WHERE id = 1",
                        [],
                        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
                    )
                    .map_err(|e| AppError::Database(e.to_string()))?;
                assert_eq!(legacy, ("g1".into(), "p1".into(), "m1".into(), 12));
                Ok(())
            })
            .unwrap();
    }

    #[test]
    fn purge_expired_keeps_recent_deletes_old() {
        let (_dir, stores) = setup();
        let now = chrono::Utc::now().timestamp();
        let old = now - 40 * 86_400;
        let recent = now - 2 * 86_400;
        insert_at(&stores, old, 200, "", "", "");
        insert_at(&stores, recent, 200, "", "", "");

        let result = stores.purge_logs_older_than_days(30).unwrap();
        assert_eq!(result.deleted, 1);
        assert_eq!(result.retained, 1);
        assert_eq!(result.retention_days, 30);
        assert_eq!(result.max_rows, i64::MAX);
        assert!(result.cutoff_unix <= now - 30 * 86_400 + 5);

        // 再 purge 不应再删
        let again = stores.purge_logs_older_than_days(30).unwrap();
        assert_eq!(again.deleted, 0);
        assert_eq!(again.retained, 1);

        let page = stores.list_logs(LogQuery::default()).unwrap();
        assert_eq!(page.stored_total, 1);
        assert_eq!(page.retention_days, LOG_RETENTION_DAYS);
        assert_eq!(page.max_rows, LOG_MAX_ROWS);
    }

    #[test]
    fn purge_logs_enforces_time_and_row_limits() {
        let (_dir, stores) = setup();
        let now = chrono::Utc::now().timestamp();
        insert_at(&stores, now - 8 * 86_400, 200, "", "", "");
        for _ in 0..1002 {
            insert_at(&stores, now, 200, "", "", "");
        }

        let result = stores.purge_logs(7, 1000).unwrap();
        assert_eq!(result.deleted, 3);
        assert_eq!(result.retained, 1000);
        assert_eq!(result.retention_days, 7);
        assert_eq!(result.max_rows, 1000);

        stores
            .with_conn(|conn| {
                let (count, oldest_time): (i64, i64) = conn
                    .query_row("SELECT COUNT(*), MIN(time) FROM request_logs", [], |row| {
                        Ok((row.get(0)?, row.get(1)?))
                    })
                    .map_err(|e| AppError::Database(e.to_string()))?;
                assert_eq!(count, 1000);
                assert_eq!(oldest_time, now);
                Ok(())
            })
            .unwrap();
    }

    #[test]
    fn default_purge_uses_seven_days_and_default_row_limit() {
        let (_dir, stores) = setup();
        let now = chrono::Utc::now().timestamp();
        insert_at(&stores, now - 8 * 86_400, 200, "", "", "");
        for _ in 0..10001 {
            insert_at(&stores, now, 200, "", "", "");
        }

        let result = stores.purge_expired_logs().unwrap();
        assert_eq!(result.deleted, 2);
        assert_eq!(result.retained, LOG_MAX_ROWS);
        assert_eq!(result.retention_days, LOG_RETENTION_DAYS);
        assert_eq!(result.max_rows, LOG_MAX_ROWS);
    }

    #[test]
    fn request_stats_classifies_and_windows() {
        let (_dir, stores) = setup();
        let (start, end) = super::local_day_bounds_unix();
        // 窗口内：成功、失败、故障转移成功
        insert_at(&stores, start + 10, 200, "", "", "");
        insert_at(&stores, start + 20, 502, "bad", "", "");
        insert_at(&stores, start + 30, 200, "", "a", "b");
        // 窗口外：昨日
        insert_at(&stores, start - 100, 200, "", "", "");
        // 窗口外：明日
        insert_at(&stores, end + 10, 500, "x", "", "");

        let stats = stores.request_stats_between(start, end).unwrap();
        assert_eq!(stats.total, 3);
        assert_eq!(stats.success, 2); // 两条 200 且无 error
        assert_eq!(stats.failure, 1);
        assert_eq!(stats.failover, 1);
        assert_eq!(stats.day_start_unix, start);
        assert_eq!(stats.day_end_unix, end);

        let empty = stores.request_stats_between(end, end + 1).unwrap();
        assert_eq!(empty.total, 0);
        assert_eq!(empty.success, 0);
        assert_eq!(empty.failure, 0);
        assert_eq!(empty.failover, 0);
    }

    #[test]
    fn last_success_returns_newest_success() {
        let (_dir, stores) = setup();
        let now = chrono::Utc::now().timestamp();
        // 较旧的成功
        stores
            .with_conn(|conn| {
                conn.execute(
                    "INSERT INTO request_logs
                     (time, group_name, provider_name, upstream_model, status_code, use_time_ms, error, failover_from, failover_to, failover_reason)
                     VALUES (?1, 'g-old', 'p-old', 'm-old', 200, 1, '', '', '', '')",
                    params![now - 100],
                )
                .map_err(|e| AppError::Database(e.to_string()))?;
                Ok(())
            })
            .unwrap();
        // 失败与 2xx 但有 error（不算成功）
        insert_at(&stores, now - 10, 502, "bad", "", "");
        insert_at(&stores, now - 5, 200, "structured", "", "");
        // 最新成功
        stores
            .with_conn(|conn| {
                conn.execute(
                    "INSERT INTO request_logs
                     (time, group_name, provider_name, upstream_model, status_code, use_time_ms, error, failover_from, failover_to, failover_reason)
                     VALUES (?1, 'g-new', 'p-new', 'm-new', 201, 2, '', '', '', '')",
                    params![now],
                )
                .map_err(|e| AppError::Database(e.to_string()))?;
                Ok(())
            })
            .unwrap();

        let last = stores
            .last_success_request()
            .unwrap()
            .expect("应有成功记录");
        assert_eq!(last.group_name, "g-new");
        assert_eq!(last.provider_name, "p-new");
        assert_eq!(last.upstream_model, "m-new");
        assert_eq!(last.status_code, 201);
        assert_eq!(last.time, now);
    }

    #[test]
    fn last_success_none_when_only_failures() {
        let (_dir, stores) = setup();
        let now = chrono::Utc::now().timestamp();
        insert_at(&stores, now - 1, 500, "err", "", "");
        insert_at(&stores, now, 200, "has-error", "", "");
        assert!(stores.last_success_request().unwrap().is_none());
    }

    #[test]
    fn last_success_none_when_empty() {
        let (_dir, stores) = setup();
        assert!(stores.last_success_request().unwrap().is_none());
    }

    #[test]
    fn last_success_tie_break_by_id_desc() {
        let (_dir, stores) = setup();
        let t = chrono::Utc::now().timestamp();
        stores
            .with_conn(|conn| {
                conn.execute(
                    "INSERT INTO request_logs
                     (time, group_name, provider_name, upstream_model, status_code, use_time_ms, error, failover_from, failover_to, failover_reason)
                     VALUES (?1, 'first', 'p1', 'm1', 200, 1, '', '', '', '')",
                    params![t],
                )
                .map_err(|e| AppError::Database(e.to_string()))?;
                conn.execute(
                    "INSERT INTO request_logs
                     (time, group_name, provider_name, upstream_model, status_code, use_time_ms, error, failover_from, failover_to, failover_reason)
                     VALUES (?1, 'second', 'p2', 'm2', 200, 1, '', '', '', '')",
                    params![t],
                )
                .map_err(|e| AppError::Database(e.to_string()))?;
                Ok(())
            })
            .unwrap();
        let last = stores
            .last_success_request()
            .unwrap()
            .expect("应有成功记录");
        assert_eq!(last.group_name, "second");
        assert_eq!(last.provider_name, "p2");
        assert_eq!(last.upstream_model, "m2");
    }

    #[test]
    fn daily_counts_buckets_by_local_day() {
        let (_dir, stores) = setup();
        // insert_log 用 Utc::now 落 time，多条都归到「今天」这一本地日桶。
        seed(&stores, "g", 200, "", "", "");
        seed(&stores, "g", 502, "bad", "", "");
        seed(&stores, "g", 200, "", "a", "b");
        let result = stores.request_daily_counts(365).unwrap();
        let total: i64 = result.days.iter().map(|d| d.count).sum();
        assert_eq!(
            total, 2,
            "只计成功请求（2xx 且 error 空）；失败/故障转移不计"
        );
        let today_bucket = local_day_start_unix(chrono::Local::now().timestamp());
        let bucket = result
            .days
            .iter()
            .find(|d| d.day_start_unix == today_bucket)
            .expect("今天应有一个桶");
        assert_eq!(bucket.count, 2);
        assert!(result.start_unix < result.end_unix);
    }

    #[test]
    fn daily_counts_buckets_split_across_days() {
        let (_dir, stores) = setup();
        let today_start = local_day_start_unix(chrono::Local::now().timestamp());
        // 今天 2 条、昨天 1 条：直接写聚合表（统计事实来源）。
        insert_stats_day(&stores, today_start, "m", 2, 0, 0, 0);
        insert_stats_day(&stores, today_start - 86_400, "m", 1, 0, 0, 0);
        let result = stores.request_daily_counts(365).unwrap();
        assert_eq!(result.days.len(), 2, "应有今天和昨天两个桶");
        // 升序：第一个是昨天，第二个是今天
        assert_eq!(result.days[0].count, 1);
        assert_eq!(result.days[1].count, 2);
        assert!(result.days[0].day_start_unix < result.days[1].day_start_unix);
    }

    #[test]
    fn daily_counts_window_excludes_out_of_range() {
        let (_dir, stores) = setup();
        let today_start = local_day_start_unix(chrono::Local::now().timestamp());
        insert_stats_day(&stores, today_start, "m", 1, 0, 0, 0);
        // 400 天前，落在默认 365 天窗口之外（聚合行也要被窗口过滤）
        insert_stats_day(&stores, today_start - 400 * 86_400, "m", 5, 0, 0, 0);
        let result = stores.request_daily_counts(365).unwrap();
        let total: i64 = result.days.iter().map(|d| d.count).sum();
        assert_eq!(total, 1, "窗口外的记录不计入");
    }

    #[test]
    fn daily_counts_respects_clamp() {
        let (_dir, stores) = setup();
        // days=0 → clamp 到 1；days=9999 → clamp 到 MAX，均不 panic。
        let narrow = stores.request_daily_counts(0).unwrap();
        assert!(narrow.start_unix < narrow.end_unix);
        let wide = stores.request_daily_counts(9999).unwrap();
        assert!(wide.start_unix < wide.end_unix);
        // clamp 到 MAX 的窗口应比 clamp 到 1 的窗口更早开始
        assert!(wide.start_unix < narrow.start_unix);
    }

    #[test]
    fn daily_counts_empty_db_returns_no_buckets() {
        let (_dir, stores) = setup();
        let result = stores.request_daily_counts(365).unwrap();
        assert!(result.days.is_empty());
        assert!(result.start_unix < result.end_unix);
    }

    #[test]
    fn overview_empty_db_zeros() {
        let (_dir, stores) = setup();
        let overview = stores.request_overview().unwrap();
        assert_eq!(overview.total.requests, 0);
        assert_eq!(overview.total.input_tokens, 0);
        assert_eq!(overview.total.output_tokens, 0);
        assert_eq!(overview.total.use_time_ms, 0);
        assert_eq!(overview.today.requests, 0);
    }

    #[test]
    fn overview_counts_only_success_requests() {
        let (_dir, stores) = setup();
        stores
            .insert_log(NewRequestLog {
                group_name: "g".into(),
                provider_name: "p".into(),
                upstream_model: "m".into(),
                status_code: 200,
                use_time_ms: 10,
                input_tokens: 100,
                output_tokens: 20,
                ..Default::default()
            })
            .unwrap();
        stores
            .insert_log(NewRequestLog {
                group_name: "g".into(),
                provider_name: "p".into(),
                upstream_model: "m".into(),
                status_code: 200,
                use_time_ms: 5,
                input_tokens: 50,
                output_tokens: 8,
                error: "bad".into(),
                ..Default::default()
            })
            .unwrap();
        stores
            .insert_log(NewRequestLog {
                group_name: "g".into(),
                provider_name: "p".into(),
                upstream_model: "m".into(),
                status_code: 500,
                use_time_ms: 3,
                input_tokens: 999,
                output_tokens: 999,
                ..Default::default()
            })
            .unwrap();

        let overview = stores.request_overview().unwrap();
        // 仅成功（2xx 且 error 空）计入：1 条，token/耗时只聚合它。
        assert_eq!(overview.total.requests, 1);
        assert_eq!(overview.total.input_tokens, 100);
        assert_eq!(overview.total.output_tokens, 20);
        assert_eq!(overview.total.use_time_ms, 10);
    }

    #[test]
    fn overview_today_only_includes_local_day() {
        let (_dir, stores) = setup();
        let (start, end) = local_day_bounds_unix();
        // 今天（范围内）、昨天与明天（范围外）：直接写聚合表。
        insert_stats_day(&stores, start, "m", 1, 0, 0, 0);
        insert_stats_day(&stores, start - 86_400, "m", 1, 0, 0, 0);
        insert_stats_day(&stores, end, "m", 1, 0, 0, 0);

        let overview = stores.request_overview().unwrap();
        assert_eq!(overview.total.requests, 3);
        assert_eq!(overview.today.requests, 1);
    }

    #[test]
    fn overview_costs_aggregate_by_model_price() {
        let (_dir, stores) = setup();
        // 直接写聚合表：token 与模型名；单价表按模型配价。
        insert_stats_day(&stores, 1, "deepseek-chat", 1, 1_000_000, 500_000, 0);
        insert_stats_day(&stores, 2, "unpriced-model", 1, 1000, 1000, 0);
        stores
            .with_conn(|conn| {
                conn.execute(
                    "INSERT INTO model_pricing (model_name, prompt_price_per_mtok, completion_price_per_mtok, updated_at)
                     VALUES ('deepseek/deepseek-chat', 1.25, 4.25, 0)",
                    [],
                )
                .map_err(|e| AppError::Database(e.to_string()))?;
                Ok(())
            })
            .unwrap();

        let overview = stores.request_overview().unwrap();
        // deepseek-chat 别名匹配：输入 1M × 1.25/1M = 1.25；输出 0.5M × 4.25/1M = 2.125
        assert!(
            (overview.total.input_cost - 1.25).abs() < 1e-9,
            "input_cost={}",
            overview.total.input_cost
        );
        assert!(
            (overview.total.output_cost - 2.125).abs() < 1e-9,
            "output_cost={}",
            overview.total.output_cost
        );
        assert!((overview.total.cost - 3.375).abs() < 1e-9);
        // 无价模型贡献 0，但请求计入
        assert_eq!(overview.total.requests, 2);
        assert_eq!(overview.total.input_tokens, 1_001_000);
    }

    #[test]
    fn overview_cost_zero_without_pricing_rows() {
        let (_dir, stores) = setup();
        stores
            .insert_log(NewRequestLog {
                group_name: "g".into(),
                provider_name: "p".into(),
                upstream_model: "deepseek-chat".into(),
                status_code: 200,
                use_time_ms: 1,
                input_tokens: 500,
                output_tokens: 300,
                ..Default::default()
            })
            .unwrap();
        let overview = stores.request_overview().unwrap();
        assert_eq!(overview.total.cost, 0.0);
        assert_eq!(overview.total.input_cost, 0.0);
        assert_eq!(overview.total.output_cost, 0.0);
        assert_eq!(overview.total.requests, 1);
    }

    #[test]
    fn daily_counts_only_include_success_requests() {
        let (_dir, stores) = setup();
        // 聚合表只由 insert_log 按成功口径写入；直接写 2 行同天聚合（等价于 2 次成功写入）。
        let today_start = local_day_start_unix(chrono::Local::now().timestamp());
        insert_stats_day(&stores, today_start, "m", 2, 0, 0, 0);

        let result = stores.request_daily_counts(1).unwrap();
        assert_eq!(result.days.len(), 1);
        assert_eq!(result.days[0].count, 2);
    }

    #[test]
    fn daily_stats_buckets_and_computes_cost() {
        let (_dir, stores) = setup();
        // 单价表：deepseek-chat 别名匹配（聚合表 model_name 为 deepseek-chat）。
        stores
            .replace_pricing(&[crate::domain::pricing::ModelPrice {
                model_name: "deepseek/deepseek-chat".into(),
                prompt_price_per_mtok: 1.25,
                completion_price_per_mtok: 5.0,
            }])
            .unwrap();
        let today_start = local_day_start_unix(chrono::Local::now().timestamp());
        // 今天 2 条成功（同桶）：1 条 deepseek-chat（100 in / 200 out），1 条无价模型。
        insert_stats_day(&stores, today_start, "deepseek-chat", 1, 100, 200, 0);
        insert_stats_day(&stores, today_start, "free-model", 1, 10, 20, 0);

        let stats = stores.request_daily_stats(30).unwrap();
        assert_eq!(stats.len(), 30, "恰好补全 30 天");
        let today = stats.last().unwrap();
        assert_eq!(today.requests, 2);
        assert_eq!(today.input_tokens, 110);
        assert_eq!(today.output_tokens, 220);
        // 100*1.25/1e6 + 200*5.0/1e6 + 无价 0 = 0.000125 + 0.001 = 0.001125
        assert!((today.cost - 0.001125).abs() < 1e-12);
        // 今天之前的一天应为空桶 0
        let yesterday = &stats[stats.len() - 2];
        assert_eq!(yesterday.requests, 0);
        assert_eq!(yesterday.cost, 0.0);
    }

    #[test]
    fn hourly_stats_buckets_by_hour_and_pads_24() {
        let (_dir, stores) = setup();
        let now = chrono::Utc::now().timestamp();
        // 同一本地小时内两条成功 + 一条失败
        insert_at(&stores, now - 60, 200, "", "", "");
        insert_at(&stores, now - 30, 200, "", "", "");
        insert_at(&stores, now - 10, 500, "", "", "");

        let stats = stores.request_hourly_stats().unwrap();
        assert_eq!(stats.len(), 24, "补全 24 小时");
        assert_eq!(stats.iter().map(|h| h.requests).sum::<i64>(), 2, "失败不计");
        let local_hour = ((now - local_day_start_unix(now)) / 3600) as usize;
        assert_eq!(stats[local_hour].requests, 2);
    }

    #[test]
    fn insert_log_accumulates_daily_stats_for_success_only() {
        let (_dir, stores) = setup();
        seed(&stores, "g", 200, "", "", "");
        seed(&stores, "g", 200, "", "", "");
        // 失败（5xx）与 2xx 带 error 不计入聚合。
        seed(&stores, "g", 500, "bad", "", "");
        seed(&stores, "g", 200, "structured-error", "", "");

        stores
            .with_conn(|conn| {
                let (requests, input, output, use_time): (i64, i64, i64, i64) = conn
                    .query_row(
                        "SELECT requests, input_tokens, output_tokens, use_time_ms
                         FROM daily_request_stats",
                        [],
                        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
                    )
                    .map_err(|e| AppError::Database(e.to_string()))?;
                assert_eq!(requests, 2, "仅成功请求累计");
                assert_eq!(input, 0);
                assert_eq!(output, 0);
                assert_eq!(use_time, 2, "每条 seed use_time_ms=1，两条共 2");
                Ok(())
            })
            .unwrap();

        // overview 与聚合一致：total=2。
        let overview = stores.request_overview().unwrap();
        assert_eq!(overview.total.requests, 2);
        assert_eq!(overview.today.requests, 2);
    }

    #[test]
    fn clear_logs_keeps_daily_stats() {
        let (_dir, stores) = setup();
        // 走 insert_log 写入一条成功请求（同事务累加聚合行）。
        stores
            .insert_log(NewRequestLog {
                group_name: "g".into(),
                provider_name: "p".into(),
                upstream_model: "m".into(),
                status_code: 200,
                use_time_ms: 10,
                input_tokens: 100,
                output_tokens: 20,
                ..Default::default()
            })
            .unwrap();
        // 再补一条历史聚合行（模拟已被 purge 的旧明细累计），与明细无关。
        let yesterday = local_day_start_unix(chrono::Local::now().timestamp()) - 86_400;
        insert_stats_day(&stores, yesterday, "old-model", 5, 1000, 200, 50);

        let before = stores.request_overview().unwrap();
        assert_eq!(before.total.requests, 6, "明细 1 + 聚合历史 5");

        stores.clear_logs().unwrap();

        // 明细清空，但聚合表与累计统计保留。
        let detail_count: i64 = stores
            .with_conn(|conn| {
                conn.query_row("SELECT COUNT(*) FROM request_logs", [], |r| r.get(0))
                    .map_err(|e| AppError::Database(e.to_string()))
            })
            .unwrap();
        assert_eq!(detail_count, 0, "clear_logs 只清明细");
        let after = stores.request_overview().unwrap();
        assert_eq!(after.total.requests, 6, "清空日志列表不得抹掉累计统计");
        assert_eq!(after.total.input_tokens, 1100);
        assert_eq!(after.total.output_tokens, 220);
        assert_eq!(after.total.use_time_ms, 60);
    }

    #[test]
    fn overview_total_survives_purge() {
        let (_dir, stores) = setup();
        // 核心回归：统计读聚合表，purge 删明细不影响总计。
        let now = chrono::Utc::now().timestamp();
        stores
            .insert_log(NewRequestLog {
                group_name: "g".into(),
                provider_name: "p".into(),
                upstream_model: "m".into(),
                status_code: 200,
                use_time_ms: 10,
                input_tokens: 100,
                output_tokens: 20,
                ..Default::default()
            })
            .unwrap();
        // 一条 8 天前（会被 purge 按时间窗口删除）的明细：走 insert_log 无法伪造时间，手插明细后回填。
        insert_at(&stores, now - 8 * 86_400, 200, "", "", "");
        // 清空聚合表（回填仅在表空时执行），让两条明细一起重建。
        stores
            .with_conn(|conn| {
                conn.execute("DELETE FROM daily_request_stats", [])
                    .map_err(|e| AppError::Database(e.to_string()))?;
                Ok(())
            })
            .unwrap();
        stores.backfill_daily_stats().unwrap();

        let before = stores.request_overview().unwrap();
        assert_eq!(before.total.requests, 2);
        // insert_log 100 in / 10ms + 手插明细（insert_at 默认 10 in / 1ms）
        assert_eq!(before.total.input_tokens, 110);
        assert_eq!(before.total.use_time_ms, 11);

        let purged = stores.purge_expired_logs().unwrap();
        assert_eq!(purged.deleted, 1, "8 天前明细应被 purge 删除");

        let after = stores.request_overview().unwrap();
        assert_eq!(after.total.requests, 2, "purge 后总计不得减少");
        assert_eq!(after.total.input_tokens, 110);
        assert_eq!(after.total.use_time_ms, 11);
    }

    #[test]
    fn backfill_rebuilds_from_details_and_is_idempotent() {
        let (_dir, stores) = setup();
        let now = chrono::Utc::now().timestamp();
        let today_start = local_day_start_unix(now);
        // 手插明细（绕过 insert_log，模拟旧库只有明细、聚合表为空）。
        insert_at_ex(&stores, today_start + 60, 200, "", "", "", "m1", 100, 20);
        insert_at_ex(&stores, today_start + 120, 200, "", "", "", "m2", 10, 5);
        insert_at_ex(&stores, today_start + 180, 500, "", "", "", "m1", 999, 999);
        insert_at_ex(
            &stores,
            today_start - 86_400 + 60,
            200,
            "",
            "",
            "",
            "m1",
            50,
            10,
        );
        // 清空聚合表，模拟旧库首次升级（Stores::new 回填发生在数据写入之前）。
        stores
            .with_conn(|conn| {
                conn.execute("DELETE FROM daily_request_stats", [])
                    .map_err(|e| AppError::Database(e.to_string()))?;
                Ok(())
            })
            .unwrap();

        stores.backfill_daily_stats().unwrap();

        // 成功口径：今天 2 条（m1 100/20 + m2 10/5）、昨天 1 条（m1 50/10）；失败不计。
        let result = stores.request_daily_counts(365).unwrap();
        let total: i64 = result.days.iter().map(|d| d.count).sum();
        assert_eq!(total, 3);
        let today = result
            .days
            .iter()
            .find(|d| d.day_start_unix == today_start)
            .expect("今天应有桶");
        assert_eq!(today.count, 2);

        // 幂等：再次回填（表非空）不重复累加。
        stores.backfill_daily_stats().unwrap();
        let again = stores.request_daily_counts(365).unwrap();
        let total2: i64 = again.days.iter().map(|d| d.count).sum();
        assert_eq!(total2, 3, "重复回填不得翻倍");

        // 回填后增量写入共存：insert_log 的今日写入在回填行上继续累加。
        stores
            .insert_log(NewRequestLog {
                group_name: "g".into(),
                provider_name: "p".into(),
                upstream_model: "m1".into(),
                status_code: 200,
                use_time_ms: 1,
                ..Default::default()
            })
            .unwrap();
        let final_total: i64 = stores
            .request_daily_counts(365)
            .unwrap()
            .days
            .iter()
            .map(|d| d.count)
            .sum();
        assert_eq!(final_total, 4, "回填后增量写入继续累加");
    }

    #[test]
    fn daily_counts_and_daily_stats_match_details() {
        let (_dir, stores) = setup();
        // insert_log 同时写明细与聚合；对账「聚合读」与「明细现算」。
        stores
            .insert_log(NewRequestLog {
                group_name: "g".into(),
                provider_name: "p".into(),
                upstream_model: "m1".into(),
                status_code: 200,
                use_time_ms: 10,
                input_tokens: 100,
                output_tokens: 20,
                ..Default::default()
            })
            .unwrap();
        stores
            .insert_log(NewRequestLog {
                group_name: "g".into(),
                provider_name: "p".into(),
                upstream_model: "m2".into(),
                status_code: 200,
                use_time_ms: 5,
                input_tokens: 30,
                output_tokens: 7,
                ..Default::default()
            })
            .unwrap();
        stores
            .insert_log(NewRequestLog {
                group_name: "g".into(),
                provider_name: "p".into(),
                upstream_model: "m1".into(),
                status_code: 404,
                use_time_ms: 1,
                ..Default::default()
            })
            .unwrap();

        // 明细口径（成功）
        let (detail_count, detail_in, detail_out): (i64, i64, i64) = stores
            .with_conn(|conn| {
                conn.query_row(
                    "SELECT COUNT(*), COALESCE(SUM(input_tokens), 0), COALESCE(SUM(output_tokens), 0)
                     FROM request_logs
                     WHERE status_code BETWEEN 200 AND 299
                       AND (error IS NULL OR length(error) = 0)",
                    [],
                    |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
                )
                .map_err(|e| AppError::Database(e.to_string()))
            })
            .unwrap();

        // 聚合口径
        let agg_count: i64 = stores
            .request_daily_counts(365)
            .unwrap()
            .days
            .iter()
            .map(|d| d.count)
            .sum();
        let today = stores
            .request_daily_stats(30)
            .unwrap()
            .last()
            .unwrap()
            .clone();

        assert_eq!(detail_count, agg_count, "daily_counts 与明细成功口径一致");
        assert_eq!(detail_count, today.requests);
        assert_eq!(detail_in, today.input_tokens);
        assert_eq!(detail_out, today.output_tokens);
        assert_eq!(today.cost, 0.0, "无单价时费用为 0");

        // overview 与明细口径一致
        let overview = stores.request_overview().unwrap();
        assert_eq!(overview.total.requests, detail_count);
        assert_eq!(overview.total.input_tokens, detail_in);
        assert_eq!(overview.total.output_tokens, detail_out);
    }
}
