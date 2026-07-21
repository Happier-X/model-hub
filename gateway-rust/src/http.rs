use std::sync::Arc;

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use serde::Serialize;

use crate::apikey::{ApiKeyStore, MemoryApiKeyStore, SqliteApiKeyStore};
use crate::auth::AuthService;
use crate::channel::{ChannelService, ChannelStore};
use crate::config::GatewayConfig;
use crate::db::{open_from_config, open_path, DbConn};
use crate::error::GatewayError;
use crate::group::{GroupService, GroupStore};
use crate::routes;

/// 应用共享状态。
#[derive(Clone)]
pub struct AppState {
    pub config: Arc<GatewayConfig>,
    pub auth: Arc<AuthService>,
    pub api_keys: Arc<dyn ApiKeyStore>,
    pub channels: Arc<ChannelService>,
    pub groups: Arc<GroupService>,
}

impl AppState {
    /// 从配置打开 SQLite 并组装状态。
    pub fn from_config(config: GatewayConfig) -> Result<Self, GatewayError> {
        let db = open_from_config(&config.database)?;
        Ok(Self::from_config_and_db(config, db))
    }

    pub fn from_config_and_db(config: GatewayConfig, db: DbConn) -> Self {
        let auth = AuthService::from_config(&config.auth);
        let api_keys: Arc<dyn ApiKeyStore> = Arc::new(SqliteApiKeyStore::new(db.clone()));
        let channels = Arc::new(ChannelService::new(ChannelStore::new(db.clone())));
        let groups = Arc::new(GroupService::new(GroupStore::new(db)));
        Self {
            config: Arc::new(config),
            auth: Arc::new(auth),
            api_keys,
            channels,
            groups,
        }
    }

    /// 测试用：临时文件库 + 固定 JWT secret。
    pub fn for_tests() -> Self {
        let mut config = GatewayConfig::default();
        config.auth.jwt_secret = Some("test-jwt-secret-do-not-use-prod".into());
        let db = open_path(":memory:").expect("open memory db for tests");
        Self::from_config_and_db(config, db)
    }

    /// 测试注入：可指定 DB 路径（如 tempfile）。
    pub fn for_tests_with_db_path(path: &str) -> Self {
        let mut config = GatewayConfig::default();
        config.auth.jwt_secret = Some("test-jwt-secret-do-not-use-prod".into());
        config.database.path = path.to_string();
        let db = open_path(path).expect("open test db");
        Self::from_config_and_db(config, db)
    }

    /// 仅内存 API Key（无 SQLite 渠道）——保留给极简单测；路由仍需 channels/groups。
    #[allow(dead_code)]
    pub fn for_tests_memory_keys_only() -> Self {
        let mut config = GatewayConfig::default();
        config.auth.jwt_secret = Some("test-jwt-secret-do-not-use-prod".into());
        let db = open_path(":memory:").expect("open memory db");
        let auth = AuthService::from_config(&config.auth);
        Self {
            config: Arc::new(config),
            auth: Arc::new(auth),
            api_keys: Arc::new(MemoryApiKeyStore::new()),
            channels: Arc::new(ChannelService::new(ChannelStore::new(db.clone()))),
            groups: Arc::new(GroupService::new(GroupStore::new(db))),
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::for_tests()
    }
}

impl std::fmt::Debug for AppState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppState")
            .field("config", &self.config)
            .field("auth", &"<redacted>")
            .field("api_keys", &"<store>")
            .field("channels", &"<service>")
            .field("groups", &"<service>")
            .finish()
    }
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct HealthResponse {
    pub status: &'static str,
    pub service: &'static str,
    pub version: &'static str,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct ErrorBody {
    pub error: ErrorDetail,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct ErrorDetail {
    pub code: &'static str,
    pub message: String,
}

pub fn build_router(state: AppState) -> Router {
    let public = Router::new()
        .route("/health", get(health_handler))
        .route("/api/v1/user/login", post(routes::login_handler));

    let admin = Router::new()
        .route("/api/v1/user/status", get(routes::status_handler))
        .route("/api/v1/apikey/list", get(routes::list_apikey_handler))
        .route("/api/v1/apikey/create", post(routes::create_apikey_handler))
        .route("/api/v1/apikey/update", post(routes::update_apikey_handler))
        .route(
            "/api/v1/apikey/delete/{id}",
            delete(routes::delete_apikey_handler),
        )
        .route("/api/v1/channel/list", get(routes::list_channel_handler))
        .route(
            "/api/v1/channel/create",
            post(routes::create_channel_handler),
        )
        .route(
            "/api/v1/channel/update",
            post(routes::update_channel_handler),
        )
        .route(
            "/api/v1/channel/enable",
            post(routes::enable_channel_handler),
        )
        .route(
            "/api/v1/channel/delete/{id}",
            delete(routes::delete_channel_handler),
        )
        .route("/api/v1/group/list", get(routes::list_group_handler))
        .route("/api/v1/group/create", post(routes::create_group_handler))
        .route("/api/v1/group/update", post(routes::update_group_handler))
        .route(
            "/api/v1/group/delete/{id}",
            delete(routes::delete_group_handler),
        );

    let client = Router::new().route("/v1/models", get(routes::models_handler));

    public
        .merge(admin)
        .merge(client)
        .fallback(not_found_handler)
        .with_state(state)
}

async fn health_handler() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        service: "model-hub-gateway",
        version: env!("CARGO_PKG_VERSION"),
    })
}

async fn not_found_handler() -> Response {
    (
        StatusCode::NOT_FOUND,
        Json(ErrorBody {
            error: ErrorDetail {
                code: "NOT_FOUND",
                message: "未找到请求的接口".to_string(),
            },
        }),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    async fn body_json(response: Response) -> serde_json::Value {
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        serde_json::from_slice(&bytes).unwrap()
    }

    #[tokio::test]
    async fn health_returns_stable_json() {
        let app = build_router(AppState::default());
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let json = body_json(response).await;
        assert_eq!(json["status"], "ok");
        assert_eq!(json["service"], "model-hub-gateway");
        assert_eq!(json["version"], env!("CARGO_PKG_VERSION"));
    }

    #[tokio::test]
    async fn unknown_path_returns_json_404() {
        let app = build_router(AppState::default());
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/does-not-exist")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        let json = body_json(response).await;
        assert_eq!(json["error"]["code"], "NOT_FOUND");
        assert_eq!(json["error"]["message"], "未找到请求的接口");
    }

    #[tokio::test]
    async fn login_and_status_matrix() {
        let app = build_router(AppState::default());

        let bad = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/user/login")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"username":"admin","password":"wrong"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(bad.status(), StatusCode::UNAUTHORIZED);
        let bad_json = body_json(bad).await;
        assert_eq!(bad_json["message"], "用户名或密码错误");
        assert_eq!(bad_json["error"]["code"], "UNAUTHORIZED");

        let ok = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/user/login")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"username":"admin","password":"admin","expire":3600}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(ok.status(), StatusCode::OK);
        let ok_json = body_json(ok).await;
        let token = ok_json["data"]["token"].as_str().unwrap().to_string();
        assert!(!token.is_empty());
        assert!(ok_json["data"]["expire_at"].is_string());

        let no_token = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/user/status")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(no_token.status(), StatusCode::UNAUTHORIZED);

        let with_token = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/user/status")
                    .header("authorization", format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(with_token.status(), StatusCode::OK);
        let status_json = body_json(with_token).await;
        assert_eq!(status_json["data"], "ok");
    }

    #[tokio::test]
    async fn channel_requires_jwt() {
        let app = build_router(AppState::default());
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/channel/list")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }
}
