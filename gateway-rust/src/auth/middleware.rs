//! 请求提取器（本地桌面场景：**不校验**管理 JWT / 客户端 API Key）。

use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum::response::Response;

use crate::http::AppState;

use super::jwt::AdminClaims;

/// 兼容旧路由签名的管理调用者占位（恒为本地信任）。
#[derive(Debug, Clone)]
pub struct AdminAuth {
    pub claims: AdminClaims,
}

/// 兼容旧路由签名的客户端调用者占位（恒为本地信任）。
#[derive(Debug, Clone)]
pub struct ClientAuth {
    pub key_id: i64,
    pub name: String,
}

fn local_admin() -> AdminAuth {
    AdminAuth {
        claims: AdminClaims {
            sub: "local".into(),
            exp: i64::MAX / 2,
            iat: 0,
        },
    }
}

fn local_client() -> ClientAuth {
    ClientAuth {
        key_id: 0,
        name: "local".into(),
    }
}

impl FromRequestParts<AppState> for AdminAuth {
    type Rejection = Response;

    async fn from_request_parts(
        _parts: &mut Parts,
        _state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        Ok(local_admin())
    }
}

impl FromRequestParts<AppState> for ClientAuth {
    type Rejection = Response;

    async fn from_request_parts(
        _parts: &mut Parts,
        _state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        Ok(local_client())
    }
}

/// 保留符号供旧调用；恒成功。
#[allow(clippy::result_large_err)]
pub fn require_admin_jwt(
    _auth: &super::AuthService,
    _parts: &Parts,
) -> Result<AdminAuth, Response> {
    Ok(local_admin())
}

/// 保留符号供旧调用；恒成功。
#[allow(clippy::result_large_err)]
pub fn require_client_api_key(
    _store: &dyn crate::apikey::ApiKeyStore,
    _parts: &Parts,
) -> Result<ClientAuth, Response> {
    Ok(local_client())
}
