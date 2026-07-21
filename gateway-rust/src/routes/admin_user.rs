//! 管理用户登录与状态。

use axum::extract::State;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::auth::AdminAuth;
use crate::http::AppState;
use crate::response::{unauthorized, DataEnvelope};

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
    pub expire: Option<u64>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct LoginData {
    pub token: String,
    pub expire_at: String,
}

pub async fn login_handler(
    State(state): State<AppState>,
    Json(body): Json<LoginRequest>,
) -> Response {
    if !state
        .auth
        .verify_password(body.username.trim(), &body.password)
    {
        return unauthorized("用户名或密码错误");
    }

    match state.auth.issue_token(body.username.trim(), body.expire) {
        Ok((token, exp)) => {
            // 不记录完整 token
            tracing::info!(user = %body.username.trim(), "管理用户登录成功");
            DataEnvelope::new(LoginData {
                token,
                expire_at: exp.to_string(),
            })
            .into_response()
        }
        Err(_) => unauthorized("签发 Token 失败"),
    }
}

pub async fn status_handler(_auth: AdminAuth) -> DataEnvelope<&'static str> {
    DataEnvelope::new("ok")
}
