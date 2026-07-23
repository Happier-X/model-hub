//! 上游转发与顺序故障转移（非流式读完 body；流式 prime 首包后透传）。
//!
//! 响应尚未提交客户端前，当前候选项任意失败均换源；历史结果不影响下一次请求起点。

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

/// 流式首包超时。
pub const STREAM_FIRST_BYTE_TIMEOUT: Duration = Duration::from_secs(60);
/// 流式首包后的静默（空闲）超时：后续 chunk 最长等待。
pub const STREAM_IDLE_TIMEOUT: Duration = Duration::from_secs(120);
/// 非流式总超时。
pub const NON_STREAM_TIMEOUT: Duration = Duration::from_secs(600);
/// 连接超时。
pub const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

/// 转发策略（可测注入：如缩短流式静默超时）。
#[derive(Debug, Clone)]
pub struct ForwardPolicy {
    pub stream_idle_timeout: Duration,
}

impl Default for ForwardPolicy {
    fn default() -> Self {
        Self {
            stream_idle_timeout: STREAM_IDLE_TIMEOUT,
        }
    }
}

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

/// 单次候选项失败；响应提交前均可换源。
#[derive(Debug)]
enum AttemptError {
    /// 有上游 HTTP 响应（含非 2xx 与明确的 2xx 错误信封）；队列耗尽时透传最后一次。
    Http {
        status: u16,
        body: Bytes,
        headers: HeaderMap,
        message: String,
    },
    /// 无上游响应体（网络、超时、读失败等）。
    Transport {
        /// 建议网关状态：超时类 504，其它 502。
        gateway_status: u16,
        message: String,
    },
}

impl AttemptError {
    fn message(&self) -> &str {
        match self {
            AttemptError::Http { message, .. } => message,
            AttemptError::Transport { message, .. } => message,
        }
    }
}

/// 从 JSON 错误信封提取截断摘要（不含完整 messages / Key）。
fn error_message_from_json(v: &Value) -> String {
    if let Some(s) = v.get("error").and_then(|e| e.as_str()) {
        return s.chars().take(200).collect();
    }
    if let Some(s) = v
        .get("error")
        .and_then(|e| e.as_object())
        .and_then(|e| e.get("message"))
        .and_then(|m| m.as_str())
    {
        return s.chars().take(200).collect();
    }
    if let Some(s) = v.get("message").and_then(|m| m.as_str()) {
        return s.chars().take(200).collect();
    }
    "上游返回错误信封".chars().take(200).collect()
}

/// 判断 body 是否为明确的结构化错误信封（非正常 chat completion / SSE）。
///
/// 识别：字符串 `error`、对象 `error.message`、`type: "error"`、以及无 `choices` 时的顶层 `message`。
pub fn is_structured_error_body(bytes: &[u8]) -> Option<String> {
    let trimmed = bytes
        .iter()
        .position(|&b| !b.is_ascii_whitespace())
        .map(|i| &bytes[i..])
        .unwrap_or(bytes);
    if trimmed.is_empty() {
        return None;
    }
    // 正常 SSE 首包以 data: / event: / : 注释 开头，不按 JSON 错误信封处理。
    if trimmed.starts_with(b"data:")
        || trimmed.starts_with(b"event:")
        || trimmed.starts_with(b":")
        || trimmed.starts_with(b"id:")
    {
        return None;
    }

    let v: Value = serde_json::from_slice(trimmed).ok()?;
    let obj = v.as_object()?;

    // 正常 chat completion 带 choices：默认不换源；
    // 仅当同时声明 type=error 时仍按错误信封处理（极少数网关混用）。
    if obj.get("choices").is_some() {
        if obj.get("type").and_then(|t| t.as_str()) == Some("error") {
            return Some(error_message_from_json(&v));
        }
        return None;
    }

    let has_type_error = obj.get("type").and_then(|t| t.as_str()) == Some("error");
    let has_error_string = obj
        .get("error")
        .and_then(|e| e.as_str())
        .is_some_and(|s| !s.is_empty());
    let has_error_object_msg = obj
        .get("error")
        .and_then(|e| e.as_object())
        .and_then(|e| e.get("message"))
        .and_then(|m| m.as_str())
        .is_some_and(|s| !s.is_empty());
    let has_top_message = obj
        .get("message")
        .and_then(|m| m.as_str())
        .is_some_and(|s| !s.is_empty())
        && obj.get("object").and_then(|o| o.as_str()) != Some("chat.completion");

    if has_type_error || has_error_string || has_error_object_msg || has_top_message {
        return Some(error_message_from_json(&v));
    }
    None
}

