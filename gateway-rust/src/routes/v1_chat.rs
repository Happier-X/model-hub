//! 客户端 OpenAI 兼容 `POST /v1/chat/completions`（非流式 + SSE 流式）。

use axum::extract::State;
use axum::response::Response;
use axum::Json;
use serde_json::Value;

use crate::auth::ClientAuth;
use crate::http::AppState;
use crate::response::{bad_request, not_found_api};
use crate::router::RouteError;
use crate::upstream::rewrite_upstream_body;

/// 处理 chat 转发（`stream=true` 走 SSE 透明代理，否则整包 JSON）。
pub async fn chat_completions_handler(
    _auth: ClientAuth,
    State(state): State<AppState>,
    Json(body): Json<Value>,
) -> Response {
    let Some(obj) = body.as_object() else {
        return bad_request("请求体必须为 JSON 对象");
    };

    let stream = obj.get("stream").and_then(|v| v.as_bool()) == Some(true);

    let group_name = match obj.get("model").and_then(|v| v.as_str()) {
        Some(name) if !name.trim().is_empty() => name.trim(),
        _ => return bad_request("缺少 model（应填分组名）"),
    };

    if !obj.contains_key("messages") {
        return bad_request("缺少 messages");
    }

    let target = match state.router.resolve(group_name) {
        Ok(t) => t,
        Err(RouteError::GroupNotFound) => {
            return not_found_api(format!("未知分组: {group_name}"));
        }
        Err(RouteError::EmptyItems) => {
            return bad_request("分组未绑定任何渠道模型");
        }
        Err(RouteError::NoAvailableChannel) => {
            return bad_request("分组无可用渠道");
        }
        Err(RouteError::Internal(msg)) => {
            tracing::error!(error = %msg, "路由解析内部错误");
            return bad_request("路由解析失败");
        }
    };

    let upstream_body = rewrite_upstream_body(body, &target.upstream_model, stream);

    let result = if stream {
        state
            .upstream
            .forward_chat_stream(&target, &upstream_body)
            .await
    } else {
        state.upstream.forward_chat(&target, &upstream_body).await
    };

    match result {
        Ok(response) => response,
        Err(err) => err.into_response(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    use crate::apikey::CreateApiKeyRequest;
    use crate::channel::{BaseUrl, ChannelKey, CreateChannelRequest};
    use crate::group::{CreateGroupRequest, GroupItem};
    use crate::http::build_router;

    async fn body_json(response: Response) -> Value {
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        serde_json::from_slice(&bytes).unwrap()
    }

    fn seed_client_key(state: &AppState) -> String {
        state
            .api_keys
            .create(CreateApiKeyRequest {
                name: "chat-test".into(),
                enabled: true,
                expire_at: None,
                max_cost: None,
                supported_models: None,
            })
            .unwrap()
            .api_key
    }

    #[tokio::test]
    async fn stream_true_unknown_group_not_401_and_not_stream_rejected() {
        let state = AppState::for_tests();
        let key = seed_client_key(&state);
        let app = build_router(state);

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/chat/completions")
                    .header("authorization", format!("Bearer {key}"))
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"model":"g","messages":[{"role":"user","content":"hi"}],"stream":true}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        // 流式不再拒绝；未知分组走业务 404，不得 401 / STREAM_NOT_SUPPORTED
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert_ne!(response.status(), StatusCode::UNAUTHORIZED);
        let json = body_json(response).await;
        assert_ne!(
            json["error"]["code"].as_str().unwrap_or(""),
            "STREAM_NOT_SUPPORTED"
        );
        assert!(json["message"].as_str().unwrap().contains("未知分组"));
    }

    #[tokio::test]
    async fn unknown_group_returns_404() {
        let state = AppState::for_tests();
        let key = seed_client_key(&state);
        let app = build_router(state);

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/chat/completions")
                    .header("authorization", format!("Bearer {key}"))
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"model":"no-such-group","messages":[{"role":"user","content":"hi"}]}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        let json = body_json(response).await;
        assert_eq!(json["error"]["code"], "NOT_FOUND");
        assert!(json["message"].as_str().unwrap().contains("未知分组"));
    }

    #[tokio::test]
    async fn missing_key_returns_401() {
        let app = build_router(AppState::for_tests());
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/chat/completions")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"model":"g","messages":[{"role":"user","content":"hi"}]}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn empty_items_group_business_error() {
        let state = AppState::for_tests();
        let key = seed_client_key(&state);
        state
            .groups
            .create(CreateGroupRequest {
                name: "empty-g".into(),
                mode: 1,
                match_regex: String::new(),
                items: vec![],
            })
            .unwrap();
        let app = build_router(state);

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/chat/completions")
                    .header("authorization", format!("Bearer {key}"))
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"model":"empty-g","messages":[{"role":"user","content":"hi"}]}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert_ne!(response.status(), StatusCode::UNAUTHORIZED);
        let json = body_json(response).await;
        assert!(json["message"].as_str().unwrap().contains("未绑定"));
    }

    #[tokio::test]
    async fn models_lists_group_names() {
        let state = AppState::for_tests();
        let key = seed_client_key(&state);
        let ch = state
            .channels
            .create(CreateChannelRequest {
                name: "c".into(),
                channel_type: 0,
                enabled: true,
                base_urls: vec![BaseUrl {
                    url: "https://example.com/v1".into(),
                    delay: 0,
                }],
                keys: vec![ChannelKey {
                    id: None,
                    channel_id: None,
                    enabled: true,
                    channel_key: "sk-x".into(),
                    remark: String::new(),
                }],
                model: "m".into(),
                custom_model: String::new(),
                proxy: false,
                auto_sync: false,
                auto_group: 0,
                custom_header: serde_json::json!([]),
            })
            .unwrap();
        state
            .groups
            .create(CreateGroupRequest {
                name: "my-group".into(),
                mode: 1,
                match_regex: String::new(),
                items: vec![GroupItem {
                    id: None,
                    group_id: None,
                    channel_id: ch.id,
                    model_name: "gpt-4o-mini".into(),
                    priority: 1,
                    weight: 1,
                }],
            })
            .unwrap();

        let app = build_router(state);
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/v1/models")
                    .header("authorization", format!("Bearer {key}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let json = body_json(response).await;
        assert_eq!(json["object"], "list");
        let data = json["data"].as_array().unwrap();
        assert_eq!(data.len(), 1);
        assert_eq!(data[0]["id"], "my-group");
        assert_eq!(data[0]["object"], "model");
        assert_eq!(data[0]["owned_by"], "model-hub");
    }
}
