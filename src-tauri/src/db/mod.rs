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
}