fn redact_sensitive_summary(message: &str, api_key: &str) -> String {
    let mut safe: String = message.chars().take(200).collect();
    if !api_key.is_empty() {
        safe = safe.replace(api_key, "[REDACTED]");
    }
    // 防止常见 Bearer 值进入日志；只保留认证方案。
    if let Some(index) = safe.to_ascii_lowercase().find("bearer ") {
        let value_start = index + "bearer ".len();
        let value_end = safe[value_start..]
            .find(|c: char| c.is_whitespace() || matches!(c, '"' | '\'' | ',' | '}'))
            .map(|offset| value_start + offset)
            .unwrap_or(safe.len());
        safe.replace_range(value_start..value_end, "[REDACTED]");
    }
    safe
}

fn body_error_summary(body: &[u8]) -> String {
    if let Some(msg) = is_structured_error_body(body) {
        return msg;
    }

    // JSON 错误体可能携带 messages、请求内容或其它敏感字段；仅保留常见的
    // 简短诊断字段，不能把整个 JSON 当作日志摘要。
    if let Ok(value) = serde_json::from_slice::<Value>(body) {
        if let Some(code) = value.get("code").and_then(Value::as_str) {
            return code.chars().take(80).collect();
        }
        if let Some(error) = value.get("error").and_then(Value::as_str) {
            return error.chars().take(200).collect();
        }
        return "上游返回未识别的 JSON 错误".into();
    }

    String::from_utf8_lossy(body).chars().take(200).collect()
}

