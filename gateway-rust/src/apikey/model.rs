//! API Key 数据模型（对齐前端 ApiKey 字段）。

use serde::{Deserialize, Serialize};

/// 内部存储记录：不含完整明文 Key。
#[derive(Debug, Clone)]
pub struct ApiKeyRecord {
    pub id: i64,
    pub name: String,
    /// 脱敏展示值，如 `sk-octopus-****abcd`
    pub api_key_masked: String,
    pub key_hash: String,
    pub enabled: bool,
    pub expire_at: Option<String>,
    pub max_cost: Option<f64>,
    pub supported_models: Option<Vec<String>>,
}

/// 列表/更新后返回给管理端的公共视图。
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ApiKeyPublic {
    pub id: i64,
    pub name: String,
    pub api_key: String,
    pub enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expire_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_cost: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supported_models: Option<Vec<String>>,
}

impl From<&ApiKeyRecord> for ApiKeyPublic {
    fn from(value: &ApiKeyRecord) -> Self {
        Self {
            id: value.id,
            name: value.name.clone(),
            api_key: value.api_key_masked.clone(),
            enabled: value.enabled,
            expire_at: value.expire_at.clone(),
            max_cost: value.max_cost,
            supported_models: value.supported_models.clone(),
        }
    }
}

/// 创建成功：仅此响应包含完整明文 `api_key`。
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ApiKeyCreated {
    pub id: i64,
    pub name: String,
    pub api_key: String,
    pub enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expire_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_cost: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supported_models: Option<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateApiKeyRequest {
    pub name: String,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    pub expire_at: Option<String>,
    pub max_cost: Option<f64>,
    pub supported_models: Option<Vec<String>>,
}

fn default_enabled() -> bool {
    true
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateApiKeyRequest {
    pub id: i64,
    pub name: Option<String>,
    pub enabled: Option<bool>,
    pub expire_at: Option<String>,
    pub max_cost: Option<f64>,
    pub supported_models: Option<Vec<String>>,
}
