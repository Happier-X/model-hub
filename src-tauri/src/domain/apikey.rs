use rand::RngCore;
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::AppError;

use super::Stores;

const KEY_PREFIX: &str = "sk-modelhub-";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyPublic {
    pub id: i64,
    pub name: String,
    pub masked: String,
    pub enabled: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyCreated {
    pub id: i64,
    pub name: String,
    pub masked: String,
    pub enabled: bool,
    pub created_at: String,
    pub raw_key: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateApiKeyPayload {
    pub name: String,
    pub enabled: bool,
}

#[derive(Debug, Deserialize)]
pub struct UpdateApiKeyPayload {
    pub id: i64,
    pub name: String,
    pub enabled: bool,
}

pub fn hash_key(raw: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(raw.as_bytes());
    hex::encode(hasher.finalize())
}

pub fn mask_key(raw: &str) -> String {
    if raw.len() <= 12 {
        return "****".into();
    }
    format!("{}...{}", &raw[..12], &raw[raw.len().saturating_sub(4)..])
}

pub fn generate_raw_key() -> String {
    let mut bytes = [0u8; 24];
    rand::thread_rng().fill_bytes(&mut bytes);
    format!("{KEY_PREFIX}{}", hex::encode(bytes))
}

impl Stores {
    pub fn list_api_keys(&self) -> Result<Vec<ApiKeyPublic>, AppError> {
        self.with_conn(|conn| {
            let mut stmt = conn
                .prepare(
                    "SELECT id, name, masked, enabled, created_at FROM api_keys ORDER BY id ASC",
                )
                .map_err(|e| AppError::Database(e.to_string()))?;
            let rows = stmt
                .query_map([], |row| {
                    Ok(ApiKeyPublic {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        masked: row.get(2)?,
                        enabled: row.get::<_, i64>(3)? != 0,
                        created_at: row.get(4)?,
                    })
                })
                .map_err(|e| AppError::Database(e.to_string()))?;
            let mut out = Vec::new();
            for r in rows {
                out.push(r.map_err(|e| AppError::Database(e.to_string()))?);
            }
            Ok(out)
        })
    }

    pub fn create_api_key(&self, payload: CreateApiKeyPayload) -> Result<ApiKeyCreated, AppError> {
        let name = payload.name.trim().to_string();
        if name.is_empty() {
            return Err(AppError::Business("Key 名称不能为空".into()));
        }
        let raw = generate_raw_key();
        let key_hash = hash_key(&raw);
        let masked = mask_key(&raw);
        let created_at = chrono::Utc::now().to_rfc3339();
        let id = self.with_conn(|conn| {
            conn.execute(
                "INSERT INTO api_keys (name, key_hash, masked, enabled, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    name,
                    key_hash,
                    masked,
                    if payload.enabled { 1 } else { 0 },
                    created_at
                ],
            )
            .map_err(|e| AppError::Database(e.to_string()))?;
            Ok(conn.last_insert_rowid())
        })?;
        Ok(ApiKeyCreated {
            id,
            name,
            masked,
            enabled: payload.enabled,
            created_at,
            raw_key: raw,
        })
    }

    pub fn update_api_key(&self, payload: UpdateApiKeyPayload) -> Result<ApiKeyPublic, AppError> {
        let name = payload.name.trim().to_string();
        if name.is_empty() {
            return Err(AppError::Business("Key 名称不能为空".into()));
        }
        self.with_conn(|conn| {
            let n = conn
                .execute(
                    "UPDATE api_keys SET name=?1, enabled=?2 WHERE id=?3",
                    params![name, if payload.enabled { 1 } else { 0 }, payload.id],
                )
                .map_err(|e| AppError::Database(e.to_string()))?;
            if n == 0 {
                return Err(AppError::Business("API Key 不存在".into()));
            }
            Ok(())
        })?;
        self.list_api_keys()?
            .into_iter()
            .find(|k| k.id == payload.id)
            .ok_or_else(|| AppError::Business("API Key 不存在".into()))
    }

    pub fn delete_api_key(&self, id: i64) -> Result<(), AppError> {
        self.with_conn(|conn| {
            let n = conn
                .execute("DELETE FROM api_keys WHERE id = ?1", [id])
                .map_err(|e| AppError::Database(e.to_string()))?;
            if n == 0 {
                return Err(AppError::Business("API Key 不存在".into()));
            }
            Ok(())
        })
    }

    /// 校验原始客户端 Key；有效返回 true。
    pub fn validate_raw_key(&self, raw: &str) -> Result<bool, AppError> {
        let raw = raw.trim();
        if raw.is_empty() {
            return Ok(false);
        }
        let key_hash = hash_key(raw);
        self.with_conn(|conn| {
            let enabled: Option<i64> = conn
                .query_row(
                    "SELECT enabled FROM api_keys WHERE key_hash = ?1",
                    [key_hash],
                    |row| row.get(0),
                )
                .optional()
                .map_err(|e| AppError::Database(e.to_string()))?;
            Ok(enabled == Some(1))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::open_db;
    use tempfile::tempdir;

    #[test]
    fn create_and_validate_key() {
        let dir = tempdir().unwrap();
        let s = Stores::new(open_db(&dir.path().join("t.db")).unwrap());
        let created = s
            .create_api_key(CreateApiKeyPayload {
                name: "cli".into(),
                enabled: true,
            })
            .unwrap();
        assert!(created.raw_key.starts_with(KEY_PREFIX));
        assert!(s.validate_raw_key(&created.raw_key).unwrap());
        assert!(!s.validate_raw_key("sk-modelhub-bad").unwrap());
        s.update_api_key(UpdateApiKeyPayload {
            id: created.id,
            name: "cli".into(),
            enabled: false,
        })
        .unwrap();
        assert!(!s.validate_raw_key(&created.raw_key).unwrap());
    }
}
