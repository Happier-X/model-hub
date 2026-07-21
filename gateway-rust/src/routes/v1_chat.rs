//! 客户端 OpenAI 兼容 `POST /v1/chat/completions`（非流式 + SSE 流式）。

use std::time::Instant;

use axum::body::Body;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use http_body_util::BodyExt;
use serde_json::Value;

use crate::auth::ClientAuth;
use crate::http::AppState;
use crate::log::{truncate_error, NewRelayLog};
use crate::response::{bad_request, not_found_api};
use crate::router::RouteError;
use crate::upstream::rewrite_upstream_body;

/// 处理 chat 转发（`stream=true` 走 SSE 透明代理，否则整包 JSON）。
pub async fn chat_completions_handler(
    _auth: ClientAuth,
    State(state): State<AppState>,
    Json(body): Json<Value>,
) -> Response {
    let started = Instant::now();

    let Some(obj) = body.as_object() else {
        return bad_request("请求体必须为 JSON 对象");
    };

    let stream = obj.get("stream").and_then(|v| v.as_bool()) == Some(true);

    let group_name = match obj.get("model").and_then(|v| v.as_str()) {
        Some(name) if !name.trim().is_empty() => name.trim().to_string(),
        _ => return bad_request("缺少 model（应填分组名）"),
    };

    if !obj.contains_key("messages") {
        return bad_request("缺少 messages");
    }

    let target = match state.router.resolve(&group_name) {
        Ok(t) => t,
        Err(RouteError::GroupNotFound) => {
            let msg = format!("未知分组: {group_name}");
            record_route_error(&state, &group_name, started, &msg);
            return not_found_api(msg);
        }
        Err(RouteError::EmptyItems) => {
            let msg = "分组未绑定任何渠道模型";
            record_route_error(&state, &group_name, started, msg);
            return bad_request(msg);
        }
        Err(RouteError::NoAvailableChannel) => {
            let msg = "分组无可用渠道";
            record_route_error(&state, &group_name, started, msg);
            return bad_request(msg);
        }
        Err(RouteError::Internal(msg)) => {
            tracing::error!(error = %msg, "路由解析内部错误");
            record_route_error(&state, &group_name, started, "路由解析失败");
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

    let use_time = elapsed_secs(started);

    match result {
        Ok(response) => {
            if stream {
                // 流式：tokens 默认 0；成功不缓冲 body；非 2xx（上游已整包缓冲）尽力解析 error
                let (response, error) = if response.status().is_success() {
                    (response, String::new())
                } else {
                    let (response, error) = capture_error_and_rebuild(response).await;
                    (response, error)
                };

                state.logs.insert_best_effort(&NewRelayLog {
                    time: chrono::Utc::now().timestamp(),
                    request_model_name: group_name,
                    channel_name: target.channel_name.clone(),
                    actual_model_name: target.upstream_model.clone(),
                    input_tokens: 0,
                    output_tokens: 0,
                    use_time,
                    cost: 0.0,
                    error,
                });
                response
            } else {
                let (response, input_tokens, output_tokens, error) =
                    finalize_non_stream_log(response).await;
                state.logs.insert_best_effort(&NewRelayLog {
                    time: chrono::Utc::now().timestamp(),
                    request_model_name: group_name,
                    channel_name: target.channel_name.clone(),
                    actual_model_name: target.upstream_model.clone(),
                    input_tokens,
                    output_tokens,
                    use_time,
                    cost: 0.0,
                    error,
                });
                response
            }
        }
        Err(err) => {
            let message = match &err {
                crate::upstream::UpstreamError::Network(msg) => msg.clone(),
            };
            state.logs.insert_best_effort(&NewRelayLog {
                time: chrono::Utc::now().timestamp(),
                request_model_name: group_name,
                channel_name: target.channel_name.clone(),
                actual_model_name: target.upstream_model.clone(),
                input_tokens: 0,
                output_tokens: 0,
                use_time,
                cost: 0.0,
                error: truncate_error(&message),
            });
            err.into_response()
        }
    }
}

fn record_route_error(state: &AppState, group_name: &str, started: Instant, error: &str) {
    state.logs.insert_best_effort(&NewRelayLog {
        time: chrono::Utc::now().timestamp(),
        request_model_name: group_name.to_string(),
        channel_name: String::new(),
        actual_model_name: String::new(),
        input_tokens: 0,
        output_tokens: 0,
        use_time: elapsed_secs(started),
        cost: 0.0,
        error: truncate_error(error),
    });
}

fn elapsed_secs(started: Instant) -> i64 {
    started.elapsed().as_secs() as i64
}

/// 非流式：缓冲 body 解析 usage / error，再重建透传响应。
async fn finalize_non_stream_log(response: Response) -> (Response, i64, i64, String) {
    let status = response.status();
    let headers = response.headers().clone();
    let bytes = match response.into_body().collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(err) => {
            tracing::warn!(error = %err, "读取上游响应 body 失败");
            let msg = truncate_error(&format!("读取上游响应失败: {err}"));
            return (
                (
                    status,
                    headers,
                    Body::from(format!(r#"{{"message":"{msg}"}}"#)),
                )
                    .into_response(),
                0,
                0,
                msg,
            );
        }
    };

    let (input_tokens, output_tokens, error) = parse_usage_and_error(status, &bytes);
    let mut builder = Response::builder().status(status);
    if let Some(hdrs) = builder.headers_mut() {
        *hdrs = headers;
    }
    let rebuilt = builder
        .body(Body::from(bytes))
        .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response());
    (rebuilt, input_tokens, output_tokens, error)
}

fn parse_usage_and_error(status: StatusCode, bytes: &[u8]) -> (i64, i64, String) {
    let Ok(json) = serde_json::from_slice::<Value>(bytes) else {
        let error = if status.is_success() {
            String::new()
        } else {
            truncate_error(&String::from_utf8_lossy(bytes))
        };
        return (0, 0, error);
    };

    let usage = json.get("usage");
    let input_tokens = usage
        .and_then(|u| u.get("prompt_tokens"))
        .and_then(|v| v.as_i64())
        .or_else(|| {
            usage
                .and_then(|u| u.get("input_tokens"))
                .and_then(|v| v.as_i64())
        })
        .unwrap_or(0);
    let output_tokens = usage
        .and_then(|u| u.get("completion_tokens"))
        .and_then(|v| v.as_i64())
        .or_else(|| {
            usage
                .and_then(|u| u.get("output_tokens"))
                .and_then(|v| v.as_i64())
        })
        .unwrap_or(0);

    let error = if status.is_success() {
        String::new()
    } else {
        extract_error_message(&json)
    };

    (input_tokens, output_tokens, error)
}

fn extract_error_message(json: &Value) -> String {
    if let Some(msg) = json.get("message").and_then(|v| v.as_str()) {
        return truncate_error(msg);
    }
    if let Some(err) = json.get("error") {
        if let Some(msg) = err.get("message").and_then(|v| v.as_str()) {
            return truncate_error(msg);
        }
        if let Some(msg) = err.as_str() {
            return truncate_error(msg);
        }
    }
    truncate_error(&json.to_string())
}

/// 读取错误 body 摘要后按原 status/headers 重建响应（透传）。
async fn capture_error_and_rebuild(response: Response) -> (Response, String) {
    let status = response.status();
    let headers = response.headers().clone();
    let bytes = match response.into_body().collect().await {
        Ok(c) => c.to_bytes(),
        Err(_) => {
            let error = "上游返回错误".to_string();
            return (
                (status, headers, Body::from(error.clone())).into_response(),
                error,
            );
        }
    };
    let error = if let Ok(json) = serde_json::from_slice::<Value>(&bytes) {
        extract_error_message(&json)
    } else {
        truncate_error(&String::from_utf8_lossy(&bytes))
    };
    let mut builder = Response::builder().status(status);
    if let Some(hdrs) = builder.headers_mut() {
        *hdrs = headers;
    }
    let rebuilt = builder
        .body(Body::from(bytes))
        .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response());
    (rebuilt, error)
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
    async fn missing_key_still_reaches_business_layer() {
        // 本地开放：无 API Key 也应进入路由（未知分组 → 404，而非 401）
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
        assert_ne!(response.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
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

    #[test]
    fn parse_usage_from_openai_json() {
        let body = br#"{"usage":{"prompt_tokens":11,"completion_tokens":7},"choices":[]}"#;
        let (inp, out, err) = parse_usage_and_error(StatusCode::OK, body);
        assert_eq!(inp, 11);
        assert_eq!(out, 7);
        assert!(err.is_empty());
    }

    #[test]
    fn parse_error_message_truncated() {
        let long = "x".repeat(600);
        let body = format!(r#"{{"message":"{long}"}}"#);
        let (_i, _o, err) = parse_usage_and_error(StatusCode::BAD_GATEWAY, body.as_bytes());
        assert_eq!(err.chars().count(), 512);
    }
}
