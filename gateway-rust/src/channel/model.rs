//! 渠道数据模型（对齐 UI channel.ts / smoke 契约）。

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BaseUrl {
    pub url: String,
    #[serde(default)]
    pub delay: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChannelKey {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel_id: Option<i64>,
    #[serde(default = "default_true")]
    pub enabled: bool,
    pub channel_key: String,
    #[serde(default)]
    pub remark: String,
}

fn default_true() -> bool {
    true
}

/// 列表/详情返回的完整渠道。
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Channel {
    pub id: i64,
    pub name: String,
    /// 数字类型枚举（UI 契约，非字符串）。
    #[serde(rename = "type")]
    pub channel_type: i64,
    pub enabled: bool,
    pub base_urls: Vec<BaseUrl>,
    pub keys: Vec<ChannelKey>,
    pub model: String,
    #[serde(default)]
    pub custom_model: String,
    #[serde(default)]
    pub proxy: bool,
    #[serde(default)]
    pub auto_sync: bool,
    #[serde(default)]
    pub auto_group: i64,
    #[serde(default)]
    pub custom_header: serde_json::Value,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateChannelRequest {
    pub name: String,
    #[serde(rename = "type")]
    pub channel_type: i64,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub base_urls: Vec<BaseUrl>,
    #[serde(default)]
    pub keys: Vec<ChannelKey>,
    #[serde(default)]
    pub model: String,
    #[serde(default)]
    pub custom_model: String,
    #[serde(default)]
    pub proxy: bool,
    #[serde(default)]
    pub auto_sync: bool,
    #[serde(default)]
    pub auto_group: i64,
    #[serde(default = "default_empty_array")]
    pub custom_header: serde_json::Value,
}

fn default_empty_array() -> serde_json::Value {
    serde_json::json!([])
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateChannelRequest {
    pub id: i64,
    pub name: Option<String>,
    #[serde(rename = "type")]
    pub channel_type: Option<i64>,
    pub enabled: Option<bool>,
    pub base_urls: Option<Vec<BaseUrl>>,
    pub model: Option<String>,
    pub custom_model: Option<String>,
    pub proxy: Option<bool>,
    pub auto_sync: Option<bool>,
    pub auto_group: Option<i64>,
    pub custom_header: Option<serde_json::Value>,
    #[serde(default)]
    pub keys_to_update: Vec<KeyUpdate>,
    #[serde(default)]
    pub keys_to_add: Vec<ChannelKey>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct KeyUpdate {
    pub id: i64,
    pub channel_key: String,
    pub enabled: Option<bool>,
    pub remark: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EnableChannelRequest {
    pub id: i64,
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChannelError {
    NotFound,
    InvalidName,
    InvalidInput(String),
    Internal,
}

impl std::fmt::Display for ChannelError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound => write!(f, "渠道不存在"),
            Self::InvalidName => write!(f, "渠道名称不能为空"),
            Self::InvalidInput(msg) => write!(f, "{msg}"),
            Self::Internal => write!(f, "内部存储错误"),
        }
    }
}
