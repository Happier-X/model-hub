//! SQLite 打开、路径解析与迁移。

mod migrate;

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use rusqlite::Connection;

use crate::config::DatabaseConfig;
use crate::error::GatewayError;

pub use migrate::migrate;

/// 共享的 SQLite 连接（短事务 + Mutex，避免跨 await 持锁）。
pub type DbConn = Arc<Mutex<Connection>>;

/// 解析相对路径：相对进程 cwd。
pub fn resolve_db_path(path: &str) -> PathBuf {
    let p = Path::new(path);
    if p.is_absolute() {
        p.to_path_buf()
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(p)
    }
}

/// 按配置打开/创建 SQLite，并执行迁移。仅支持 sqlite。
pub fn open_from_config(config: &DatabaseConfig) -> Result<DbConn, GatewayError> {
    if !config.db_type.trim().eq_ignore_ascii_case("sqlite") {
        return Err(GatewayError::invalid_config(format!(
            "仅支持 database.type = sqlite，当前为: {}",
            config.db_type
        )));
    }
    open_path(&config.path)
}

/// 打开指定路径（文件或 `:memory:`）。
pub fn open_path(path: &str) -> Result<DbConn, GatewayError> {
    let conn = if path == ":memory:" {
        Connection::open_in_memory()
            .map_err(|e| GatewayError::database(format!("打开内存库失败: {e}")))?
    } else {
        let resolved = resolve_db_path(path);
        if let Some(parent) = resolved.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent).map_err(|e| {
                    GatewayError::database(format!(
                        "创建数据库目录失败 ({}): {e}",
                        parent.display()
                    ))
                })?;
            }
        }
        Connection::open(&resolved).map_err(|e| {
            GatewayError::database(format!("打开数据库失败 ({}): {e}", resolved.display()))
        })?
    };

    conn.execute_batch("PRAGMA foreign_keys = ON;")
        .map_err(|e| GatewayError::database(format!("启用 foreign_keys 失败: {e}")))?;

    migrate(&conn)?;

    Ok(Arc::new(Mutex::new(conn)))
}

/// 测试用：临时文件库（多连接场景比 :memory: 更稳）。
#[cfg(test)]
pub fn open_tempfile_db() -> (DbConn, tempfile::TempDir) {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("test.db");
    let conn = open_path(path.to_str().expect("utf8 path")).expect("open temp db");
    (conn, dir)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::DatabaseConfig;

    #[test]
    fn rejects_non_sqlite() {
        let cfg = DatabaseConfig {
            db_type: "mysql".into(),
            path: "data/data.db".into(),
        };
        let err = open_from_config(&cfg).unwrap_err();
        assert!(matches!(err, GatewayError::ConfigInvalid { .. }));
    }

    #[test]
    fn open_file_and_migrate() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("data.db");
        let conn = open_path(path.to_str().unwrap()).unwrap();
        {
            let guard = conn.lock().unwrap();
            let n: i64 = guard
                .query_row(
                    "SELECT COUNT(*) FROM schema_migrations WHERE version = 1",
                    [],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(n, 1);
        }
        // 再次打开幂等
        let _conn2 = open_path(path.to_str().unwrap()).unwrap();
    }
}
