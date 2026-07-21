use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};

use crate::error::AppError;

use super::Stores;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Provider {
    pub id: i64,
    pub name: String,
    pub base_url: String,
    pub api_key: String,
    pub enabled: bool,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateProviderPayload {
    pub name: String,
    pub base_url: String,
    pub api_key: String,
    pub enabled: bool,
}

#[derive(Debug, Deserialize)]
pub struct UpdateProviderPayload {
    pub id: i64,
    pub name: String,
    pub base_url: String,
    pub api_key: String,
    pub enabled: bool,
}

fn map_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Provider> {
    Ok(Provider {
        id: row.get(0)?,
        name: row.get(1)?,
        base_url: row.get(2)?,
        api_key: row.get(3)?,
        enabled: row.get::<_, i64>(4)? != 0,
        created_at: row.get(5)?,
    })
}

impl Stores {
    pub fn list_providers(&self) -> Result<Vec<Provider>, AppError> {
        self.with_conn(|conn| {
            let mut stmt = conn
                .prepare(
                    "SELECT id, name, base_url, api_key, enabled, created_at FROM providers ORDER BY id ASC",
                )
                .map_err(|e| AppError::Database(e.to_string()))?;
            let rows = stmt
                .query_map([], map_row)
                .map_err(|e| AppError::Database(e.to_string()))?;
            let mut out = Vec::new();
            for r in rows {
                out.push(r.map_err(|e| AppError::Database(e.to_string()))?);
            }
            Ok(out)
        })
    }

    pub fn get_provider(&self, id: i64) -> Result<Option<Provider>, AppError> {
        self.with_conn(|conn| {
            conn.query_row(
                "SELECT id, name, base_url, api_key, enabled, created_at FROM providers WHERE id = ?1",
                [id],
                map_row,
            )
            .optional()
            .map_err(|e| AppError::Database(e.to_string()))
        })
    }

    pub fn create_provider(&self, payload: CreateProviderPayload) -> Result<Provider, AppError> {
        let name = payload.name.trim().to_string();
        let base_url = payload.base_url.trim().trim_end_matches('/').to_string();
        if name.is_empty() {
            return Err(AppError::Business("供应商名称不能为空".into()));
        }
        if base_url.is_empty() {
            return Err(AppError::Business("Base URL 不能为空".into()));
        }
        let created_at = chrono::Utc::now().to_rfc3339();
        self.with_conn(|conn| {
            conn.execute(
                "INSERT INTO providers (name, base_url, api_key, enabled, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    name,
                    base_url,
                    payload.api_key,
                    if payload.enabled { 1 } else { 0 },
                    created_at
                ],
            )
            .map_err(|e| AppError::Database(e.to_string()))?;
            let id = conn.last_insert_rowid();
            Ok(Provider {
                id,
                name,
                base_url,
                api_key: payload.api_key,
                enabled: payload.enabled,
                created_at,
            })
        })
    }

    pub fn update_provider(&self, payload: UpdateProviderPayload) -> Result<Provider, AppError> {
        let name = payload.name.trim().to_string();
        let base_url = payload.base_url.trim().trim_end_matches('/').to_string();
        if name.is_empty() {
            return Err(AppError::Business("供应商名称不能为空".into()));
        }
        if base_url.is_empty() {
            return Err(AppError::Business("Base URL 不能为空".into()));
        }
        self.with_conn(|conn| {
            let n = conn
                .execute(
                    "UPDATE providers SET name=?1, base_url=?2, api_key=?3, enabled=?4 WHERE id=?5",
                    params![
                        name,
                        base_url,
                        payload.api_key,
                        if payload.enabled { 1 } else { 0 },
                        payload.id
                    ],
                )
                .map_err(|e| AppError::Database(e.to_string()))?;
            if n == 0 {
                return Err(AppError::Business("供应商不存在".into()));
            }
            Ok(())
        })?;
        self.get_provider(payload.id)?
            .ok_or_else(|| AppError::Business("供应商不存在".into()))
    }

    pub fn delete_provider(&self, id: i64) -> Result<(), AppError> {
        self.with_conn(|conn| {
            let n = conn
                .execute("DELETE FROM providers WHERE id = ?1", [id])
                .map_err(|e| AppError::Database(e.to_string()))?;
            if n == 0 {
                return Err(AppError::Business("供应商不存在".into()));
            }
            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::open_db;
    use tempfile::tempdir;

    fn stores() -> Stores {
        let dir = tempdir().unwrap();
        let db = open_db(&dir.path().join("t.db")).unwrap();
        // leak tempdir for test duration via forget - use open_in_memory style
        std::mem::forget(dir);
        Stores::new(db)
    }

    #[test]
    fn provider_crud() {
        let s = stores();
        let p = s
            .create_provider(CreateProviderPayload {
                name: "p1".into(),
                base_url: "https://api.example.com/v1/".into(),
                api_key: "k".into(),
                enabled: true,
            })
            .unwrap();
        assert_eq!(p.base_url, "https://api.example.com/v1");
        let list = s.list_providers().unwrap();
        assert_eq!(list.len(), 1);
        s.update_provider(UpdateProviderPayload {
            id: p.id,
            name: "p2".into(),
            base_url: p.base_url,
            api_key: "k2".into(),
            enabled: false,
        })
        .unwrap();
        s.delete_provider(p.id).unwrap();
        assert!(s.list_providers().unwrap().is_empty());
    }
}