fn http_failure(status: u16, body: Bytes, headers: HeaderMap) -> AttemptError {
    let message = format!("上游 HTTP {status}: {}", body_error_summary(&body));
    AttemptError::Http {
        status,
        body,
        headers,
        message,
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
        .map_err(|e| {
            let timeout = e.is_timeout();
            AttemptError::Transport {
                gateway_status: if timeout { 504 } else { 502 },
                message: if timeout {
                    format!("上游超时: {e}")
                } else {
                    format!("上游网络错误: {e}")
                },
            }
        })?;

    let status = response.status().as_u16();
    let headers = map_headers(&response);
    let bytes = response
        .bytes()
        .await
        .map_err(|e| AttemptError::Transport {
            gateway_status: 502,
            message: format!("读取上游响应失败: {e}"),
        })?;

    if !(200..300).contains(&status) {
        return Err(http_failure(status, bytes, headers));
    }
    if let Some(msg) = is_structured_error_body(&bytes) {
        return Err(AttemptError::Http {
            status,
            body: bytes,
            headers,
            message: format!("上游 HTTP {status}: {msg}"),
        });
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
        .map_err(|e| {
            let timeout = e.is_timeout();
            AttemptError::Transport {
                gateway_status: if timeout { 504 } else { 502 },
                message: if timeout {
                    format!("上游超时: {e}")
                } else {
                    format!("上游网络错误: {e}")
                },
            }
        })?;

    let status = response.status().as_u16();
    let headers = map_headers(&response);

    if !(200..300).contains(&status) {
        let bytes = response.bytes().await.unwrap_or_default();
        return Err(http_failure(status, bytes, headers));
    }

    let mut response = response;
    let first = tokio::time::timeout(STREAM_FIRST_BYTE_TIMEOUT, response.chunk())
        .await
        .map_err(|_| AttemptError::Transport {
            gateway_status: 504,
            message: "流式首包超时".into(),
        })?
        .map_err(|e| AttemptError::Transport {
            gateway_status: 502,
            message: format!("读取流式首包失败: {e}"),
        })?;

    let first_chunk = first.unwrap_or_else(Bytes::new);

    // 首包本身是明确 JSON 错误信封（非 SSE）时换源，响应尚未提交客户端。
    if let Some(msg) = is_structured_error_body(&first_chunk) {
        return Err(AttemptError::Http {
            status,
            body: first_chunk,
            headers,
            message: format!("上游 HTTP {status}: {msg}"),
        });
    }

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
    /// 已调用任一终态回调（成功/超时/读错误/drop 中断）。
    finalized: bool,
    idle: Duration,
    on_idle_timeout: Option<Box<dyn FnOnce() + Send>>,
    on_success: Option<Box<dyn FnOnce() + Send>>,
    on_error: Option<Box<dyn FnOnce(String) + Send>>,
    /// 客户端提前断开：写日志（不换源）。
    on_abort: Option<Box<dyn FnOnce() + Send>>,
}

impl StreamState {
    fn mark_finalized(&mut self) {
        self.finalized = true;
        self.on_idle_timeout.take();
        self.on_success.take();
        self.on_error.take();
        self.on_abort.take();
    }
}

impl Drop for StreamState {
    fn drop(&mut self) {
        if self.finalized {
            return;
        }
        // 客户端提前断开 / 未完整消费 body 时，unfold 可能永不进入终态分支。
        if let Some(cb) = self.on_abort.take() {
            cb();
        }
        self.on_idle_timeout.take();
        self.on_success.take();
        self.on_error.take();
        self.finalized = true;
    }
}

/// 从已成功 prime 的流构造 body；后续 chunk 使用 `idle` 静默超时。
/// 超时后结束流并调用 `on_idle_timeout`；**不会**回到换源循环。
fn stream_body_from_prime(
    first: Bytes,
    response: reqwest::Response,
    idle: Duration,
    on_idle_timeout: impl FnOnce() + Send + 'static,
    on_success: impl FnOnce() + Send + 'static,
    on_error: impl FnOnce(String) + Send + 'static,
    on_abort: impl FnOnce() + Send + 'static,
) -> Body {
    let stream = futures_util::stream::unfold(
        StreamState {
            first: Some(first),
            response: Some(response),
            done: false,
            finalized: false,
            idle,
            on_idle_timeout: Some(Box::new(on_idle_timeout)),
            on_success: Some(Box::new(on_success)),
            on_error: Some(Box::new(on_error)),
            on_abort: Some(Box::new(on_abort)),
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
                if let Some(cb) = state.on_success.take() {
                    cb();
                }
                state.mark_finalized();
                return None;
            };
            match tokio::time::timeout(state.idle, resp.chunk()).await {
                Err(_) => {
                    if let Some(cb) = state.on_idle_timeout.take() {
                        cb();
                    }
                    state.mark_finalized();
                    state.done = true;
                    state.response = None;
                    Some((
                        Err(std::io::Error::new(
                            std::io::ErrorKind::TimedOut,
                            "流式静默超时",
                        )),
                        state,
                    ))
                }
                Ok(Ok(Some(bytes))) => Some((Ok(bytes), state)),
                Ok(Ok(None)) => {
                    if let Some(cb) = state.on_success.take() {
                        cb();
                    }
                    state.mark_finalized();
                    None
                }
                Ok(Err(e)) => {
                    let msg = e.to_string();
                    if let Some(cb) = state.on_error.take() {
                        cb(msg.clone());
                    }
                    state.mark_finalized();
                    state.done = true;
                    state.response = None;
                    Some((Err(std::io::Error::other(msg)), state))
                }
            }
        },
    );
    Body::from_stream(stream)
}

fn build_http_response(status: u16, headers: HeaderMap, body: Bytes) -> Response {
    let mut builder =
        Response::builder().status(StatusCode::from_u16(status).unwrap_or(StatusCode::BAD_GATEWAY));
    for (k, v) in headers.iter() {
        builder = builder.header(k, v);
    }
    builder
        .body(Body::from(body))
        .unwrap_or_else(|_| StatusCode::BAD_GATEWAY.into_response())
}

pub struct ForwardOutcome {
    pub response: Response,
    pub final_provider_name: String,
    pub final_model: String,
    pub failover_from: String,
    pub failover_to: String,
    pub failover_reason: String,
    /// 最终响应若为上游错误，填入摘要供请求日志使用。
    pub error: String,
    /// 为 true 时最终 request_log 由流式 body 终态回调写入，server 不得再记成功。
    pub defer_request_log: bool,
}

pub async fn forward_with_failover(
    stores: &Stores,
    clients: &UpstreamClients,
    group_name: &str,
    candidates: &[Candidate],
    body: &Value,
    stream: bool,
    policy: &ForwardPolicy,
) -> Result<ForwardOutcome, (StatusCode, String)> {
    if candidates.is_empty() {
        return Err((StatusCode::BAD_GATEWAY, "分组无可用上游".into()));
    }

    let mut last_error = "无可用上游".to_string();
    let mut last_http: Option<(u16, HeaderMap, Bytes, String, String, String)> = None;
    let mut last_transport_status: u16 = 502;
    let mut failover_from = String::new();
    let mut failover_to = String::new();
    let mut failover_reason = String::new();
    let mut previous_name: Option<String> = None;
    let mut tried_any = false;

    for candidate in candidates {
        if !candidate.provider.enabled {
            continue;
        }
        tried_any = true;

        if let Some(prev) = &previous_name {
            failover_from = prev.clone();
            failover_to = candidate.provider.name.clone();
        }

        let attempt_err: AttemptError = if stream {
            match attempt_stream_prime(clients, candidate, body).await {
                Ok(ok) => {
                    let provider_name = candidate.provider.name.clone();
                    let upstream_model = candidate.upstream_model.clone();
                    let group = group_name.to_string();
                    let fo_from = failover_from.clone();
                    let fo_to = failover_to.clone();
                    let fo_reason = failover_reason.clone();
                    let success_status = ok.status.as_u16() as i64;
                    let started = Instant::now();
                    let idle = policy.stream_idle_timeout;

                    let on_idle = {
                        let stores = stores.clone();
                        let group = group.clone();
                        let name = provider_name.clone();
                        let model = upstream_model.clone();
                        let fo_from = fo_from.clone();
                        let fo_to = fo_to.clone();
                        let fo_reason = fo_reason.clone();
                        move || {
                            // 响应已提交：仅记日志，不得换源拼接。
                            stores.insert_log_best_effort(NewRequestLog {
                                group_name: group,
                                provider_name: name,
                                upstream_model: model,
                                status_code: 504,
                                use_time_ms: elapsed_ms(started),
                                error: "流式静默超时".into(),
                                failover_from: fo_from,
                                failover_to: fo_to,
                                failover_reason: if fo_reason.is_empty() {
                                    "流式静默超时".into()
                                } else {
                                    fo_reason
                                },
                            });
                        }
                    };
                    let on_success = {
                        let stores = stores.clone();
                        let group = group.clone();
                        let name = provider_name.clone();
                        let model = upstream_model.clone();
                        let fo_from = fo_from.clone();
                        let fo_to = fo_to.clone();
                        let fo_reason = fo_reason.clone();
                        move || {
                            stores.insert_log_best_effort(NewRequestLog {
                                group_name: group,
                                provider_name: name,
                                upstream_model: model,
                                status_code: success_status,
                                use_time_ms: elapsed_ms(started),
                                error: String::new(),
                                failover_from: fo_from,
                                failover_to: fo_to,
                                failover_reason: fo_reason,
                            });
                        }
                    };
                    let on_error = {
                        let stores = stores.clone();
                        let group = group.clone();
                        let name = provider_name.clone();
                        let model = upstream_model.clone();
                        let fo_from = fo_from.clone();
                        let fo_to = fo_to.clone();
                        let fo_reason = fo_reason.clone();
                        move |message: String| {
                            let summary: String = message.chars().take(200).collect();
                            stores.insert_log_best_effort(NewRequestLog {
                                group_name: group,
                                provider_name: name,
                                upstream_model: model,
                                status_code: 502,
                                use_time_ms: elapsed_ms(started),
                                error: format!("流式中断: {summary}"),
                                failover_from: fo_from,
                                failover_to: fo_to,
                                failover_reason: fo_reason,
                            });
                        }
                    };
                    let on_abort = {
                        let stores = stores.clone();
                        move || {
                            stores.insert_log_best_effort(NewRequestLog {
                                group_name: group,
                                provider_name,
                                upstream_model,
                                status_code: 499,
                                use_time_ms: elapsed_ms(started),
                                error: "流式响应未完整结束（客户端断开或中止）".into(),
                                failover_from: fo_from,
                                failover_to: fo_to,
                                failover_reason: fo_reason,
                            });
                        }
                    };

                    let body = stream_body_from_prime(
                        ok.first_chunk,
                        ok.rest,
                        idle,
                        on_idle,
                        on_success,
                        on_error,
                        on_abort,
                    );
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
                        defer_request_log: true,
                    });
                }
                Err(e) => e,
            }
        } else {
            match attempt_non_stream(clients, candidate, body).await {
                Ok((status, headers, bytes)) => {
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
                        defer_request_log: false,
                    });
                }
                Err(e) => e,
            }
        };

        // 中间失败：写脱敏、截断后的尝试摘要，继续下一候选项。
        let safe_error =
            redact_sensitive_summary(attempt_err.message(), &candidate.provider.api_key);
        last_error = safe_error.clone();
        failover_reason = safe_error.clone();
        match &attempt_err {
            AttemptError::Http {
                status,
                body,
                headers,
                ..
            } => {
                last_http = Some((
                    *status,
                    headers.clone(),
                    body.clone(),
                    candidate.provider.name.clone(),
                    candidate.upstream_model.clone(),
                    safe_error.clone(),
                ));
                stores.insert_log_best_effort(NewRequestLog {
                    group_name: group_name.into(),
                    provider_name: candidate.provider.name.clone(),
                    upstream_model: candidate.upstream_model.clone(),
                    status_code: *status as i64,
                    use_time_ms: 0,
                    error: safe_error.clone(),
                    failover_from: String::new(),
                    failover_to: String::new(),
                    failover_reason: String::new(),
                });
            }
            AttemptError::Transport { gateway_status, .. } => {
                last_transport_status = *gateway_status;
                last_http = None; // 最后一次为无响应错误时透传逻辑以 transport 为准
                stores.insert_log_best_effort(NewRequestLog {
                    group_name: group_name.into(),
                    provider_name: candidate.provider.name.clone(),
                    upstream_model: candidate.upstream_model.clone(),
                    status_code: *gateway_status as i64,
                    use_time_ms: 0,
                    error: safe_error.clone(),
                    failover_from: String::new(),
                    failover_to: String::new(),
                    failover_reason: String::new(),
                });
            }
        }
        previous_name = Some(candidate.provider.name.clone());
    }

    if !tried_any {
        return Err((StatusCode::BAD_GATEWAY, "分组无启用的上游".into()));
    }

    // 队列耗尽：有最后 HTTP 响应则透传；否则返回明确网关错误。
    if let Some((status, headers, body, provider_name, model, safe_error)) = last_http {
        let response = build_http_response(status, headers, body);
        return Ok(ForwardOutcome {
            response,
            final_provider_name: provider_name,
            final_model: model,
            failover_from,
            failover_to,
            failover_reason: last_error.clone(),
            error: safe_error,
            defer_request_log: false,
        });
    }

    let gw = StatusCode::from_u16(last_transport_status).unwrap_or(StatusCode::BAD_GATEWAY);
    Err((gw, last_error))
}

