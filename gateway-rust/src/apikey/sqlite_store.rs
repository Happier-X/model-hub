//! SQLite 实现的 ApiKeyStore：仅存哈希 + 脱敏，不存完整 Key。

use rusqlite::{params, OptionalExtension};

use super::model::{
    ApiKeyCreated, ApiKeyPublic, ApiKeyRecord, CreateApiKeyRequest, UpdateApiKeyRequest,
};
use super::service::{generate_raw_key, hash_key, hashes_equal, mask_key};
use super::store::{ApiKeyStore, ApiKeyStoreError};
use crate::db::DbConn;

/// 基于 SQLite 的 API Key 存储。
#[derive(Clone)]
pub struct SqliteApiKeyStore {
    db: DbConn,
}

impl SqliteApiKeyStore {
    pub fn new(db: DbConn) -> Self {
        Self { db }
    }

    fn with_conn<T>(
        &self,
        f: impl FnOnce(&rusqlite::Connection) -> Result<T, ApiKeyStoreError>,
    ) -> Result<T, ApiKeyStoreError> {
        let guard = self.db.lock().map_err(|_| ApiKeyStoreError::Internal)?;
        f(&guard)
    }

    fn row_to_record(row: &rusqlite::Row<'_>) -> rusqlite::Result<ApiKeyRecord> {
        let supported_models_json: Option<String> = row.get(7)?;
        let supported_models = match supported_models_json {
            Some(s) if !s.is_empty() => serde_json::from_str(&s).ok(),
            _ => None,
        };
        Ok(ApiKeyRecord {
            id: row.get(0)?,
            name: row.get(1)?,
            api_key_masked: row.get(2)?,
            key_hash: row.get(3)?,
            enabled: row.get::<_, i64>(4)? != 0,
            expire_at: row.get(5)?,
            max_cost: row.get(6)?,
            supported_models,
        })
    }
}

impl ApiKeyStore for SqliteApiKeyStore {
    fn list(&self) -> Vec<ApiKeyPublic> {
        self.with_conn(|conn| {
            let mut stmt = conn
                .prepare(
                    "SELECT id, name, api_key_masked, key_hash, enabled, expire_at, max_cost, supported_models_json
                     FROM api_keys ORDER BY id ASC",
                )
                .map_err(|_| ApiKeyStoreError::Internal)?;
            let rows = stmt
                .query_map([], Self::row_to_record)
                .map_err(|_| ApiKeyStoreError::Internal)?;
            let mut out = Vec::new();
            for rec in rows.flatten() {
                out.push(ApiKeyPublic::from(&rec));
            }
            Ok(out)
        })
        .unwrap_or_default()
    }

    fn create(&self, req: CreateApiKeyRequest) -> Result<ApiKeyCreated, ApiKeyStoreError> {
        let name = req.name.trim().to_string();
        if name.is_empty() {
            return Err(ApiKeyStoreError::InvalidName);
        }

        let raw = generate_raw_key();
        let key_hash = hash_key(&raw);
        let masked = mask_key(&raw);
        let models_json = req
            .supported_models
            .as_ref()
            .map(|m| serde_json::to_string(m).unwrap_or_else(|_| "[]".into()));

        let id = self.with_conn(|conn| {
            conn.execute(
                "INSERT INTO api_keys (name, api_key_masked, key_hash, enabled, expire_at, max_cost, supported_models_json)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    name,
                    masked,
                    key_hash,
                    if req.enabled { 1 } else { 0 },
                    req.expire_at,
                    req.max_cost,
                    models_json,
                ],
            )
            .map_err(|_| ApiKeyStoreError::Internal)?;
            Ok(conn.last_insert_rowid())
        })?;

        debug_assert!(!key_hash.starts_with("sk-modelhub-"));
        debug_assert_ne!(masked, raw);

