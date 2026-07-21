//! 客户端 OpenAI 兼容 `/v1/models`：列出已配置分组名。

use axum::extract::State;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;

use crate::auth::ClientAuth;
use crate::http::AppState;
use crate::response::bad_request;

#[derive(Debug, Serialize)]
pub struct ModelsListResponse {
    pub object: &'static str,
    pub data: Vec<ModelObject>,
}

#[derive(Debug, Serialize)]
pub struct ModelObject {
    pub id: String,
    pub object: &'static str,
    pub owned_by: &'static str,
}

/// 鉴权通过后返回分组列表；不使用管理信封。
pub async fn models_handler(_auth: ClientAuth, State(state): State<AppState>) -> Response {
    match state.groups.list() {
        Ok(groups) => {
            let data = groups
                .into_iter()
                .map(|g| ModelObject {
                    id: g.name,
                    object: "model",
                    owned_by: "model-hub",
                })
                .collect();
            Json(ModelsListResponse {
                object: "list",
                data,
            })
            .into_response()
        }
        Err(err) => {
            tracing::error!(error = %err, "列出分组失败");
            bad_request("列出模型失败")
        }
    }
}
