//! 源库类型检测。

use std::path::Path;

use rusqlite::Connection;

use crate::error::GatewayError;

/// 源库分类。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceKind {
    /// octopus 风格（含 base_urls 列与 users/migration_records 等特征）。
    Octopus,
    /// 已是 gateway-rust schema。
    RustGateway,
    /// 无法识别。
    Unknown,
}

/// 打开源库（只读优先）并检测类型。
pub fn detect_source(path: &Path) -> Result<SourceKind, GatewayError> {
    if !path.exists() {
        return Err(GatewayError::database(format!(
            "源库不存在: {}",
            path.display()
        )));
    }

    // 使用 URI 只读打开，避免误写源库
    let uri = format!("file:{}?mode=ro", path_to_uri(path));
    let conn = Connection::open_with_flags(
        &uri,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_URI,
    )
    .or_else(|_| {
        // 部分环境 URI 路径失败时回退普通只读
        Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
    })
    .map_err(|e| GatewayError::database(format!("打开源库失败 ({}): {e}", path.display())))?;

    Ok(classify_connection(&conn))
}

/// 根据已打开连接分类（供测试）。
pub fn classify_connection(conn: &Connection) -> SourceKind {
    let has_schema_migrations = table_exists(conn, "schema_migrations");
    let has_migration_records = table_exists(conn, "migration_records");
    let has_users = table_exists(conn, "users");
    let has_channels = table_exists(conn, "channels");
    let channels_has_base_urls = has_channels && column_exists(conn, "channels", "base_urls");
    let has_channel_base_urls = table_exists(conn, "channel_base_urls");

    let octopus_like =
        (has_migration_records || has_users) && channels_has_base_urls && has_channels;

    // 优先识别 octopus；否则有 schema_migrations 或规范化 channel_base_urls 表视为 rust 库
    if octopus_like {
        SourceKind::Octopus
    } else if has_schema_migrations || (has_channel_base_urls && !channels_has_base_urls) {
        SourceKind::RustGateway
    } else {
        SourceKind::Unknown
    }
}

fn table_exists(conn: &Connection, name: &str) -> bool {
    conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?1",
        [name],
        |row| row.get::<_, i64>(0),
    )
    .map(|n| n > 0)
    .unwrap_or(false)
}

fn column_exists(conn: &Connection, table: &str, column: &str) -> bool {
    let mut stmt = match conn.prepare(&format!("PRAGMA table_info({table})")) {
        Ok(s) => s,
        Err(_) => return false,
    };
    let rows = match stmt.query_map([], |row| row.get::<_, String>(1)) {
        Ok(r) => r,
        Err(_) => return false,
    };
    for name in rows.flatten() {
        if name == column {
            return true;
        }
    }
    false
}

/// Windows 路径转 SQLite URI 片段（尽量兼容）。
fn path_to_uri(path: &Path) -> String {
    let s = path.to_string_lossy().replace('\\', "/");
    // file:/C:/... 形式；空格编码
    s.replace(' ', "%20")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::migrate;
    use rusqlite::Connection;

    #[test]
    fn classify_empty_unknown() {
        let conn = Connection::open_in_memory().unwrap();
        assert_eq!(classify_connection(&conn), SourceKind::Unknown);
    }

    #[test]
    fn classify_rust_after_migrate() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
        migrate(&conn).unwrap();
        assert_eq!(classify_connection(&conn), SourceKind::RustGateway);
    }

    #[test]
    fn classify_octopus_mini() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            r#"
            CREATE TABLE users (id INTEGER PRIMARY KEY, username TEXT, password TEXT);
            CREATE TABLE channels (
              id INTEGER PRIMARY KEY,
              name TEXT,
              type INTEGER,
              enabled INTEGER,
              base_urls TEXT,
              model TEXT
            );
            CREATE TABLE migration_records (version INTEGER PRIMARY KEY, status INTEGER);
            "#,
        )
        .unwrap();
        assert_eq!(classify_connection(&conn), SourceKind::Octopus);
    }
}
