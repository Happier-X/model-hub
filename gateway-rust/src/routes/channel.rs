//! 渠道管理 API 路由。

use axum::extract::{Path, State};
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::auth::AdminAuth;
use crate::channel::{
    ChannelError, CreateChannelRequest, EnableChannelRequest, UpdateChannelRequest,
};
use crate::http::AppState;
use crate::response::{bad_request, not_found_api, DataEnvelope};
use crate::upstream::UpstreamError;

fn map_channel_err(err: ChannelError) -> Response {
    match err {
        ChannelError::NotFound => not_found_api("渠道不存在"),
        ChannelError::InvalidName => bad_request("渠道名称不能为空"),
        ChannelError::InvalidInput(msg) => bad_request(msg),
        ChannelError::Internal => bad_request("内部存储错误"),
    }
}

pub async fn list_channel_handler(_auth: AdminAuth, State(state): State<AppState>) -> Response {
    match state.channels.list() {
        Ok(list) => DataEnvelope::new(list).into_response(),
        Err(err) => map_channel_err(err),
    }
}

#[derive(Debug, Deserialize)]
pub struct ProbeModelsRequest {
    pub base_url: String,
    #[serde(default)]
    pub api_key: String,
}

#[derive(Debug, Serialize)]
pub struct ProbeModelsData {
    pub models: Vec<String>,
}

/// 代理探测上游 OpenAI 兼容 `GET {base_url}/models`，供创建渠道时选模型。
pub async fn probe_models_handler(
    _auth: AdminAuth,
    State(state): State<AppState>,
    Json(body): Json<ProbeModelsRequest>,
) -> Response {
    let base_url = body.base_url.trim();
    if base_url.is_empty() {
        return bad_request("Base URL 不能为空");
    }
    if !(base_url.starts_with("http://") || base_url.starts_with("https://")) {
        return bad_request("Base URL 须以 http:// 或 https:// 开头");
    }

    match state.upstream.fetch_models(base_url, &body.api_key).await {
        Ok(models) => DataEnvelope::new(ProbeModelsData { models }).into_response(),
        Err(UpstreamError::Network(message)) => bad_request(message),
    }
}

pub async fn create_channel_handler(
    _auth: AdminAuth,
    State(state): State<AppState>,
    Json(body): Json<CreateChannelRequest>,
) -> Response {
    match state.channels.create(body) {
        Ok(created) => {
            tracing::info!(id = created.id, name = %created.name, "已创建渠道");
            DataEnvelope::new(created).into_response()
        }
        Err(err) => map_channel_err(err),
    }
}

pub async fn update_channel_handler(
    _auth: AdminAuth,
    State(state): State<AppState>,
    Json(body): Json<UpdateChannelRequest>,
) -> Response {
    match state.channels.update(body) {
        Ok(updated) => {
            tracing::info!(id = updated.id, "已更新渠道");
            DataEnvelope::new(updated).into_response()
        }
        Err(err) => map_channel_err(err),
    }
}

pub async fn enable_channel_handler(
    _auth: AdminAuth,
    State(state): State<AppState>,
    Json(body): Json<EnableChannelRequest>,
) -> Response {
    match state.channels.enable(body) {
        Ok(updated) => {
            tracing::info!(
                id = updated.id,
                enabled = updated.enabled,
                "已切换渠道启用状态"
            );
            DataEnvelope::new(updated).into_response()
        }
        Err(err) => map_channel_err(err),
    }
}

pub async fn delete_channel_handler(
    _auth: AdminAuth,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Response {
    match state.channels.delete(id) {
        Ok(()) => {
            tracing::info!(id, "已删除渠道");
            DataEnvelope::new(serde_json::json!({ "ok": true })).into_response()
        }
        Err(err) => map_channel_err(err),
    }
}
