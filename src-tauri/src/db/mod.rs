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
    fn open_db_upgrades_legacy_groups_and_preserves_items() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("legacy.db");
        let conn = Connection::open(&path).unwrap();
        conn.execute_batch(
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
            INSERT INTO providers (name, base_url, created_at)
            VALUES ('legacy-provider', 'https://example.com/v1', '2024-01-01T00:00:00Z');
            INSERT INTO groups (name)
            VALUES ('legacy-group');
            INSERT INTO group_items (group_id, provider_id, upstream_model, sort_order)
            VALUES (1, 1, 'legacy-model', 0);",
        )
        .unwrap();
        drop(conn);

        let stores = Stores::new(open_db(&path).unwrap());
        let groups = stores.list_groups().unwrap();

        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].name, "legacy-group");
        assert!(groups[0].auto_failover);
        assert!(!groups[0].created_at.is_empty());
        chrono::DateTime::parse_from_rfc3339(&groups[0].created_at).unwrap();
        assert_eq!(groups[0].items.len(), 1);
        assert_eq!(groups[0].items[0].upstream_model, "legacy-model");
        let group = stores.get_group_by_name("legacy-group").unwrap().unwrap();
        assert_eq!(group.items.len(), 1);
        assert_eq!(group.items[0].provider_name.as_deref(), Some("legacy-provider"));
    }
}
