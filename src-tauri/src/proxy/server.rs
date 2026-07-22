//! 进程内 HTTP 代理：`/health` + `/v1/*`。

use std::sync::Arc;
use std::time::Instant;

use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::{json, Value};
use tower_http::cors::CorsLayer;

use crate::domain::log::NewRequestLog;
use crate::domain::Stores;
use crate::proxy::circuit::CircuitRegistry;
use crate::proxy::forward::{
    elapsed_ms, forward_with_failover, Candidate, ForwardPolicy, UpstreamClients,
};

#[derive(Clone)]
pub struct AppState {
    pub stores: Stores,
    pub circuits: CircuitRegistry,
    pub clients: UpstreamClients,
    pub forward_policy: ForwardPolicy,
}

fn extract_client_key(headers: &HeaderMap) -> Option<String> {
    if let Some(auth) = headers.get("authorization").and_then(|v| v.to_str().ok()) {
        let auth = auth.trim();
        if let Some(rest) = auth
            .strip_prefix("Bearer ")
            .or_else(|| auth.strip_prefix("bearer "))
        {
            let key = rest.trim();
            if !key.is_empty() {
                return Some(key.to_string());
            }
        }
    }
    if let Some(key) = headers
        .get("x-api-key")
        .or_else(|| headers.get("api-key"))
        .and_then(|v| v.to_str().ok())
    {
        let key = key.trim();
        if !key.is_empty() {
            return Some(key.to_string());
        }
    }
    None
}

/// 本机默认：可不带客户端 Key。
/// 若请求携带了 Key，则必须有效且启用。
async fn require_key(state: &AppState, headers: &HeaderMap) -> Result<(), Response> {
    let Some(raw) = extract_client_key(headers) else {
        return Ok(());
    };
    match state.stores.validate_raw_key(&raw) {
        Ok(true) => Ok(()),
        Ok(false) => Err((
            StatusCode::UNAUTHORIZED,
            Json(json!({"message":"无效或已停用的 API Key","error":{"code":"UNAUTHORIZED","message":"无效或已停用的 API Key"}})),
        )
            .into_response()),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"message":e.to_string()})),
        )
            .into_response()),
    }
}

async fn health() -> impl IntoResponse {
    Json(json!({
        "status": "ok",
        "service": "model-hub-proxy",
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

async fn list_models(State(state): State<AppState>, headers: HeaderMap) -> Response {
    if let Err(resp) = require_key(&state, &headers).await {
        return resp;
    }
    match state.stores.list_group_names() {
        Ok(names) => {
            let data: Vec<Value> = names
                .into_iter()
                .map(|name| {
                    json!({
                        "id": name,
                        "object": "model",
                        "owned_by": "model-hub",
                    })
                })
                .collect();
            Json(json!({"object":"list","data":data})).into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"message": e.to_string()})),
        )
            .into_response(),
    }
}

async fn chat_completions(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Response {
    if let Err(resp) = require_key(&state, &headers).await {
        return resp;
    }

    let group_name = body
        .get("model")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim()
        .to_string();
    if group_name.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"message":"请求体缺少 model（应为分组名）"})),
        )
            .into_response();
    }

    let stream = body
        .get("stream")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let group = match state.stores.get_group_by_name(&group_name) {
        Ok(Some(g)) => g,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({"message": format!("分组不存在: {group_name}")})),
            )
                .into_response();
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"message": e.to_string()})),
            )
                .into_response();
        }
    };

    let mut candidates = Vec::new();
    for item in &group.items {
        if let Ok(Some(provider)) = state.stores.get_provider(item.provider_id) {
            if provider.enabled {
                candidates.push(Candidate {
                    provider,
                    upstream_model: item.upstream_model.clone(),
                });
            }
        }
    }

    let start = Instant::now();
    match forward_with_failover(
        &state.stores,
        &state.circuits,
        &state.clients,
        &group_name,
        group.auto_failover,
        &candidates,
        &body,
        stream,
        &state.forward_policy,
    )
    .await
    {
        Ok(outcome) => {
            // 流式：最终日志由 body 正常结束 / 静默超时 / 读错误回调写入，避免 prime 成功时误记 200。
            if !outcome.defer_request_log {
                let status = outcome.response.status().as_u16() as i64;
                let _ = state.stores.insert_log(NewRequestLog {
                    group_name: group_name.clone(),
                    provider_name: outcome.final_provider_name.clone(),
                    upstream_model: outcome.final_model.clone(),
                    status_code: status,
                    use_time_ms: elapsed_ms(start),
                    error: outcome.error,
                    failover_from: outcome.failover_from,
                    failover_to: outcome.failover_to,
                    failover_reason: outcome.failover_reason,
                });
            }
            outcome.response
        }
        Err((status, message)) => {
            let _ = state.stores.insert_log(NewRequestLog {
                group_name,
                provider_name: String::new(),
                upstream_model: String::new(),
                status_code: status.as_u16() as i64,
                use_time_ms: elapsed_ms(start),
                error: message.clone(),
                failover_from: String::new(),
                failover_to: String::new(),
                failover_reason: String::new(),
            });
            (status, Json(json!({"message": message, "error": {"message": message}}))).into_response()
        }
    }
}

pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/v1/models", get(list_models))
        .route("/v1/chat/completions", post(chat_completions))
        .layer(CorsLayer::permissive())
        .with_state(state)
}

/// 在已绑定 listener 上服务，直到 shutdown 信号。
pub async fn serve(
    listener: tokio::net::TcpListener,
    state: AppState,
    shutdown_rx: tokio::sync::oneshot::Receiver<()>,
) {
    let app = build_router(state);
    let server = axum::serve(listener, app).with_graceful_shutdown(async move {
        let _ = shutdown_rx.await;
    });
    if let Err(err) = server.await {
        tracing::error!(error = %err, "代理 HTTP 服务异常退出");
    }
}

pub type SharedState = Arc<AppState>;
