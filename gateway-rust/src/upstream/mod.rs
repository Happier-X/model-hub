//! 上游 HTTP 客户端（非流式 chat 转发）。

use std::time::Duration;

use axum::http::{HeaderMap, HeaderName, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use reqwest::Client;
use serde_json::Value;

use crate::response::AuthErrorBody;
use crate::router::{build_chat_url, RouteTarget};

/// 默认上游超时（秒）。
pub const DEFAULT_UPSTREAM_TIMEOUT_SECS: u64 = 60;

/// 封装 reqwest 客户端。
#[derive(Clone)]
pub struct UpstreamClient {
    client: Client,
}

impl UpstreamClient {
    pub fn new(timeout_secs: u64) -> Result<Self, String> {
        let timeout = Duration::from_secs(if timeout_secs == 0 {
            DEFAULT_UPSTREAM_TIMEOUT_SECS
        } else {
            timeout_secs
        });
        let client = Client::builder()
            .timeout(timeout)
            .build()
            .map_err(|err| format!("创建 HTTP 客户端失败: {err}"))?;
        Ok(Self { client })
    }

    pub fn with_default_timeout() -> Self {
        Self::new(DEFAULT_UPSTREAM_TIMEOUT_SECS).expect("default reqwest client")
    }

    /// 非流式转发 chat completions。
    pub async fn forward_chat(
        &self,
        target: &RouteTarget,
        body: &Value,
    ) -> Result<Response, UpstreamError> {
        let url = build_chat_url(&target.base_url);
        tracing::info!(
            group_id = target.group_id,
            group_name = %target.group_name,
            channel_id = target.channel_id,
            upstream_model = %target.upstream_model,
            %url,
            "转发 chat 到上游"
        );

        let mut request = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", target.channel_key))
            .header("Content-Type", "application/json")
            .json(body);

        request = apply_custom_headers(request, &target.custom_header);

        let response = request.send().await.map_err(|err| {
            tracing::warn!(
                group_id = target.group_id,
                channel_id = target.channel_id,
                error = %err,
                "上游网络请求失败"
            );
            UpstreamError::Network(format!("上游请求失败: {err}"))
        })?;

        let status =
            StatusCode::from_u16(response.status().as_u16()).unwrap_or(StatusCode::BAD_GATEWAY);
        let content_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());
        let bytes = response
            .bytes()
            .await
            .map_err(|err| UpstreamError::Network(format!("读取上游响应失败: {err}")))?;

        let mut headers = HeaderMap::new();
        if let Some(ct) = content_type {
            if let Ok(value) = HeaderValue::from_str(&ct) {
                headers.insert(axum::http::header::CONTENT_TYPE, value);
            }
        }

        Ok((status, headers, bytes).into_response())
    }
}

#[derive(Debug)]
pub enum UpstreamError {
    Network(String),
}

impl UpstreamError {
    pub fn into_response(self) -> Response {
        match self {
            Self::Network(message) => {
                let body = AuthErrorBody {
                    message: message.clone(),
                    error: crate::response::AuthErrorDetail {
                        code: "BAD_GATEWAY",
                        message,
                    },
                };
                (StatusCode::BAD_GATEWAY, Json(body)).into_response()
            }
        }
    }
}

/// 将 `custom_header` 尽力应用到请求；支持对象数组 `[{name,value}]` 或 map。
fn apply_custom_headers(
    mut request: reqwest::RequestBuilder,
    custom_header: &Value,
) -> reqwest::RequestBuilder {
    match custom_header {
        Value::Array(items) => {
            for item in items {
                if let Some((name, value)) = header_pair_from_value(item) {
                    request = request.header(name, value);
                }
            }
        }
        Value::Object(map) => {
            for (name, value) in map {
                if let Some(v) = value.as_str() {
                    request = request.header(name.as_str(), v);
                }
            }
        }
        _ => {}
    }
    request
}

fn header_pair_from_value(item: &Value) -> Option<(String, String)> {
    let obj = item.as_object()?;
    let name = obj
        .get("name")
        .or_else(|| obj.get("key"))
        .and_then(|v| v.as_str())?
        .trim();
    let value = obj.get("value").and_then(|v| v.as_str())?.trim();
    if name.is_empty() || value.is_empty() {
        return None;
    }
    // 校验 header 名合法性，失败则忽略单条
    HeaderName::from_bytes(name.as_bytes()).ok()?;
    HeaderValue::from_str(value).ok()?;
    Some((name.to_string(), value.to_string()))
}

/// 改写发往上游的 body：`model` → 上游模型名；去掉 stream 字段（调用方已拒绝 stream=true）。
pub fn rewrite_upstream_body(mut body: Value, upstream_model: &str) -> Value {
    if let Some(obj) = body.as_object_mut() {
        obj.insert("model".into(), Value::String(upstream_model.to_string()));
        obj.remove("stream");
    }
    body
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn rewrite_replaces_model_and_strips_stream() {
        let body = json!({
            "model": "group-name",
            "stream": false,
            "messages": [{"role": "user", "content": "hi"}]
        });
        let out = rewrite_upstream_body(body, "gpt-4o-mini");
        assert_eq!(out["model"], "gpt-4o-mini");
        assert!(out.get("stream").is_none());
        assert_eq!(out["messages"][0]["content"], "hi");
    }
}
