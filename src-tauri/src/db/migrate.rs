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

/// 兼容旧 gateway-rust 的 api_keys(api_key_masked, key_hash, ...)。
fn ensure_api_keys_columns(conn: &Connection) -> Result<(), AppError> {
    let columns = table_column_names(conn, "api_keys")?;

    // key_hash 若完全缺失，无法从脱敏值恢复原始 Key；补空值仅保证 schema 可读，
    // 用户需重建该类更早明文 schema 的 Key（本任务不迁移 octopus 明文表）。
    let required: &[(&str, &str)] = &[
        ("name", "name TEXT NOT NULL DEFAULT ''"),
        ("key_hash", "key_hash TEXT NOT NULL DEFAULT ''"),
        ("masked", "masked TEXT NOT NULL DEFAULT ''"),
        ("enabled", "enabled INTEGER NOT NULL DEFAULT 1"),
    ];
    for (name, ddl) in required {
        if columns.contains(*name) {
            continue;
        }
        conn.execute(&format!("ALTER TABLE api_keys ADD COLUMN {ddl}"), [])
            .map_err(|e| AppError::Database(format!("添加 api_keys.{name} 字段失败: {e}")))?;
    }

    if columns.contains("api_key_masked") {
        conn.execute(
            "UPDATE api_keys SET masked = api_key_masked WHERE masked = ''",
            [],
        )
        .map_err(|e| AppError::Database(format!("回填 api_keys.masked 失败: {e}")))?;
    }

    if !columns.contains("created_at") {
        let now = chrono::Utc::now().to_rfc3339();
        let escaped_now = now.replace('\'', "''");
        conn.execute(
            &format!(
                "ALTER TABLE api_keys ADD COLUMN created_at TEXT NOT NULL DEFAULT '{escaped_now}'"
            ),
            [],
        )
        .map_err(|e| AppError::Database(format!("添加 api_keys.created_at 字段失败: {e}")))?;
    }
    Ok(())
}

