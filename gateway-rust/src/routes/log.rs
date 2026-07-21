//! 请求日志管理 API。

use axum::extract::{Query, State};
use axum::response::{IntoResponse, Response};
use serde::Deserialize;

use crate::auth::AdminAuth;
use crate::http::AppState;
use crate::response::{bad_request, DataEnvelope};

#[derive(Debug, Deserialize)]
pub struct ListLogQuery {
    #[serde(default = "default_page")]
    pub page: u32,
    #[serde(default = "default_page_size")]
    pub page_size: u32,
}

fn default_page() -> u32 {
    1
}

fn default_page_size() -> u32 {
    20
}

pub async fn list_log_handler(
    _auth: AdminAuth,
    State(state): State<AppState>,
    Query(query): Query<ListLogQuery>,
) -> Response {
    match state.logs.list(query.page, query.page_size) {
        Ok(list) => DataEnvelope::new(list).into_response(),
        Err(_) => bad_request("读取请求日志失败"),
    }
}

pub async fn clear_log_handler(_auth: AdminAuth, State(state): State<AppState>) -> Response {
    match state.logs.clear() {
        Ok(()) => {
            tracing::info!("已清空请求日志");
            DataEnvelope::new(serde_json::Value::Null).into_response()
        }
        Err(_) => bad_request("清空请求日志失败"),
    }
}
