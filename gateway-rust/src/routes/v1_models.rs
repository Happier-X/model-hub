//! 客户端 OpenAI 兼容 `/v1/models` 占位。

use axum::Json;
use serde::Serialize;

use crate::auth::ClientAuth;

#[derive(Debug, Serialize)]
pub struct ModelsListResponse {
    pub object: &'static str,
    pub data: Vec<serde_json::Value>,
}

/// 鉴权通过后返回空列表；不使用管理信封。
pub async fn models_handler(_auth: ClientAuth) -> Json<ModelsListResponse> {
    Json(ModelsListResponse {
        object: "list",
        data: vec![],
    })
}
