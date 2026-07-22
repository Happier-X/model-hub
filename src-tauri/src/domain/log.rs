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

#[derive(Debug, Clone, Serialize)]
pub struct LogPage {
    pub items: Vec<RequestLog>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
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

impl Stores {
    pub fn insert_log(&self, log: NewRequestLog) -> Result<(), AppError> {
        let time = chrono::Utc::now().timestamp();
        self.with_conn(|conn| {
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
            Ok(())
        })
    }

    pub fn list_logs(&self, query: LogQuery) -> Result<LogPage, AppError> {
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

            Ok(LogPage {
                items,
                total,
                page,
                page_size,
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
}
