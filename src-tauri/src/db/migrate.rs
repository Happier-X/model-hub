//! 当前 Schema v1 迁移；不执行旧版数据库自动迁移。

use rusqlite::Connection;

use crate::error::AppError;

const MIGRATION_V1: &str = r#"
CREATE TABLE IF NOT EXISTS schema_migrations (
  version INTEGER PRIMARY KEY,
  applied_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS providers (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  name TEXT NOT NULL,
  base_url TEXT NOT NULL,
  api_key TEXT NOT NULL DEFAULT '',
  enabled INTEGER NOT NULL DEFAULT 1,
  created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS groups (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  name TEXT NOT NULL UNIQUE,
  auto_failover INTEGER NOT NULL DEFAULT 1,
  created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS group_items (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  group_id INTEGER NOT NULL REFERENCES groups(id) ON DELETE CASCADE,
  provider_id INTEGER NOT NULL REFERENCES providers(id) ON DELETE CASCADE,
  upstream_model TEXT NOT NULL,
  sort_order INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS api_keys (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  name TEXT NOT NULL,
  key_hash TEXT NOT NULL UNIQUE,
  masked TEXT NOT NULL,
  enabled INTEGER NOT NULL DEFAULT 1,
  created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS request_logs (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  time INTEGER NOT NULL,
  group_name TEXT NOT NULL DEFAULT '',
  provider_name TEXT NOT NULL DEFAULT '',
  upstream_model TEXT NOT NULL DEFAULT '',
  status_code INTEGER NOT NULL DEFAULT 0,
  use_time_ms INTEGER NOT NULL DEFAULT 0,
  error TEXT NOT NULL DEFAULT '',
  failover_from TEXT NOT NULL DEFAULT '',
  failover_to TEXT NOT NULL DEFAULT '',
  failover_reason TEXT NOT NULL DEFAULT ''
);
CREATE INDEX IF NOT EXISTS idx_request_logs_time ON request_logs(time DESC);
"#;

fn ensure_group_auto_failover(conn: &Connection) -> Result<(), AppError> {
    let mut stmt = conn
        .prepare("PRAGMA table_info(groups)")
        .map_err(|e| AppError::Database(format!("检查 groups 表结构失败: {e}")))?;
    let mut columns = stmt
        .query([])
        .map_err(|e| AppError::Database(format!("读取 groups 表结构失败: {e}")))?;
    let mut has_auto_failover = false;
    while let Some(row) = columns
        .next()
        .map_err(|e| AppError::Database(format!("读取 groups 表字段失败: {e}")))?
    {
        let name: String = row
            .get(1)
            .map_err(|e| AppError::Database(format!("读取 groups 表字段名失败: {e}")))?;
        if name == "auto_failover" {
            has_auto_failover = true;
            break;
        }
    }
    drop(columns);
    drop(stmt);

    if !has_auto_failover {
        conn.execute(
            "ALTER TABLE groups ADD COLUMN auto_failover INTEGER NOT NULL DEFAULT 1",
            [],
        )
        .map_err(|e| AppError::Database(format!("添加 groups.auto_failover 字段失败: {e}")))?;
    }
    Ok(())
}

fn apply_version(conn: &Connection, version: i64) -> Result<(), AppError> {
    let applied: bool = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM schema_migrations WHERE version = ?1)",
            [version],
            |row| row.get(0),
        )
        .map_err(|e| AppError::Database(format!("查询迁移版本失败: {e}")))?;

    if !applied {
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO schema_migrations (version, applied_at) VALUES (?1, ?2)",
            rusqlite::params![version, now],
        )
        .map_err(|e| AppError::Database(format!("记录迁移版本失败: {e}")))?;
    }
    Ok(())
}

pub fn migrate(conn: &Connection) -> Result<(), AppError> {
    conn.execute_batch(MIGRATION_V1)
        .map_err(|e| AppError::Database(format!("执行 schema v1 失败: {e}")))?;
    ensure_group_auto_failover(conn)?;
    apply_version(conn, 1)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrate_is_idempotent() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
        migrate(&conn).unwrap();
        migrate(&conn).unwrap();
        for table in [
            "providers",
            "groups",
            "group_items",
            "api_keys",
            "request_logs",
        ] {
            let n: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?1",
                    [table],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(n, 1, "missing {table}");
        }
    }

    #[test]
    fn migrate_adds_missing_group_auto_failover_without_losing_data() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE groups (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL UNIQUE,
                created_at TEXT NOT NULL
            );
            INSERT INTO groups (name, created_at) VALUES ('legacy', '2024-01-01T00:00:00Z');",
        )
        .unwrap();

        migrate(&conn).unwrap();

        let group: (String, i64) = conn
            .query_row(
                "SELECT name, auto_failover FROM groups WHERE id = 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(group, ("legacy".into(), 1));

        migrate(&conn).unwrap();
        let value: i64 = conn
            .query_row(
                "SELECT auto_failover FROM groups WHERE name = 'legacy'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(value, 1);
    }

    #[test]
    fn migrate_does_not_overwrite_existing_auto_failover_value() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE groups (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL UNIQUE,
                auto_failover INTEGER NOT NULL DEFAULT 1,
                created_at TEXT NOT NULL
            );
            INSERT INTO groups (name, auto_failover, created_at)
            VALUES ('disabled', 0, '2024-01-01T00:00:00Z');",
        )
        .unwrap();

        migrate(&conn).unwrap();
        migrate(&conn).unwrap();

        let value: i64 = conn
            .query_row(
                "SELECT auto_failover FROM groups WHERE name = 'disabled'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(value, 0);
    }
}