        Ok(ApiKeyCreated {
            id,
            name,
            api_key: raw,
            enabled: req.enabled,
            expire_at: req.expire_at,
            max_cost: req.max_cost,
            supported_models: req.supported_models,
        })
    }

    fn update(&self, req: UpdateApiKeyRequest) -> Result<ApiKeyPublic, ApiKeyStoreError> {
        self.with_conn(|conn| {
            let mut rec = conn
                .query_row(
                    "SELECT id, name, api_key_masked, key_hash, enabled, expire_at, max_cost, supported_models_json
                     FROM api_keys WHERE id = ?1",
                    [req.id],
                    Self::row_to_record,
                )
                .optional()
                .map_err(|_| ApiKeyStoreError::Internal)?
                .ok_or(ApiKeyStoreError::NotFound)?;

            if let Some(name) = req.name {
                let name = name.trim().to_string();
                if name.is_empty() {
                    return Err(ApiKeyStoreError::InvalidName);
                }
                rec.name = name;
            }
            if let Some(enabled) = req.enabled {
                rec.enabled = enabled;
            }
            if req.expire_at.is_some() {
                rec.expire_at = req.expire_at;
            }
            if req.max_cost.is_some() {
                rec.max_cost = req.max_cost;
            }
            if req.supported_models.is_some() {
                rec.supported_models = req.supported_models;
            }

            let models_json = rec
                .supported_models
                .as_ref()
                .map(|m| serde_json::to_string(m).unwrap_or_else(|_| "[]".into()));

            conn.execute(
                "UPDATE api_keys SET name=?1, enabled=?2, expire_at=?3, max_cost=?4, supported_models_json=?5
                 WHERE id=?6",
                params![
                    rec.name,
                    if rec.enabled { 1 } else { 0 },
                    rec.expire_at,
                    rec.max_cost,
                    models_json,
                    rec.id,
                ],
            )
            .map_err(|_| ApiKeyStoreError::Internal)?;

            Ok(ApiKeyPublic::from(&rec))
        })
    }

    fn delete(&self, id: i64) -> Result<(), ApiKeyStoreError> {
        self.with_conn(|conn| {
            let n = conn
                .execute("DELETE FROM api_keys WHERE id = ?1", [id])
                .map_err(|_| ApiKeyStoreError::Internal)?;
            if n == 0 {
                return Err(ApiKeyStoreError::NotFound);
            }
            Ok(())
        })
    }

    fn find_by_raw_key(&self, raw: &str) -> Option<ApiKeyRecord> {
        let target = hash_key(raw);
        self.with_conn(|conn| {
            let mut stmt = conn
                .prepare(
                    "SELECT id, name, api_key_masked, key_hash, enabled, expire_at, max_cost, supported_models_json
                     FROM api_keys",
                )
                .map_err(|_| ApiKeyStoreError::Internal)?;
            let rows = stmt
                .query_map([], Self::row_to_record)
                .map_err(|_| ApiKeyStoreError::Internal)?;
            for row in rows.flatten() {
                if hashes_equal(&row.key_hash, &target) {
                    return Ok(Some(row));
                }
            }
            Ok(None)
        })
        .ok()
        .flatten()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::open_tempfile_db;

    #[test]
    fn persist_across_store_instances() {
        let (db, _dir) = open_tempfile_db();
        let store = SqliteApiKeyStore::new(db.clone());
        let created = store
            .create(CreateApiKeyRequest {
                name: "persist".into(),
                enabled: true,
                expire_at: None,
                max_cost: None,
                supported_models: None,
            })
            .unwrap();
        assert!(created.api_key.starts_with("sk-modelhub-"));

        // 新 store 实例读同一库
        let store2 = SqliteApiKeyStore::new(db);
        let list = store2.list();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, created.id);
        assert_ne!(list[0].api_key, created.api_key);
        assert!(list[0].api_key.contains("****"));

        let found = store2.find_by_raw_key(&created.api_key).unwrap();
        assert_eq!(found.id, created.id);
        assert!(!format!("{found:?}").contains(&created.api_key));
    }

    #[test]
    fn update_delete_roundtrip() {
        let (db, _dir) = open_tempfile_db();
        let store = SqliteApiKeyStore::new(db);
        let created = store
            .create(CreateApiKeyRequest {
                name: "a".into(),
                enabled: true,
                expire_at: None,
                max_cost: None,
                supported_models: Some(vec!["gpt-4o".into()]),
            })
            .unwrap();

        let updated = store
            .update(UpdateApiKeyRequest {
                id: created.id,
                name: Some("b".into()),
                enabled: Some(false),
                expire_at: None,
                max_cost: Some(10.0),
                supported_models: None,
            })
            .unwrap();
        assert_eq!(updated.name, "b");
        assert!(!updated.enabled);

        store.delete(created.id).unwrap();
        assert!(store.list().is_empty());
        assert!(store.find_by_raw_key(&created.api_key).is_none());
        assert_eq!(store.delete(created.id), Err(ApiKeyStoreError::NotFound));
    }
}
