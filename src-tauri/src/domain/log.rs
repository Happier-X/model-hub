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
pub const LOG_RETENTION_DAYS: i64 = 30;

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
}

#[derive(Debug, Clone, Serialize)]
pub struct LogPurgeResult {
    pub deleted: i64,
    pub retained: i64,
    pub retention_days: i64,
    pub cutoff_unix: i64,
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
                    "error",
                    "failover_from",
                    "failover_to",
                    "failover_reason",
                ];
                let mut placeholders = vec![
                    "?1", "?2", "?3", "?4", "?5", "?6", "?7", "?8", "?9", "?10",
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
                     (time, group_name, provider_name, upstream_model, status_code, use_time_ms, error, failover_from, failover_to, failover_reason)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                    params![
                        time,
                        log.group_name,
                        log.provider_name,
                        log.upstream_model,
                        log.status_code,
                        log.use_time_ms,
                        log.error,
                        log.failover_from,
                        log.failover_to,
                        log.failover_reason
                    ],
                )
                .map_err(|e| AppError::Database(e.to_string()))?;
            }
            Ok(())
        })
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

    /// 删除 `time < now - retention_days` 的行。
    pub fn purge_expired_logs(&self) -> Result<LogPurgeResult, AppError> {
        self.purge_logs_older_than_days(LOG_RETENTION_DAYS)
    }

    pub fn purge_logs_older_than_days(&self, retention_days: i64) -> Result<LogPurgeResult, AppError> {
        let days = retention_days.max(1);
        let now = chrono::Utc::now().timestamp();
        let cutoff = now.saturating_sub(days.saturating_mul(86_400));
        self.with_conn(|conn| {
            let deleted = conn
                .execute(
                    "DELETE FROM request_logs WHERE time < ?1",
                    params![cutoff],
                )
                .map_err(|e| AppError::Database(e.to_string()))? as i64;
            let retained: i64 = conn
                .query_row("SELECT COUNT(*) FROM request_logs", [], |row| row.get(0))
                .map_err(|e| AppError::Database(e.to_string()))?;
            Ok(LogPurgeResult {
                deleted,
                retained,
                retention_days: days,
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

    /// 本地自然日 00:00（含）至次日 00:00（不含）的请求聚合。
    pub fn request_stats_today(&self) -> Result<RequestStats, AppError> {
        let (start_ts, end_ts) = local_day_bounds_unix();
        self.request_stats_between(start_ts, end_ts)
    }

    pub fn request_stats_between(&self, start_ts: i64, end_ts: i64) -> Result<RequestStats, AppError> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::open_db;
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

    fn insert_at(stores: &Stores, time: i64, status: i64, err: &str, fo_from: &str, fo_to: &str) {
        stores
            .with_conn(|conn| {
                conn.execute(
                    "INSERT INTO request_logs
                     (time, group_name, provider_name, upstream_model, status_code, use_time_ms, error, failover_from, failover_to, failover_reason)
                     VALUES (?1, 'g', 'p', 'm', ?2, 1, ?3, ?4, ?5, '')",
                    params![time, status, err, fo_from, fo_to],
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
        assert!(result.cutoff_unix <= now - 30 * 86_400 + 5);

        // 再 purge 不应再删
        let again = stores.purge_logs_older_than_days(30).unwrap();
        assert_eq!(again.deleted, 0);
        assert_eq!(again.retained, 1);

        let page = stores.list_logs(LogQuery::default()).unwrap();
        assert_eq!(page.stored_total, 1);
        assert_eq!(page.retention_days, LOG_RETENTION_DAYS);
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
}
