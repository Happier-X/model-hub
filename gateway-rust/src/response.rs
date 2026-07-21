//! 管理 API 成功信封与统一 401 错误体。

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;

/// 成功响应：`{ "data": T }`，兼容前端 `gatewayHttp` 解包。
#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct DataEnvelope<T> {
    pub data: T,
}

impl<T: Serialize> DataEnvelope<T> {
    pub fn new(data: T) -> Self {
        Self { data }
    }
}

impl<T: Serialize> IntoResponse for DataEnvelope<T> {
    fn into_response(self) -> Response {
        (StatusCode::OK, Json(self)).into_response()
    }
}

/// 鉴权失败等业务错误：顶层 `message` + `error.{code,message}`。
#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct AuthErrorBody {
    pub message: String,
    pub error: AuthErrorDetail,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct AuthErrorDetail {
    pub code: &'static str,
    pub message: String,
}

pub fn unauthorized(message: impl Into<String>) -> Response {
    let message = message.into();
    (
        StatusCode::UNAUTHORIZED,
        Json(AuthErrorBody {
            message: message.clone(),
            error: AuthErrorDetail {
                code: "UNAUTHORIZED",
                message,
            },
        }),
    )
        .into_response()
}

pub fn bad_request(message: impl Into<String>) -> Response {
    let message = message.into();
    (
        StatusCode::BAD_REQUEST,
        Json(AuthErrorBody {
            message: message.clone(),
            error: AuthErrorDetail {
                code: "BAD_REQUEST",
                message,
            },
        }),
    )
        .into_response()
}

pub fn not_found_api(message: impl Into<String>) -> Response {
    let message = message.into();
    (
        StatusCode::NOT_FOUND,
        Json(AuthErrorBody {
            message: message.clone(),
            error: AuthErrorDetail {
                code: "NOT_FOUND",
                message,
            },
        }),
    )
        .into_response()
}
