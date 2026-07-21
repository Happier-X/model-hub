//! 管理 JWT / 客户端 API Key 提取中间件。

use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum::response::Response;

use crate::apikey::{ApiKeyStore, API_KEY_PREFIX};
use crate::http::AppState;
use crate::response::unauthorized;

use super::jwt::AdminClaims;
use super::AuthService;

/// 已通过校验的管理 JWT。
#[derive(Debug, Clone)]
pub struct AdminAuth {
    pub claims: AdminClaims,
}

/// 已通过校验的客户端 API Key（内部记录 id）。
#[derive(Debug, Clone)]
pub struct ClientAuth {
    pub key_id: i64,
    pub name: String,
}

impl FromRequestParts<AppState> for AdminAuth {
    type Rejection = Response;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        require_admin_jwt(&state.auth, parts)
    }
}

impl FromRequestParts<AppState> for ClientAuth {
    type Rejection = Response;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        require_client_api_key(state.api_keys.as_ref(), parts)
    }
}

#[allow(clippy::result_large_err)]
pub fn require_admin_jwt(auth: &AuthService, parts: &Parts) -> Result<AdminAuth, Response> {
    let token = extract_bearer(parts).ok_or_else(|| unauthorized("缺少管理 Token"))?;
    match auth.verify_token(token) {
        Ok(claims) => Ok(AdminAuth { claims }),
        Err(_) => Err(unauthorized("无效或过期的管理 Token")),
    }
}

#[allow(clippy::result_large_err)]
pub fn require_client_api_key(
    store: &dyn ApiKeyStore,
    parts: &Parts,
) -> Result<ClientAuth, Response> {
    let raw = extract_client_key(parts).ok_or_else(|| unauthorized("缺少客户端 API Key"))?;

    // 客户端路径只接受 sk-octopus- 前缀；管理 JWT 即使合法也拒绝。
    if !raw.starts_with(API_KEY_PREFIX) {
        return Err(unauthorized("无效的客户端 API Key"));
    }

    match store.find_by_raw_key(raw) {
        Some(record) if record.enabled => Ok(ClientAuth {
            key_id: record.id,
            name: record.name,
        }),
        Some(_) => Err(unauthorized("API Key 已禁用")),
        None => Err(unauthorized("无效的客户端 API Key")),
    }
}

fn extract_bearer(parts: &Parts) -> Option<&str> {
    let value = parts.headers.get(axum::http::header::AUTHORIZATION)?;
    let value = value.to_str().ok()?;
    let token = value
        .strip_prefix("Bearer ")
        .or_else(|| value.strip_prefix("bearer "))?;
    let token = token.trim();
    if token.is_empty() {
        None
    } else {
        Some(token)
    }
}

fn extract_client_key(parts: &Parts) -> Option<&str> {
    if let Some(token) = extract_bearer(parts) {
        return Some(token);
    }
    let value = parts.headers.get("x-api-key")?;
    let key = value.to_str().ok()?.trim();
    if key.is_empty() {
        None
    } else {
        Some(key)
    }
}
