//! 上游转发与故障转移（非流式读完 body；流式 prime 首包后透传）。

use std::time::{Duration, Instant};

use axum::body::Body;
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use bytes::Bytes;
use reqwest::Client;
use serde_json::Value;

use crate::domain::log::NewRequestLog;
use crate::domain::provider::Provider;
use crate::domain::Stores;
use crate::proxy::circuit::CircuitRegistry;

/// 流式首包超时。
pub const STREAM_FIRST_BYTE_TIMEOUT: Duration = Duration::from_secs(60);
/// 非流式总超时。
pub const NON_STREAM_TIMEOUT: Duration = Duration::from_secs(120);
/// 连接超时。
pub const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Clone)]
pub struct UpstreamClients {
    non_stream: Client,
    stream: Client,
}

impl UpstreamClients {
    pub fn new() -> Self {
        let non_stream = Client::builder()
            .timeout(NON_STREAM_TIMEOUT)
            .connect_timeout(CONNECT_TIMEOUT)
            .build()
            .expect("http client");
        let stream = Client::builder()
            .connect_timeout(CONNECT_TIMEOUT)
            .build()
            .expect("stream http client");
        Self { non_stream, stream }
    }
}

impl Default for UpstreamClients {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct Candidate {
    pub provider: Provider,
    pub upstream_model: String,
}

#[derive(Debug)]
pub enum AttemptError {
    /// 可换源重试
    Retryable { status: Option<u16>, message: String },
    /// 不可换源（明确客户端错误）
    NonRetryable {
        status: u16,
        body: Bytes,
        headers: HeaderMap,
    },
}

fn is_retryable_status(status: u16) -> bool {
    matches!(status, 401 | 403 | 408 | 409 | 425 | 429) || (500..600).contains(&status)
}

fn classify_status_error(status: u16, body: Bytes, headers: HeaderMap) -> AttemptError {
    if is_retryable_status(status) {
        let msg = String::from_utf8_lossy(&body)
            .chars()
            .take(200)
            .collect::<String>();
        AttemptError::Retryable {
            status: Some(status),
            message: format!("上游 HTTP {status}: {msg}"),
        }
    } else {
        AttemptError::NonRetryable {
            status,
            body,
            headers,
        }
    }
}

fn chat_url(base_url: &str) -> String {
    let base = base_url.trim_end_matches('/');
    if base.ends_with("/v1") {
        format!("{base}/chat/completions")
    } else {
        format!("{base}/v1/chat/completions")
    }
}

fn rewrite_model(body: &Value, upstream_model: &str) -> Value {
    let mut v = body.clone();
    if let Some(obj) = v.as_object_mut() {
        obj.insert("model".into(), Value::String(upstream_model.to_string()));
    }
    v
}

fn map_headers(resp: &reqwest::Response) -> HeaderMap {
    let mut headers = HeaderMap::new();
    for (k, v) in resp.headers().iter() {
        if k.as_str().eq_ignore_ascii_case("transfer-encoding")
            || k.as_str().eq_ignore_ascii_case("content-length")
            || k.as_str().eq_ignore_ascii_case("connection")
        {
            continue;
        }
        if let Ok(name) = axum::http::HeaderName::from_bytes(k.as_str().as_bytes()) {
            if let Ok(val) = HeaderValue::from_bytes(v.as_bytes()) {
                headers.insert(name, val);
            }
        }
    }
    headers
}

async fn attempt_non_stream(
    clients: &UpstreamClients,
    candidate: &Candidate,
    body: &Value,
) -> Result<(StatusCode, HeaderMap, Bytes), AttemptError> {
    let url = chat_url(&candidate.provider.base_url);
    let payload = rewrite_model(body, &candidate.upstream_model);
    let response = clients
        .non_stream
        .post(&url)
        .header(
            "Authorization",
            format!("Bearer {}", candidate.provider.api_key),
        )
        .header("Content-Type", "application/json")
        .json(&payload)
        .send()
        .await
        .map_err(|e| AttemptError::Retryable {
            status: None,
            message: format!("上游网络错误: {e}"),
        })?;

    let status = response.status().as_u16();
    let headers = map_headers(&response);
    let bytes = response
        .bytes()
        .await
        .map_err(|e| AttemptError::Retryable {
            status: Some(status),
            message: format!("读取上游响应失败: {e}"),
        })?;

    if !(200..300).contains(&status) {
        return Err(classify_status_error(status, bytes, headers));
    }
    Ok((
        StatusCode::from_u16(status).unwrap_or(StatusCode::OK),
        headers,
        bytes,
    ))
}

struct StreamPrimeOk {
    status: StatusCode,
    headers: HeaderMap,
    first_chunk: Bytes,
    rest: reqwest::Response,
}

async fn attempt_stream_prime(
    clients: &UpstreamClients,
    candidate: &Candidate,
    body: &Value,
) -> Result<StreamPrimeOk, AttemptError> {
    let url = chat_url(&candidate.provider.base_url);
    let payload = rewrite_model(body, &candidate.upstream_model);
    let response = clients
        .stream
        .post(&url)
        .header(
            "Authorization",
            format!("Bearer {}", candidate.provider.api_key),
        )
        .header("Content-Type", "application/json")
        .json(&payload)
        .send()
        .await
        .map_err(|e| AttemptError::Retryable {
            status: None,
            message: format!("上游网络错误: {e}"),
        })?;

    let status = response.status().as_u16();
    let headers = map_headers(&response);

    if !(200..300).contains(&status) {
        let bytes = response.bytes().await.unwrap_or_default();
        return Err(classify_status_error(status, bytes, headers));
    }

    let mut response = response;
    let first = tokio::time::timeout(STREAM_FIRST_BYTE_TIMEOUT, response.chunk())
        .await
        .map_err(|_| AttemptError::Retryable {
            status: Some(status),
            message: "流式首包超时".into(),
        })?
        .map_err(|e| AttemptError::Retryable {
            status: Some(status),
            message: format!("读取流式首包失败: {e}"),
        })?;

    let first_chunk = first.unwrap_or_else(Bytes::new);

    Ok(StreamPrimeOk {
        status: StatusCode::from_u16(status).unwrap_or(StatusCode::OK),
        headers,
        first_chunk,
        rest: response,
    })
}

struct StreamState {
    first: Option<Bytes>,
    response: Option<reqwest::Response>,
    done: bool,
}

fn stream_body_from_prime(first: Bytes, response: reqwest::Response) -> Body {
    let stream = futures_util::stream::unfold(
        StreamState {
            first: Some(first),
            response: Some(response),
            done: false,
        },
        |mut state| async move {
            if state.done {
                return None;
            }
            if let Some(chunk) = state.first.take() {
                if !chunk.is_empty() {
                    return Some((Ok::<Bytes, std::io::Error>(chunk), state));
                }
            }
            let Some(resp) = state.response.as_mut() else {
                return None;
            };
            match resp.chunk().await {
                Ok(Some(bytes)) => Some((Ok(bytes), state)),
                Ok(None) => None,
                Err(e) => Some((Err(std::io::Error::other(e.to_string())), state)),
            }
        },
    );
    Body::from_stream(stream)
}

pub struct ForwardOutcome {
    pub response: Response,
    pub final_provider_name: String,
    pub final_model: String,
    pub failover_from: String,
    pub failover_to: String,
    pub failover_reason: String,
    /// 最终响应若为上游错误（如不可重试 4xx），填入摘要供请求日志使用。
    pub error: String,
}

pub async fn forward_with_failover(
    stores: &Stores,
    circuits: &CircuitRegistry,
    clients: &UpstreamClients,
    group_name: &str,
    auto_failover: bool,
    candidates: &[Candidate],
    body: &Value,
    stream: bool,
) -> Result<ForwardOutcome, (StatusCode, String)> {
    if candidates.is_empty() {
        return Err((StatusCode::BAD_GATEWAY, "分组无可用上游".into()));
    }

    let mut last_error = "无可用上游".to_string();
    let mut failover_from = String::new();
    let mut failover_to = String::new();
    let mut failover_reason = String::new();
    let mut previous_name: Option<String> = None;

    let try_list: Vec<&Candidate> = if auto_failover {
        candidates.iter().collect()
    } else {
        candidates.iter().take(1).collect()
    };

    for candidate in try_list {
        if !candidate.provider.enabled {
            continue;
        }
        if !circuits.allow_request(candidate.provider.id) {
            last_error = format!("供应商 {} 熔断中", candidate.provider.name);
            continue;
        }

        if let Some(prev) = &previous_name {
            failover_from = prev.clone();
            failover_to = candidate.provider.name.clone();
        }

        let attempt_err: AttemptError = if stream {
            match attempt_stream_prime(clients, candidate, body).await {
                Ok(ok) => {
                    circuits.record_success(candidate.provider.id);
                    let body = stream_body_from_prime(ok.first_chunk, ok.rest);
                    let mut builder = Response::builder().status(ok.status);
                    for (k, v) in ok.headers.iter() {
                        builder = builder.header(k, v);
                    }
                    let response = builder.body(body).unwrap_or_else(|_| {
                        (StatusCode::INTERNAL_SERVER_ERROR, "构建流式响应失败").into_response()
                    });
                    return Ok(ForwardOutcome {
                        response,
                        final_provider_name: candidate.provider.name.clone(),
                        final_model: candidate.upstream_model.clone(),
                        failover_from,
                        failover_to,
                        failover_reason,
                        error: String::new(),
                    });
                }
                Err(e) => e,
            }
        } else {
            match attempt_non_stream(clients, candidate, body).await {
                Ok((status, headers, bytes)) => {
                    circuits.record_success(candidate.provider.id);
                    let mut builder = Response::builder().status(status);
                    for (k, v) in headers.iter() {
                        builder = builder.header(k, v);
                    }
                    let response = builder
                        .body(Body::from(bytes))
                        .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response());
                    return Ok(ForwardOutcome {
                        response,
                        final_provider_name: candidate.provider.name.clone(),
                        final_model: candidate.upstream_model.clone(),
                        failover_from,
                        failover_to,
                        failover_reason,
                        error: String::new(),
                    });
                }
                Err(e) => e,
            }
        };

        match attempt_err {
            AttemptError::NonRetryable {
                status,
                body,
                headers,
            } => {
                // 明确客户端/不可重试错误不推进熔断（避免请求体错误误伤供应商健康）。
                let msg = String::from_utf8_lossy(&body)
                    .chars()
                    .take(200)
                    .collect::<String>();
                // 最终日志由 server 统一写入，避免双写。
                let mut builder = Response::builder()
                    .status(StatusCode::from_u16(status).unwrap_or(StatusCode::BAD_REQUEST));
                for (k, v) in headers.iter() {
                    builder = builder.header(k, v);
                }
                let response = builder
                    .body(Body::from(body))
                    .unwrap_or_else(|_| StatusCode::BAD_REQUEST.into_response());
                return Ok(ForwardOutcome {
                    response,
                    final_provider_name: candidate.provider.name.clone(),
                    final_model: candidate.upstream_model.clone(),
                    failover_from,
                    failover_to,
                    failover_reason: "不可重试错误".into(),
                    error: msg,
                });
            }
            AttemptError::Retryable { status, message } => {
                circuits.record_failure(candidate.provider.id);
                last_error = message.clone();
                if previous_name.is_some() || auto_failover {
                    failover_reason = message.clone();
                }
                let _ = stores.insert_log(NewRequestLog {
                    group_name: group_name.into(),
                    provider_name: candidate.provider.name.clone(),
                    upstream_model: candidate.upstream_model.clone(),
                    status_code: status.unwrap_or(0) as i64,
                    use_time_ms: 0,
                    error: message,
                    failover_from: String::new(),
                    failover_to: String::new(),
                    failover_reason: String::new(),
                });
                previous_name = Some(candidate.provider.name.clone());
                if !auto_failover {
                    break;
                }
            }
        }
    }

    Err((StatusCode::BAD_GATEWAY, last_error))
}

pub fn elapsed_ms(start: Instant) -> i64 {
    start.elapsed().as_millis() as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retryable_status_matrix() {
        assert!(is_retryable_status(500));
        assert!(is_retryable_status(401));
        assert!(!is_retryable_status(400));
        assert!(!is_retryable_status(404));
    }

    #[test]
    fn rewrite_model_replaces_field() {
        let body = serde_json::json!({"model":"group","messages":[]});
        let out = rewrite_model(&body, "gpt-4o");
        assert_eq!(out["model"], "gpt-4o");
    }
}
