//! 请求日志 SQLite 存储。

use rusqlite::params;

use super::model::{NewRelayLog, RelayLog};
use crate::db::DbConn;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LogStoreError {
    Internal,
}

impl std::fmt::Display for LogStoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Internal => write!(f, "日志存储内部错误"),
        }
    }
}

#[derive(Clone)]
pub struct LogStore {
    db: DbConn,
}

impl LogStore {
    pub fn new(db: DbConn) -> Self {
        Self { db }
    }

    pub fn insert(&self, log: &NewRelayLog) -> Result<i64, LogStoreError> {
        let conn = self.db.lock().map_err(|_| LogStoreError::Internal)?;
        conn.execute(
            "INSERT INTO request_logs (
                time, request_model_name, channel_name, actual_model_name,
                input_tokens, output_tokens, use_time, cost, error
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                log.time,
                log.request_model_name,
                log.channel_name,
                log.actual_model_name,
                log.input_tokens,
                log.output_tokens,
                log.use_time,
                log.cost,
                log.error,
            ],
        )
        .map_err(|_| LogStoreError::Internal)?;
        Ok(conn.last_insert_rowid())
    }

    /// 按 id 倒序分页；`offset = (page-1) * page_size`。
    pub fn list(&self, page: u32, page_size: u32) -> Result<Vec<RelayLog>, LogStoreError> {
        let offset = (page.saturating_sub(1) as i64) * (page_size as i64);
        let limit = page_size as i64;
        let conn = self.db.lock().map_err(|_| LogStoreError::Internal)?;
        let mut stmt = conn
            .prepare(
                "SELECT id, time, request_model_name, channel_name, actual_model_name,
                        input_tokens, output_tokens, use_time, cost, error
                 FROM request_logs
                 ORDER BY id DESC
                 LIMIT ?1 OFFSET ?2",
            )
            .map_err(|_| LogStoreError::Internal)?;
        let rows = stmt
            .query_map(params![limit, offset], |row| {
                Ok(RelayLog {
                    id: row.get(0)?,
                    time: row.get(1)?,
                    request_model_name: row.get(2)?,
                    channel_name: row.get(3)?,
                    actual_model_name: row.get(4)?,
                    input_tokens: row.get(5)?,
                    output_tokens: row.get(6)?,
                    use_time: row.get(7)?,
                    cost: row.get(8)?,
                    error: row.get(9)?,
                })
            })
            .map_err(|_| LogStoreError::Internal)?;

        let mut out = Vec::new();
        for row in rows {
            out.push(row.map_err(|_| LogStoreError::Internal)?);
        }
        Ok(out)
    }

    pub fn clear(&self) -> Result<(), LogStoreError> {
        let conn = self.db.lock().map_err(|_| LogStoreError::Internal)?;
        conn.execute("DELETE FROM request_logs", [])
            .map_err(|_| LogStoreError::Internal)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::open_path;

    #[test]
    fn insert_list_clear() {
        let db = open_path(":memory:").unwrap();
        let store = LogStore::new(db);
        let id = store
            .insert(&NewRelayLog {
                time: 1_700_000_000,
                request_model_name: "g".into(),
                channel_name: "c".into(),
                actual_model_name: "m".into(),
                input_tokens: 3,
                output_tokens: 5,
                use_time: 1,
                cost: 0.0,
                error: String::new(),
            })
            .unwrap();
        assert!(id > 0);

        let list = store.list(1, 20).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].request_model_name, "g");
        assert_eq!(list[0].input_tokens, 3);
        assert_eq!(list[0].output_tokens, 5);

        store.clear().unwrap();
        assert!(store.list(1, 20).unwrap().is_empty());
    }
}