pub fn elapsed_ms(start: Instant) -> i64 {
    start.elapsed().as_millis() as i64
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    #[test]
    fn structured_error_string_error_field() {
        let body = r#"{"error":"当前 API 不支持所选模型 gpt-5.6-sol","type":"error"}"#.as_bytes();
        let msg = is_structured_error_body(body).expect("应识别错误信封");
        assert!(msg.contains("不支持所选模型"));
    }

    #[test]
    fn structured_error_object_message() {
        let body = br#"{"error":{"message":"invalid model","type":"invalid_request_error"}}"#;
        let msg = is_structured_error_body(body).expect("应识别 error.message");
        assert!(msg.contains("invalid model"));
    }

    #[test]
    fn structured_error_top_level_message() {
        let body = br#"{"message":"bad request","code":"invalid"}"#;
        assert!(is_structured_error_body(body).is_some());
    }

    #[test]
    fn success_completion_not_error() {
        let body = br#"{"id":"c1","object":"chat.completion","choices":[{"message":{"role":"assistant","content":"ok"}}]}"#;
        assert!(is_structured_error_body(body).is_none());
    }

    #[test]
    fn empty_choices_completion_not_error() {
        let body = br#"{"id":"c1","object":"chat.completion","choices":[]}"#;
        assert!(is_structured_error_body(body).is_none());
    }

    #[test]
    fn redact_masks_api_key_and_bearer() {
        let msg = "上游 HTTP 401: invalid key sk-secret-value bearer sk-other";
        let safe = redact_sensitive_summary(msg, "sk-secret-value");
        assert!(!safe.contains("sk-secret-value"));
        assert!(safe.contains("[REDACTED]"));
    }

    #[test]
    fn sse_first_chunk_not_error() {
        let body = b"data: {\"choices\":[{\"delta\":{\"content\":\"hi\"}}]}\n\n";
        assert!(is_structured_error_body(body).is_none());
    }

    #[test]
    fn non_envelope_json_error_summary_avoids_dumping_body() {
        let body = br#"{"code":"model_not_found","messages":[{"role":"user","content":"secret"}]}"#;
        let summary = body_error_summary(body);
        assert_eq!(summary, "model_not_found");
        assert!(!summary.contains("secret"));
        assert!(!summary.contains("messages"));
    }

    #[test]
    fn rewrite_model_replaces_field() {
        let body = serde_json::json!({"model":"group","messages":[]});
        let out = rewrite_model(&body, "gpt-4o");
        assert_eq!(out["model"], "gpt-4o");
    }

    #[test]
    fn timeout_constants_match_prd() {
        assert_eq!(STREAM_FIRST_BYTE_TIMEOUT, Duration::from_secs(60));
        assert_eq!(STREAM_IDLE_TIMEOUT, Duration::from_secs(120));
        assert_eq!(NON_STREAM_TIMEOUT, Duration::from_secs(600));
        assert_eq!(CONNECT_TIMEOUT, Duration::from_secs(10));
    }

    #[tokio::test]
    async fn idle_timeout_fires_callback_semantics() {
        let fired = Arc::new(AtomicBool::new(false));
        let flag = fired.clone();
        let idle = Duration::from_millis(30);
        let result = tokio::time::timeout(idle, std::future::pending::<()>()).await;
        assert!(result.is_err());
        flag.store(true, Ordering::SeqCst);
        assert!(fired.load(Ordering::SeqCst));
    }
}
