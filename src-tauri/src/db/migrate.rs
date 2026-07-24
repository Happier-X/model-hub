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
  created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS group_items (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  group_id INTEGER NOT NULL REFERENCES groups(id) ON DELETE CASCADE,
  provider_id INTEGER NOT NULL REFERENCES providers(id) ON DELETE CASCADE,
  upstream_model TEXT NOT NULL,
  sort_order INTEGER NOT NULL DEFAULT 0
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

fn table_column_names(
    conn: &Connection,
    table: &str,
) -> Result<std::collections::HashSet<String>, AppError> {
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
    let has_created_at = columns.contains("created_at");

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

/// 旧库若含 `groups.auto_failover`，以重建表方式删除该列并保留 id/name/created_at 与 group_items。
fn drop_groups_auto_failover_if_present(conn: &Connection) -> Result<(), AppError> {
    let columns = table_column_names(conn, "groups")?;
    if !columns.contains("auto_failover") {
        return Ok(());
    }

    // 删除列前确保 created_at 存在，便于 SELECT 拷贝。
    if !columns.contains("created_at") {
        ensure_group_columns(conn)?;
    }

    let fk_was_on: bool = conn
        .query_row("PRAGMA foreign_keys", [], |row| row.get::<_, i64>(0))
        .map(|v| v != 0)
        .unwrap_or(false);
    if fk_was_on {
        conn.execute_batch("PRAGMA foreign_keys = OFF;")
            .map_err(|e| AppError::Database(format!("关闭 foreign_keys 失败: {e}")))?;
    }

    let result = (|| {
        let tx = conn.unchecked_transaction().map_err(|e| {
            AppError::Database(format!("开始删除 groups.auto_failover 事务失败: {e}"))
        })?;
        tx.execute_batch(
            r#"
CREATE TABLE groups__new (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  name TEXT NOT NULL UNIQUE,
  created_at TEXT NOT NULL
);
INSERT INTO groups__new (id, name, created_at)
  SELECT id, name, created_at FROM groups;
DROP TABLE groups;
ALTER TABLE groups__new RENAME TO groups;
"#,
        )
        .map_err(|e| AppError::Database(format!("删除 groups.auto_failover 重建表失败: {e}")))?;
        tx.commit()
            .map_err(|e| AppError::Database(format!("提交删除 groups.auto_failover 事务失败: {e}")))
    })();

    let restore_result = if fk_was_on {
        conn.execute_batch("PRAGMA foreign_keys = ON;")
            .map_err(|e| AppError::Database(format!("恢复 foreign_keys 失败: {e}")))
    } else {
        Ok(())
    };
    result?;
    restore_result?;

    // 校验列已移除，且重建没有破坏 group_items 外键关系。
    let after = table_column_names(conn, "groups")?;
    if after.contains("auto_failover") {
        return Err(AppError::Database(
            "删除 groups.auto_failover 后列仍存在".into(),
        ));
    }
    // 只校验 group_items → groups：避免因历史孤儿 provider_id 阻断启动。
    let orphan_items: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM group_items gi
             LEFT JOIN groups g ON g.id = gi.group_id
             WHERE g.id IS NULL",
            [],
            |row| row.get(0),
        )
        .map_err(|e| AppError::Database(format!("校验 group_items 与 groups 关联失败: {e}")))?;
    if orphan_items != 0 {
        return Err(AppError::Database(format!(
            "删除 groups.auto_failover 后发现 {orphan_items} 条 group_items 失去分组关联"
        )));
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
        conn.execute(&format!("ALTER TABLE group_items ADD COLUMN {ddl}"), [])
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
        (
            "failover_reason",
            "failover_reason TEXT NOT NULL DEFAULT ''",
        ),
    ];

    for (name, ddl) in required {
        if columns.contains(*name) {
            continue;
        }
        conn.execute(&format!("ALTER TABLE request_logs ADD COLUMN {ddl}"), [])
            .map_err(|e| AppError::Database(format!("添加 request_logs.{name} 字段失败: {e}")))?;
    }

    // 旧 gateway-rust 列 → 当前列条件回填（仅当新列仍为默认空/0）。
    let columns = table_column_names(conn, "request_logs")?;
    if columns.contains("request_model_name") && columns.contains("group_name") {
        conn.execute(
            "UPDATE request_logs SET group_name = request_model_name
             WHERE (group_name IS NULL OR group_name = '')
               AND request_model_name IS NOT NULL AND length(request_model_name) > 0",
            [],
        )
        .map_err(|e| AppError::Database(format!("回填 request_logs.group_name 失败: {e}")))?;
    }
    if columns.contains("channel_name") && columns.contains("provider_name") {
        conn.execute(
            "UPDATE request_logs SET provider_name = channel_name
             WHERE (provider_name IS NULL OR provider_name = '')
               AND channel_name IS NOT NULL AND length(channel_name) > 0",
            [],
        )
        .map_err(|e| AppError::Database(format!("回填 request_logs.provider_name 失败: {e}")))?;
    }
    if columns.contains("actual_model_name") && columns.contains("upstream_model") {
        conn.execute(
            "UPDATE request_logs SET upstream_model = actual_model_name
             WHERE (upstream_model IS NULL OR upstream_model = '')
               AND actual_model_name IS NOT NULL AND length(actual_model_name) > 0",
            [],
        )
        .map_err(|e| AppError::Database(format!("回填 request_logs.upstream_model 失败: {e}")))?;
    }
    if columns.contains("use_time") && columns.contains("use_time_ms") {
        conn.execute(
            "UPDATE request_logs SET use_time_ms = use_time
             WHERE use_time_ms = 0 AND use_time IS NOT NULL AND use_time != 0",
            [],
        )
        .map_err(|e| AppError::Database(format!("回填 request_logs.use_time_ms 失败: {e}")))?;
    }

    conn.execute_batch(
        "CREATE INDEX IF NOT EXISTS idx_request_logs_time ON request_logs(time DESC);",
    )
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
    drop_groups_auto_failover_if_present(conn)?;
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
        for table in ["providers", "groups", "group_items", "request_logs"] {
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
    fn migrate_adds_missing_created_at_without_losing_data() {
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

        let group: (String, String) = conn
            .query_row(
                "SELECT name, created_at FROM groups WHERE id = 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(group.0, "legacy");
        assert!(!group.1.is_empty());
        chrono::DateTime::parse_from_rfc3339(&group.1).unwrap();
        assert!(!table_column_names(&conn, "groups")
            .unwrap()
            .contains("auto_failover"));

        migrate(&conn).unwrap();
        let created_at: String = conn
            .query_row(
                "SELECT created_at FROM groups WHERE name = 'legacy'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(created_at, group.1);
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
        assert_eq!(
            row,
            (1, 7, "legacy-model".into(), 3, 7, "legacy-model".into())
        );

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

        // 迁移后可做与首页统计相同的聚合
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
    fn migrate_backfills_legacy_request_logs_names_into_current_columns() {
        let conn = Connection::open_in_memory().unwrap();
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
            );
            INSERT INTO request_logs
                (time, request_model_name, channel_name, actual_model_name, use_time, error)
            VALUES
                (1700000001, 'legacy-group', 'legacy-channel', 'legacy-upstream', 42, 'boom');",
        )
        .unwrap();

        migrate(&conn).unwrap();

        let row: (String, String, String, i64, String) = conn
            .query_row(
                "SELECT group_name, provider_name, upstream_model, use_time_ms, error
                 FROM request_logs WHERE id = 1",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
            )
            .unwrap();
        assert_eq!(row.0, "legacy-group");
        assert_eq!(row.1, "legacy-channel");
        assert_eq!(row.2, "legacy-upstream");
        assert_eq!(row.3, 42);
        assert_eq!(row.4, "boom");

        migrate(&conn).unwrap();
        let again: (String, i64) = conn
            .query_row(
                "SELECT group_name, use_time_ms FROM request_logs WHERE id = 1",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(again, ("legacy-group".into(), 42));
    }

    #[test]
    fn migrate_drops_auto_failover_and_preserves_group_and_items() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "PRAGMA foreign_keys = ON;
            CREATE TABLE groups (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL UNIQUE,
                auto_failover INTEGER NOT NULL DEFAULT 1,
                created_at TEXT NOT NULL
            );
            CREATE TABLE group_items (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                group_id INTEGER NOT NULL REFERENCES groups(id) ON DELETE CASCADE,
                provider_id INTEGER NOT NULL,
                upstream_model TEXT NOT NULL,
                sort_order INTEGER NOT NULL DEFAULT 0
            );
            INSERT INTO groups (name, auto_failover, created_at)
            VALUES ('disabled', 0, '2024-01-01T00:00:00Z');
            INSERT INTO group_items (group_id, provider_id, upstream_model, sort_order)
            VALUES (1, 9, 'legacy-model', 3);",
        )
        .unwrap();

        migrate(&conn).unwrap();
        migrate(&conn).unwrap();

        let values: (i64, String, String) = conn
            .query_row(
                "SELECT g.id, g.created_at, gi.upstream_model
                 FROM groups g JOIN group_items gi ON gi.group_id = g.id
                 WHERE g.name = 'disabled'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        assert_eq!(
            values,
            (1, "2024-01-01T00:00:00Z".into(), "legacy-model".into())
        );
        assert!(!table_column_names(&conn, "groups")
            .unwrap()
            .contains("auto_failover"));
        let foreign_keys_on: i64 = conn
            .query_row("PRAGMA foreign_keys", [], |row| row.get(0))
            .unwrap();
        assert_eq!(foreign_keys_on, 1);
        let orphan_items: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM group_items gi
                 LEFT JOIN groups g ON g.id = gi.group_id
                 WHERE g.id IS NULL",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(orphan_items, 0);
        // 重建后 group_items 仍应引用 groups，并保留 ON DELETE CASCADE。
        let mut targets = Vec::new();
        let mut stmt = conn
            .prepare("PRAGMA foreign_key_list(group_items)")
            .unwrap();
        let rows = stmt.query_map([], |row| row.get::<_, String>(2)).unwrap();
        for row in rows {
            targets.push(row.unwrap());
        }
        assert!(targets.iter().any(|t| t == "groups"));
        conn.execute("DELETE FROM groups WHERE id = 1", []).unwrap();
        let remaining_items: i64 = conn
            .query_row("SELECT COUNT(*) FROM group_items", [], |row| row.get(0))
            .unwrap();
        assert_eq!(remaining_items, 0, "重建后 ON DELETE CASCADE 必须继续生效");
    }
}
