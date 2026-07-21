pub mod apikey;
pub mod group;
pub mod log;
pub mod provider;

use crate::db::DbConn;
use crate::error::AppError;

#[derive(Clone)]
pub struct Stores {
    pub db: DbConn,
}

impl Stores {
    pub fn new(db: DbConn) -> Self {
        Self { db }
    }

    pub fn with_conn<T>(&self, f: impl FnOnce(&rusqlite::Connection) -> Result<T, AppError>) -> Result<T, AppError> {
        let guard = self.db.lock().map_err(|_| AppError::LockPoisoned)?;
        f(&guard)
    }
}
