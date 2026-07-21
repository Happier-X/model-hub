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

    pub fn list_logs(&self, page: i64, page_size: i64) -> Result<Vec<RequestLog>, AppError> {
        let page = page.max(1);
        let page_size = page_size.clamp(1, 100);
        let offset = (page - 1) * page_size;
        self.with_conn(|conn| {
            let mut stmt = conn
                .prepare(
                    "SELECT id, time, group_name, provider_name, upstream_model, status_code, use_time_ms, error, failover_from, failover_to, failover_reason
                     FROM request_logs ORDER BY id DESC LIMIT ?1 OFFSET ?2",
                )
                .map_err(|e| AppError::Database(e.to_string()))?;
            let rows = stmt
                .query_map(params![page_size, offset], |row| {
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
                })
                .map_err(|e| AppError::Database(e.to_string()))?;
            let mut out = Vec::new();
            for r in rows {
                out.push(r.map_err(|e| AppError::Database(e.to_string()))?);
            }
            Ok(out)
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
