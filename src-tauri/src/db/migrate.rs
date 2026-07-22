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

fn table_column_names(conn: &Connection, table: &str) -> Result<std::collections::HashSet<String>, AppError> {
    // PRAGMA table_info 不支持绑定参数表名；table 仅内部常量调用。
    let mut stmt = conn
        .prepare(&format!("PRAGMA table_info({table})"))
        .map_err(|e| AppError::Database(format!("检查 {table} 表结构失败: {e}")))?;
    let mut rows = stmt
        .query([])
        .map_err(|e| AppError::Database(format!("读取 {table} 表结构失败: {e}")))?;
    let mut names = std::collections::HashSet::new();
    while let Some(row) = rows
        .next()
        .map_err(|e| AppError::Database(format!("读取 {table} 表字段失败: {e}")))?
    {
        let name: String = row
            .get(1)
            .map_err(|e| AppError::Database(format!("读取 {table} 表字段名失败: {e}")))?;
        names.insert(name);
    }
    Ok(names)
}

fn ensure_group_columns(conn: &Connection) -> Result<(), AppError> {
    let columns = table_column_names(conn, "groups")?;
    let has_auto_failover = columns.contains("auto_failover");
    let has_created_at = columns.contains("created_at");

    if !has_auto_failover {
        conn.execute(
            "ALTER TABLE groups ADD COLUMN auto_failover INTEGER NOT NULL DEFAULT 1",
            [],
        )
        .map_err(|e| AppError::Database(format!("添加 groups.auto_failover 字段失败: {e}")))?;
    }
    if !has_created_at {
        let now = chrono::Utc::now().to_rfc3339();
        // SQLite 的 ALTER TABLE 不接受参数化 DEFAULT；RFC3339 只需转义单引号即可安全作为字面量。
        let escaped_now = now.replace('\'', "''");
        conn.execute(
            &format!(
                "ALTER TABLE groups ADD COLUMN created_at TEXT NOT NULL DEFAULT '{escaped_now}'"
            ),
            [],
        )
        .map_err(|e| AppError::Database(format!("添加 groups.created_at 字段失败: {e}")))?;
    }
    Ok(())
}

/// 为残缺的既有 `request_logs` 表补齐当前 schema 列（CREATE IF NOT EXISTS 不会改旧表）。
fn ensure_request_logs_columns(conn: &Connection) -> Result<(), AppError> {
    // 表尚不存在时由 MIGRATION_V1 创建完整表，此处无需处理。
    let exists: bool = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='request_logs')",
            [],
            |row| row.get(0),
        )
        .map_err(|e| AppError::Database(format!("检查 request_logs 是否存在失败: {e}")))?;
    if !exists {
        return Ok(());
    }

    let columns = table_column_names(conn, "request_logs")?;
    // (列名, 完整 ADD COLUMN 子句)
    let required: &[(&str, &str)] = &[
        ("time", "time INTEGER NOT NULL DEFAULT 0"),
        ("group_name", "group_name TEXT NOT NULL DEFAULT ''"),
        ("provider_name", "provider_name TEXT NOT NULL DEFAULT ''"),
        ("upstream_model", "upstream_model TEXT NOT NULL DEFAULT ''"),
        ("status_code", "status_code INTEGER NOT NULL DEFAULT 0"),
        ("use_time_ms", "use_time_ms INTEGER NOT NULL DEFAULT 0"),
        ("error", "error TEXT NOT NULL DEFAULT ''"),
        ("failover_from", "failover_from TEXT NOT NULL DEFAULT ''"),
        ("failover_to", "failover_to TEXT NOT NULL DEFAULT ''"),
        ("failover_reason", "failover_reason TEXT NOT NULL DEFAULT ''"),
    ];

    for (name, ddl) in required {
        if columns.contains(*name) {
            continue;
        }
        conn.execute(
            &format!("ALTER TABLE request_logs ADD COLUMN {ddl}"),
            [],
        )
        .map_err(|e| {
            AppError::Database(format!("添加 request_logs.{name} 字段失败: {e}"))
        })?;
    }

    conn.execute_batch("CREATE INDEX IF NOT EXISTS idx_request_logs_time ON request_logs(time DESC);")
        .map_err(|e| AppError::Database(format!("创建 request_logs 时间索引失败: {e}")))?;
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
    ensure_group_columns(conn)?;
    ensure_request_logs_columns(conn)?;
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
    fn migrate_adds_missing_group_columns_without_losing_data() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE groups (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL UNIQUE
            );
            INSERT INTO groups (name) VALUES ('legacy');",
        )
        .unwrap();

        migrate(&conn).unwrap();

        let group: (String, i64, String) = conn
            .query_row(
                "SELECT name, auto_failover, created_at FROM groups WHERE id = 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        assert_eq!(group.0, "legacy");
        assert_eq!(group.1, 1);
        assert!(!group.2.is_empty());
        chrono::DateTime::parse_from_rfc3339(&group.2).unwrap();

        migrate(&conn).unwrap();
        let (value, created_at): (i64, String) = conn
            .query_row(
                "SELECT auto_failover, created_at FROM groups WHERE name = 'legacy'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(value, 1);
        assert_eq!(created_at, group.2);
    }

    #[test]
    fn migrate_adds_missing_request_logs_columns_without_losing_rows() {
        let conn = Connection::open_in_memory().unwrap();
        // 残缺旧表：仅有 id/time/message 风格字段，缺 status_code 等
        conn.execute_batch(
            "CREATE TABLE request_logs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                time INTEGER NOT NULL,
                message TEXT NOT NULL DEFAULT ''
            );
            INSERT INTO request_logs (time, message) VALUES (1700000000, 'legacy row');",
        )
        .unwrap();

        migrate(&conn).unwrap();

        let cols = super::table_column_names(&conn, "request_logs").unwrap();
        for name in [
            "status_code",
            "use_time_ms",
            "error",
            "group_name",
            "provider_name",
            "upstream_model",
            "failover_from",
            "failover_to",
            "failover_reason",
        ] {
            assert!(cols.contains(name), "missing column {name}");
        }

        let row: (i64, i64, i64, String) = conn
            .query_row(
                "SELECT id, time, status_code, message FROM request_logs WHERE id = 1",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .unwrap();
        assert_eq!(row.0, 1);
        assert_eq!(row.1, 1700000000);
        assert_eq!(row.2, 0); // 新列默认 0
        assert_eq!(row.3, "legacy row");

        // 迁移后可做与概览统计相同的聚合
        let total: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM request_logs WHERE status_code >= 0",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(total, 1);

        migrate(&conn).unwrap();
        let n: i64 = conn
            .query_row("SELECT COUNT(*) FROM request_logs", [], |r| r.get(0))
            .unwrap();
        assert_eq!(n, 1);
    }

    #[test]
    fn migrate_does_not_overwrite_existing_group_values() {
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

        let values: (i64, String) = conn
            .query_row(
                "SELECT auto_failover, created_at FROM groups WHERE name = 'disabled'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(values, (0, "2024-01-01T00:00:00Z".into()));
    }
}
