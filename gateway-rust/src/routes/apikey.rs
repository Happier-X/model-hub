//! 管理端 API Key CRUD。

use axum::extract::{Path, State};
use axum::response::{IntoResponse, Response};
use axum::Json;

use crate::apikey::{ApiKeyStoreError, CreateApiKeyRequest, UpdateApiKeyRequest};
use crate::auth::AdminAuth;
use crate::http::AppState;
use crate::response::{bad_request, not_found_api, DataEnvelope};

pub async fn list_apikey_handler(
    _auth: AdminAuth,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let list = state.api_keys.list();
    DataEnvelope::new(list)
}

pub async fn create_apikey_handler(
    _auth: AdminAuth,
    State(state): State<AppState>,
    Json(body): Json<CreateApiKeyRequest>,
) -> Response {
    match state.api_keys.create(body) {
        Ok(created) => {
            // 仅日志记录 id/name，禁止完整 key
            tracing::info!(id = created.id, name = %created.name, "已创建 API Key");
            DataEnvelope::new(created).into_response()
        }
        Err(ApiKeyStoreError::InvalidName) => bad_request("名称不能为空"),
        Err(ApiKeyStoreError::NotFound) => not_found_api("API Key 不存在"),
    }
}

pub async fn update_apikey_handler(
    _auth: AdminAuth,
    State(state): State<AppState>,
    Json(body): Json<UpdateApiKeyRequest>,
) -> Response {
    match state.api_keys.update(body) {
        Ok(updated) => {
            tracing::info!(id = updated.id, "已更新 API Key");
            DataEnvelope::new(updated).into_response()
        }
        Err(ApiKeyStoreError::NotFound) => not_found_api("API Key 不存在"),
        Err(ApiKeyStoreError::InvalidName) => bad_request("名称不能为空"),
    }
}

pub async fn delete_apikey_handler(
    _auth: AdminAuth,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Response {
    match state.api_keys.delete(id) {
        Ok(()) => {
            tracing::info!(id, "已删除 API Key");
            DataEnvelope::new(serde_json::json!({ "ok": true })).into_response()
        }
        Err(ApiKeyStoreError::NotFound) => not_found_api("API Key 不存在"),
        Err(ApiKeyStoreError::InvalidName) => bad_request("名称不能为空"),
    }
}
