//! 分组数据模型（对齐 UI group.ts / smoke 契约）。

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GroupItem {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group_id: Option<i64>,
    pub channel_id: i64,
    pub model_name: String,
    #[serde(default = "default_one")]
    pub priority: i64,
    #[serde(default = "default_one")]
    pub weight: i64,
}

fn default_one() -> i64 {
    1
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Group {
    pub id: i64,
    pub name: String,
    /// 数字 mode；1 = 轮询
    pub mode: i64,
    pub match_regex: String,
    pub items: Vec<GroupItem>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateGroupRequest {
    pub name: String,
    #[serde(default = "default_mode")]
    pub mode: i64,
    #[serde(default)]
    pub match_regex: String,
    #[serde(default)]
    pub items: Vec<GroupItem>,
}

fn default_mode() -> i64 {
    1
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateGroupRequest {
    pub id: i64,
    pub name: Option<String>,
    pub mode: Option<i64>,
    pub match_regex: Option<String>,
    #[serde(default)]
    pub items_to_delete: Vec<i64>,
    #[serde(default)]
    pub items_to_add: Vec<GroupItem>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GroupError {
    NotFound,
    InvalidName,
    InvalidInput(String),
    Internal,
}

impl std::fmt::Display for GroupError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound => write!(f, "分组不存在"),
            Self::InvalidName => write!(f, "分组名称不能为空"),
            Self::InvalidInput(msg) => write!(f, "{msg}"),
            Self::Internal => write!(f, "内部存储错误"),
        }
    }
}