/// 兼容旧 gateway-rust 的 group_items(channel_id, model_name, priority, weight)。
///
/// 使用加列 + 条件回填，不重建/删除旧表；旧列保留用于回溯。
fn ensure_group_items_columns(conn: &Connection) -> Result<(), AppError> {
    let columns = table_column_names(conn, "group_items")?;
    let required: &[(&str, &str)] = &[
        ("provider_id", "provider_id INTEGER NOT NULL DEFAULT 0"),
        ("upstream_model", "upstream_model TEXT NOT NULL DEFAULT ''"),
        ("sort_order", "sort_order INTEGER NOT NULL DEFAULT 0"),
    ];
    for (name, ddl) in required {
        if columns.contains(*name) {
            continue;
        }
        conn.execute(
            &format!("ALTER TABLE group_items ADD COLUMN {ddl}"),
            [],
        )
        .map_err(|e| AppError::Database(format!("添加 group_items.{name} 字段失败: {e}")))?;
    }

    // 仅从确实存在的旧列回填；只覆盖新列默认值，保留已经迁移/编辑过的数据。
    if columns.contains("channel_id") {
        conn.execute(
            "UPDATE group_items SET provider_id = channel_id WHERE provider_id = 0",
            [],
        )
        .map_err(|e| AppError::Database(format!("回填 group_items.provider_id 失败: {e}")))?;
    }
    if columns.contains("model_name") {
        conn.execute(
            "UPDATE group_items SET upstream_model = model_name WHERE upstream_model = ''",
            [],
        )
        .map_err(|e| AppError::Database(format!("回填 group_items.upstream_model 失败: {e}")))?;
    }
    if columns.contains("priority") {
        conn.execute(
            "UPDATE group_items SET sort_order = COALESCE(priority, 0) WHERE sort_order = 0",
            [],
        )
        .map_err(|e| AppError::Database(format!("回填 group_items.sort_order 失败: {e}")))?;
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
    ensure_api_keys_columns(conn)?;
    ensure_group_items_columns(conn)?;
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
    fn migrate_adds_and_backfills_api_keys_current_columns() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE api_keys (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                api_key_masked TEXT NOT NULL,
                key_hash TEXT NOT NULL UNIQUE,
                enabled INTEGER NOT NULL DEFAULT 1,
                expire_at TEXT,
                max_cost REAL,
                supported_models_json TEXT
            );
            INSERT INTO api_keys
                (name, api_key_masked, key_hash, enabled)
            VALUES ('legacy', 'sk-modelhub-****abcd', 'legacy-hash', 0);",
        )
        .unwrap();

        migrate(&conn).unwrap();

        let row: (String, String, String, i64, String) = conn
            .query_row(
                "SELECT name, masked, key_hash, enabled, created_at FROM api_keys WHERE id = 1",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
            )
            .unwrap();
        assert_eq!(row.0, "legacy");
        assert_eq!(row.1, "sk-modelhub-****abcd");
        assert_eq!(row.2, "legacy-hash");
        assert_eq!(row.3, 0);
        assert!(!row.4.is_empty());
        chrono::DateTime::parse_from_rfc3339(&row.4).unwrap();

        migrate(&conn).unwrap();
        let row2: (String, String) = conn
            .query_row(
                "SELECT masked, created_at FROM api_keys WHERE id = 1",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(row2, (row.1, row.4));
    }

    #[test]
    fn migrate_preserves_existing_api_key_current_values() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE api_keys (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                api_key_masked TEXT NOT NULL,
                key_hash TEXT NOT NULL UNIQUE,
                masked TEXT NOT NULL DEFAULT '',
                enabled INTEGER NOT NULL DEFAULT 1,
                created_at TEXT NOT NULL
            );
            INSERT INTO api_keys
                (name, api_key_masked, key_hash, masked, enabled, created_at)
            VALUES ('k', 'legacy-mask', 'hash', 'current-mask', 1, '2024-01-01T00:00:00Z');",
        )
        .unwrap();
        migrate(&conn).unwrap();
        let row: (String, String, String) = conn
            .query_row(
                "SELECT masked, key_hash, created_at FROM api_keys",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .unwrap();
        assert_eq!(row, ("current-mask".into(), "hash".into(), "2024-01-01T00:00:00Z".into()));
    }

    #[test]
    fn migrate_adds_and_backfills_group_items_current_columns() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE groups (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL UNIQUE
            );
            CREATE TABLE group_items (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                group_id INTEGER NOT NULL,
                channel_id INTEGER NOT NULL,
                model_name TEXT NOT NULL,
                priority INTEGER NOT NULL DEFAULT 1,
                weight INTEGER NOT NULL DEFAULT 1
            );
            INSERT INTO groups (name) VALUES ('legacy-group');
            INSERT INTO group_items
                (group_id, channel_id, model_name, priority, weight)
            VALUES (1, 7, 'legacy-model', 3, 2);",
        )
        .unwrap();

        migrate(&conn).unwrap();

        let row: (i64, i64, String, i64, i64, String) = conn
            .query_row(
                "SELECT id, provider_id, upstream_model, sort_order, channel_id, model_name
                 FROM group_items WHERE id = 1",
                [],
                |r| {
                    Ok((
                        r.get(0)?,
                        r.get(1)?,
                        r.get(2)?,
                        r.get(3)?,
                        r.get(4)?,
                        r.get(5)?,
                    ))
                },
            )
            .unwrap();
        assert_eq!(row, (1, 7, "legacy-model".into(), 3, 7, "legacy-model".into()));

        // 当前应用写入：兼容表必须同步填新旧两套 NOT NULL 列。
        conn.execute(
            "INSERT INTO group_items
             (group_id, provider_id, upstream_model, sort_order,
              channel_id, model_name, priority, weight)
             VALUES (1, 8, 'new-model', 4, 8, 'new-model', 4, 1)",
            [],
        )
        .unwrap();
        migrate(&conn).unwrap();
        let n: i64 = conn
            .query_row("SELECT COUNT(*) FROM group_items", [], |r| r.get(0))
            .unwrap();
        assert_eq!(n, 2);
    }

    #[test]
    fn migrate_group_items_preserves_existing_current_values() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE group_items (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                group_id INTEGER NOT NULL,
                channel_id INTEGER NOT NULL,
                model_name TEXT NOT NULL,
                priority INTEGER NOT NULL,
                weight INTEGER NOT NULL,
                provider_id INTEGER NOT NULL DEFAULT 0,
                upstream_model TEXT NOT NULL DEFAULT '',
                sort_order INTEGER NOT NULL DEFAULT 0
            );
            INSERT INTO group_items
                (group_id, channel_id, model_name, priority, weight, provider_id, upstream_model, sort_order)
            VALUES (1, 7, 'old', 1, 1, 9, 'current', 5);",
        )
        .unwrap();
        migrate(&conn).unwrap();
        let row: (i64, String, i64) = conn
            .query_row(
                "SELECT provider_id, upstream_model, sort_order FROM group_items",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .unwrap();
        assert_eq!(row, (9, "current".into(), 5));
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
