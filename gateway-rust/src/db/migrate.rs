//! Schema 迁移。

use rusqlite::Connection;

use crate::error::GatewayError;

const MIGRATION_V1: &str = r#"
CREATE TABLE IF NOT EXISTS schema_migrations (
  version INTEGER PRIMARY KEY,
  applied_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS api_keys (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  name TEXT NOT NULL,
  api_key_masked TEXT NOT NULL,
  key_hash TEXT NOT NULL UNIQUE,
  enabled INTEGER NOT NULL DEFAULT 1,
  expire_at TEXT,
  max_cost REAL,
  supported_models_json TEXT
);

CREATE TABLE IF NOT EXISTS channels (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  name TEXT NOT NULL,
  type INTEGER NOT NULL,
  enabled INTEGER NOT NULL DEFAULT 1,
  model TEXT NOT NULL DEFAULT '',
  custom_model TEXT NOT NULL DEFAULT '',
  proxy INTEGER NOT NULL DEFAULT 0,
  auto_sync INTEGER NOT NULL DEFAULT 0,
  auto_group INTEGER NOT NULL DEFAULT 0,
  custom_header_json TEXT NOT NULL DEFAULT '[]'
);

CREATE TABLE IF NOT EXISTS channel_base_urls (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  channel_id INTEGER NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
  url TEXT NOT NULL,
  delay INTEGER NOT NULL DEFAULT 0,
  sort_order INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS channel_keys (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  channel_id INTEGER NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
  enabled INTEGER NOT NULL DEFAULT 1,
  channel_key TEXT NOT NULL,
  remark TEXT NOT NULL DEFAULT ''
);

CREATE TABLE IF NOT EXISTS groups (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  name TEXT NOT NULL,
  mode INTEGER NOT NULL DEFAULT 1,
  match_regex TEXT NOT NULL DEFAULT ''
);

CREATE TABLE IF NOT EXISTS group_items (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  group_id INTEGER NOT NULL REFERENCES groups(id) ON DELETE CASCADE,
  channel_id INTEGER NOT NULL,
  model_name TEXT NOT NULL,
  priority INTEGER NOT NULL DEFAULT 1,
  weight INTEGER NOT NULL DEFAULT 1
);
"#;

/// 执行到最新 schema；幂等。
pub fn migrate(conn: &Connection) -> Result<(), GatewayError> {
    conn.execute_batch(MIGRATION_V1)
        .map_err(|e| GatewayError::database(format!("执行 schema 失败: {e}")))?;

    let applied: bool = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM schema_migrations WHERE version = 1)",
            [],
            |row| row.get(0),
        )
        .map_err(|e| GatewayError::database(format!("查询迁移版本失败: {e}")))?;

    if !applied {
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO schema_migrations (version, applied_at) VALUES (1, ?1)",
            [&now],
        )
        .map_err(|e| GatewayError::database(format!("记录迁移版本失败: {e}")))?;
        tracing::info!(version = 1, "已应用数据库迁移");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    #[test]
    fn migrate_is_idempotent() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
        migrate(&conn).unwrap();
        migrate(&conn).unwrap();

        let version: i64 = conn
            .query_row(
                "SELECT version FROM schema_migrations WHERE version = 1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(version, 1);

        // 表存在
        for table in [
            "api_keys",
            "channels",
            "channel_keys",
            "channel_base_urls",
            "groups",
            "group_items",
        ] {
            let n: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?1",
                    [table],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(n, 1, "missing table {table}");
        }
    }
}
