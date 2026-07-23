//! SQLite 连接与迁移。

mod migrate;

use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use rusqlite::Connection;

use crate::error::AppError;

pub use migrate::migrate;

pub type DbConn = Arc<Mutex<Connection>>;

pub fn open_db(db_path: &Path) -> Result<DbConn, AppError> {
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent).map_err(|source| AppError::CreateDirectory {
            path: parent.display().to_string(),
            source,
        })?;
    }
    let conn = Connection::open(db_path).map_err(|e| AppError::Database(e.to_string()))?;
    conn.execute_batch("PRAGMA foreign_keys = ON;")
        .map_err(|e| AppError::Database(e.to_string()))?;
    // 代理写日志与 UI 读库并发时减少 SQLITE_BUSY 静默失败。
    conn.busy_timeout(std::time::Duration::from_secs(5))
        .map_err(|e| AppError::Database(e.to_string()))?;
    let _ = conn.pragma_update(None, "journal_mode", "WAL");
    migrate(&conn)?;
    Ok(Arc::new(Mutex::new(conn)))
}

pub fn default_db_path(gateway_dir: &str) -> PathBuf {
    PathBuf::from(gateway_dir).join("data").join("data.db")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::Stores;
    use tempfile::tempdir;

    #[test]
    fn open_and_migrate_twice() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("data.db");
        let db = open_db(&path).unwrap();
        {
            let conn = db.lock().unwrap();
            migrate(&conn).unwrap();
        }
    }

    #[test]
    fn open_db_upgrades_confirmed_legacy_schema_and_all_domain_reads_succeed() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("legacy.db");
        let conn = Connection::open(&path).unwrap();
        let recent_ts = chrono::Utc::now().timestamp() - 3600;
        conn.execute_batch(&format!(
            "CREATE TABLE providers (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                base_url TEXT NOT NULL,
                api_key TEXT NOT NULL DEFAULT '',
                enabled INTEGER NOT NULL DEFAULT 1,
                created_at TEXT NOT NULL
            );
            CREATE TABLE groups (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL UNIQUE
            );
            CREATE TABLE group_items (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                group_id INTEGER NOT NULL REFERENCES groups(id) ON DELETE CASCADE,
                provider_id INTEGER NOT NULL REFERENCES providers(id) ON DELETE CASCADE,
                upstream_model TEXT NOT NULL,
                sort_order INTEGER NOT NULL DEFAULT 0
            );
            CREATE TABLE request_logs (
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
            INSERT INTO providers
                (name, base_url, api_key, enabled, created_at)
            VALUES
                ('legacy-provider', 'https://example.com/v1', 'legacy-secret', 0,
                 '2024-01-01T00:00:00Z');
            INSERT INTO groups (name) VALUES ('legacy-group');
            INSERT INTO group_items
                (group_id, provider_id, upstream_model, sort_order)
            VALUES (1, 1, 'legacy-model', 7);
            INSERT INTO request_logs
                (time, group_name, provider_name, upstream_model, status_code,
                 use_time_ms, error, failover_from, failover_to, failover_reason)
            VALUES
                -- time 用近期秒级时间戳，避免 list_logs 默认 30 天保留策略清掉测试行
                ({recent_ts}, 'legacy-group', 'legacy-provider', 'legacy-model', 503,
                 321, 'upstream unavailable', 'legacy-provider', 'backup-provider',
                 'server error');"
        ))
        .unwrap();
        drop(conn);

        let stores = Stores::new(open_db(&path).unwrap());
        let groups = stores.list_groups().unwrap();
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].name, "legacy-group");
        assert!(groups[0].auto_failover);
        chrono::DateTime::parse_from_rfc3339(&groups[0].created_at).unwrap();
        assert_eq!(groups[0].items.len(), 1);
        assert_eq!(groups[0].items[0].upstream_model, "legacy-model");
        assert_eq!(groups[0].items[0].sort_order, 7);

        let migrated_created_at = groups[0].created_at.clone();
        {
            let conn = stores.db.lock().unwrap();
            migrate(&conn).unwrap();
        }
        let group = stores.get_group_by_name("legacy-group").unwrap().unwrap();
        assert_eq!(group.created_at, migrated_created_at);
        assert_eq!(group.items.len(), 1);
        assert_eq!(
            group.items[0].provider_name.as_deref(),
            Some("legacy-provider")
        );

        let providers = stores.list_providers().unwrap();
        assert_eq!(providers.len(), 1);
        assert_eq!(providers[0].api_key, "legacy-secret");
        assert!(!providers[0].enabled);
        let provider = stores.get_provider(1).unwrap().unwrap();
        assert_eq!(provider.created_at, "2024-01-01T00:00:00Z");

        let logs = stores
            .list_logs(crate::domain::log::LogQuery {
                page: 1,
                page_size: 100,
                ..Default::default()
            })
            .unwrap();
        assert_eq!(logs.total, 1);
        assert_eq!(logs.items.len(), 1);
        assert_eq!(logs.items[0].status_code, 503);
        assert_eq!(logs.items[0].use_time_ms, 321);
        assert_eq!(logs.items[0].failover_to, "backup-provider");
    }
}
