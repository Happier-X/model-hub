//! 分组管理 API 路由。

use axum::extract::{Path, State};
use axum::response::{IntoResponse, Response};
use axum::Json;

use crate::auth::AdminAuth;
use crate::group::{CreateGroupRequest, GroupError, UpdateGroupRequest};
use crate::http::AppState;
use crate::response::{bad_request, not_found_api, DataEnvelope};

fn map_group_err(err: GroupError) -> Response {
    match err {
        GroupError::NotFound => not_found_api("分组不存在"),
        GroupError::InvalidName => bad_request("分组名称不能为空"),
        GroupError::InvalidInput(msg) => bad_request(msg),
        GroupError::Internal => bad_request("内部存储错误"),
    }
}

pub async fn list_group_handler(_auth: AdminAuth, State(state): State<AppState>) -> Response {
    match state.groups.list() {
        Ok(list) => DataEnvelope::new(list).into_response(),
        Err(err) => map_group_err(err),
    }
}

pub async fn create_group_handler(
    _auth: AdminAuth,
    State(state): State<AppState>,
    Json(body): Json<CreateGroupRequest>,
) -> Response {
    match state.groups.create(body) {
        Ok(created) => {
            tracing::info!(id = created.id, name = %created.name, "已创建分组");
            DataEnvelope::new(created).into_response()
        }
        Err(err) => map_group_err(err),
    }
}

pub async fn update_group_handler(
    _auth: AdminAuth,
    State(state): State<AppState>,
    Json(body): Json<UpdateGroupRequest>,
) -> Response {
    match state.groups.update(body) {
        Ok(updated) => {
            tracing::info!(id = updated.id, "已更新分组");
            DataEnvelope::new(updated).into_response()
        }
        Err(err) => map_group_err(err),
    }
}

pub async fn delete_group_handler(
    _auth: AdminAuth,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Response {
    match state.groups.delete(id) {
        Ok(()) => {
            tracing::info!(id, "已删除分组");
            DataEnvelope::new(serde_json::json!({ "ok": true })).into_response()
        }
        Err(err) => map_group_err(err),
    }
}
