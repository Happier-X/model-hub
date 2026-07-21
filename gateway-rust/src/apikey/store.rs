//! ApiKeyStore trait 与内存实现。

use std::sync::{Arc, Mutex};

use super::model::{
    ApiKeyCreated, ApiKeyPublic, ApiKeyRecord, CreateApiKeyRequest, UpdateApiKeyRequest,
};
use super::service::{generate_raw_key, hash_key, hashes_equal, mask_key};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApiKeyStoreError {
    NotFound,
    InvalidName,
    /// 数据库/锁等内部错误
    Internal,
}

impl std::fmt::Display for ApiKeyStoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound => write!(f, "API Key 不存在"),
            Self::InvalidName => write!(f, "名称不能为空"),
            Self::Internal => write!(f, "内部存储错误"),
        }
    }
}

pub trait ApiKeyStore: Send + Sync {
    fn list(&self) -> Vec<ApiKeyPublic>;
    fn create(&self, req: CreateApiKeyRequest) -> Result<ApiKeyCreated, ApiKeyStoreError>;
    fn update(&self, req: UpdateApiKeyRequest) -> Result<ApiKeyPublic, ApiKeyStoreError>;
    fn delete(&self, id: i64) -> Result<(), ApiKeyStoreError>;
    fn find_by_raw_key(&self, raw: &str) -> Option<ApiKeyRecord>;
}

#[derive(Debug, Default)]
struct MemoryInner {
    next_id: i64,
    records: Vec<ApiKeyRecord>,
}

/// 进程内内存存储；完整 Key 永不写入 records。
#[derive(Debug, Clone, Default)]
pub struct MemoryApiKeyStore {
    inner: Arc<Mutex<MemoryInner>>,
}

impl MemoryApiKeyStore {
    pub fn new() -> Self {
        Self::default()
    }
}

impl ApiKeyStore for MemoryApiKeyStore {
    fn list(&self) -> Vec<ApiKeyPublic> {
        let guard = self.inner.lock().expect("api key store lock");
        guard.records.iter().map(ApiKeyPublic::from).collect()
    }

    fn create(&self, req: CreateApiKeyRequest) -> Result<ApiKeyCreated, ApiKeyStoreError> {
        let name = req.name.trim().to_string();
        if name.is_empty() {
            return Err(ApiKeyStoreError::InvalidName);
        }

        let raw = generate_raw_key();
        let key_hash = hash_key(&raw);
        let masked = mask_key(&raw);

        let mut guard = self.inner.lock().expect("api key store lock");
        guard.next_id += 1;
        let id = guard.next_id;
        let record = ApiKeyRecord {
            id,
            name: name.clone(),
            api_key_masked: masked,
            key_hash,
            enabled: req.enabled,
            expire_at: req.expire_at.clone(),
            max_cost: req.max_cost,
            supported_models: req.supported_models.clone(),
        };
        // 断言：存储字段不含完整 raw
        debug_assert!(!record.key_hash.starts_with("sk-modelhub-"));
        debug_assert_ne!(record.api_key_masked, raw);

        guard.records.push(record);

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
        let mut guard = self.inner.lock().expect("api key store lock");
        let record = guard
            .records
            .iter_mut()
            .find(|r| r.id == req.id)
            .ok_or(ApiKeyStoreError::NotFound)?;

        if let Some(name) = req.name {
            let name = name.trim().to_string();
            if name.is_empty() {
                return Err(ApiKeyStoreError::InvalidName);
            }
            record.name = name;
        }
        if let Some(enabled) = req.enabled {
            record.enabled = enabled;
        }
        if req.expire_at.is_some() {
            record.expire_at = req.expire_at;
        }
        if req.max_cost.is_some() {
            record.max_cost = req.max_cost;
        }
        if req.supported_models.is_some() {
            record.supported_models = req.supported_models;
        }

        Ok(ApiKeyPublic::from(&*record))
    }

    fn delete(&self, id: i64) -> Result<(), ApiKeyStoreError> {
        let mut guard = self.inner.lock().expect("api key store lock");
        let before = guard.records.len();
        guard.records.retain(|r| r.id != id);
        if guard.records.len() == before {
            return Err(ApiKeyStoreError::NotFound);
        }
        Ok(())
    }

    fn find_by_raw_key(&self, raw: &str) -> Option<ApiKeyRecord> {
        let target = hash_key(raw);
        let guard = self.inner.lock().expect("api key store lock");
        guard
            .records
            .iter()
            .find(|r| hashes_equal(&r.key_hash, &target))
            .cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_list_masks_and_find_works() {
        let store = MemoryApiKeyStore::new();
        let created = store
            .create(CreateApiKeyRequest {
                name: "local".into(),
                enabled: true,
                expire_at: None,
                max_cost: None,
                supported_models: None,
            })
            .unwrap();
        assert!(created.api_key.starts_with("sk-modelhub-"));

        let list = store.list();
        assert_eq!(list.len(), 1);
        assert_ne!(list[0].api_key, created.api_key);
        assert!(list[0].api_key.contains("****"));

        // 内部 records 不含完整 key
        {
            let guard = store.inner.lock().unwrap();
            let rec = &guard.records[0];
            assert_ne!(rec.api_key_masked, created.api_key);
            assert!(!rec.key_hash.contains("sk-modelhub-"));
            assert!(!format!("{rec:?}").contains(&created.api_key));
        }

        let found = store.find_by_raw_key(&created.api_key).unwrap();
        assert_eq!(found.id, created.id);
        assert!(store.find_by_raw_key("sk-modelhub-nope").is_none());
    }

    #[test]
    fn update_and_delete() {
        let store = MemoryApiKeyStore::new();
        let created = store
            .create(CreateApiKeyRequest {
                name: "a".into(),
                enabled: true,
                expire_at: None,
                max_cost: None,
                supported_models: None,
            })
            .unwrap();

        let updated = store
            .update(UpdateApiKeyRequest {
                id: created.id,
                name: Some("b".into()),
                enabled: Some(false),
                expire_at: None,
                max_cost: None,
                supported_models: None,
            })
            .unwrap();
        assert_eq!(updated.name, "b");
        assert!(!updated.enabled);
        // 禁用后 find 仍能找到记录（鉴权层再判断 enabled）
        let rec = store.find_by_raw_key(&created.api_key).unwrap();
        assert!(!rec.enabled);

        store.delete(created.id).unwrap();
        assert!(store.list().is_empty());
        assert!(store.find_by_raw_key(&created.api_key).is_none());
    }
}
