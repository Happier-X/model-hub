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

/// 对一段 JSON 字节判定是否为明确的结构化错误信封（不含 SSE 帧解析）。
///
/// 规则（裸 JSON 与 SSE 帧内 data payload 共用，避免两套判定漂移）：
/// - 有 `choices` 且 `type != "error"` → 非错误；`type == "error"` → 错误
/// - 无 `choices` 且（字符串 `error` 非空 / 对象 `error.message` 非空 /
///   `type == "error"` / 顶层 `message` 非空且 `object != "chat.completion"`）→ 错误
/// - 非 JSON → 非错误
fn classify_json_error_envelope(bytes: &[u8]) -> Option<String> {
    let v: Value = serde_json::from_slice(bytes).ok()?;
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

/// 首字节（跳过前导空白后）是否形似 SSE 帧行前缀。
fn looks_like_sse(trimmed: &[u8]) -> bool {
    trimmed.starts_with(b"data:")
        || trimmed.starts_with(b"event:")
        || trimmed.starts_with(b":")
        || trimmed.starts_with(b"id:")
        || trimmed.starts_with(b"retry:")
}

/// 从 SSE 帧字节中抽取 `data:` 行拼接后的 payload。
///
/// - 按 `\n` / `\r\n` 拆行；收集 `data:`（可选单空格）行值，多行以 `\n` 拼接。
/// - 忽略 `event:` / `id:` / `retry:` / 注释行（`:` 开头）/ 空行。
/// - 无任何 `data:` 行、或拼接后去空白为空 → `None`（当正常 SSE 放行）。
/// - 仅解析首 chunk 内可见的事件，不跨 chunk 重组。
fn extract_sse_data_payload(trimmed: &[u8]) -> Option<Vec<u8>> {
    let text = std::str::from_utf8(trimmed).ok()?;
    let mut parts: Vec<&str> = Vec::new();
    for raw_line in text.split('\n') {
        let line = raw_line.strip_suffix('\r').unwrap_or(raw_line);
        if let Some(rest) = line.strip_prefix("data:") {
            // SSE 规范：`data:` 后可选单个前导空格。
            let value = rest.strip_prefix(' ').unwrap_or(rest);
            parts.push(value);
        }
        // 其它行（event:/id:/retry:/注释/空行）忽略。
    }
    if parts.is_empty() {
        return None;
    }
    let payload = parts.join("\n");
    if payload.trim().is_empty() {
        return None;
    }
    Some(payload.into_bytes())
}

/// 判断 body 是否为明确的结构化错误信封（非正常 chat completion / SSE）。
///
/// 识别裸 JSON 与 **SSE 帧内 `data:` payload** 两种形态；`data: [DONE]`、纯注释、
/// 无 `data:` 行、正常 delta（含 `choices`）一律放行。
/// 识别字段：字符串 `error`、对象 `error.message`、`type: "error"`、无 `choices` 时的顶层 `message`。
pub fn is_structured_error_body(bytes: &[u8]) -> Option<String> {
    let trimmed = bytes
        .iter()
        .position(|&b| !b.is_ascii_whitespace())
        .map(|i| &bytes[i..])
        .unwrap_or(bytes);
    if trimmed.is_empty() {
        return None;
    }

    // SSE 帧：剥出 data payload 再复用 JSON 错误信封判定；
    // 无 data / [DONE] / 纯注释等正常 SSE 放行，避免误伤正常流。
    if looks_like_sse(trimmed) {
        let payload = extract_sse_data_payload(trimmed)?;
        if payload
            .iter()
            .position(|&b| !b.is_ascii_whitespace())
            .map(|i| &payload[i..])
            .unwrap_or(&payload)
            == b"[DONE]"
        {
            return None;
        }
        return classify_json_error_envelope(&payload);
    }

    // 裸 JSON 路径。
    classify_json_error_envelope(trimmed)
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

fn rewrite_model(body: &Value, upstream_model: &str, effort: &str) -> Value {
    let mut v = body.clone();
    if let Some(obj) = v.as_object_mut() {
        obj.insert("model".into(), Value::String(upstream_model.to_string()));
        strip_tool_strict(obj);
        apply_thinking_effort(obj, upstream_model, effort);
    }
    v
}

/// 思考强度可注入的模型家族。
enum ThinkingFamily {
    /// OpenAI 推理系（gpt-5*、o1/o3/o4）。`supports_minimal` 仅原版 GPT-5 系为 true。
    OpenAiReasoning { supports_minimal: bool },
    /// Claude extended thinking（sonnet-4 / opus-4 / 3.7）。
    ClaudeThinking,
    /// Qwen3 思考模式（enable_thinking 顶层布尔字段，DashScope 形态）。
    QwenThinking,
    /// 其它一律不注入（gpt-4o、claude haiku、deepseek r1、qwen-turbo 等）。
    None,
}

/// 词界匹配 o1/o3/o4：`(^|[-_/])o[134]([-_/]|$)`。避免误伤 `o1` 出现在其它 token 中。
fn matches_o_series(model: &str) -> bool {
    let bytes = model.as_bytes();
    let is_sep = |b: u8| b == b'-' || b == b'_' || b == b'/';
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'o' {
            let prev_ok = i == 0 || is_sep(bytes[i - 1]);
            let has_digit = i + 1 < bytes.len() && matches!(bytes[i + 1], b'1' | b'3' | b'4');
            let next_ok = i + 2 >= bytes.len() || is_sep(bytes[i + 2]);
            if prev_ok && has_digit && next_ok {
                return true;
            }
        }
        i += 1;
    }
    false
}

/// 按 upstream 模型名（小写子串 + 词界）识别思考家族。
fn thinking_family(upstream_model: &str) -> ThinkingFamily {
    let m = upstream_model.to_ascii_lowercase();

    // OpenAI 推理系：gpt-5 / gpt5 → 支持 minimal；o1/o3/o4 → 不支持 minimal。
    if m.contains("gpt-5") || m.contains("gpt5") {
        return ThinkingFamily::OpenAiReasoning {
            supports_minimal: true,
        };
    }
    if matches_o_series(&m) {
        return ThinkingFamily::OpenAiReasoning {
            supports_minimal: false,
        };
    }

    // Claude extended thinking：需同时含 claude 与推理档型号标记。
    if m.contains("claude")
        && (m.contains("sonnet-4")
            || m.contains("sonnet4")
            || m.contains("opus-4")
            || m.contains("opus4")
            || m.contains("3-7")
            || m.contains("3.7"))
    {
        return ThinkingFamily::ClaudeThinking;
    }

    // Qwen3 思考：qwen3* 或 qwen 且含 3。
    if m.contains("qwen3") || (m.contains("qwen") && m.contains('3')) {
        return ThinkingFamily::QwenThinking;
    }

    ThinkingFamily::None
}

/// OpenAI `reasoning_effort` 档位映射；o 系不支持 minimal 时降级 low。
fn openai_reasoning_value(effort: &str, supports_minimal: bool) -> Option<&'static str> {
    match effort {
        "minimal" => Some(if supports_minimal { "minimal" } else { "low" }),
        "low" => Some("low"),
        "medium" => Some("medium"),
        "high" => Some("high"),
        "auto" => Some("medium"),
        _ => None,
    }
}

/// Claude `thinking.budget_tokens` 档位映射（均 ≥ 1024）。
fn claude_budget_tokens(effort: &str) -> Option<i64> {
    match effort {
        "minimal" => Some(2048),
        "low" => Some(4096),
        "medium" => Some(8192),
        "high" => Some(16384),
        "auto" => Some(8192),
        _ => None,
    }
}

/// 按 upstream 模型家族翻译思考强度档位为对应厂商字段。
///
/// - `effort == "off"` → 入口直接 return，对所有家族都不改动 body。
/// - 客户端已显式声明对应字段 → 保留，不覆盖。
/// - 未识别家族 / 非推理模型 → 不注入。
fn apply_thinking_effort(obj: &mut serde_json::Map<String, Value>, upstream_model: &str, effort: &str) {
    if effort == "off" {
        return;
    }
    match thinking_family(upstream_model) {
        ThinkingFamily::OpenAiReasoning { supports_minimal } => {
            if obj.contains_key("reasoning_effort") {
                return;
            }
            if let Some(value) = openai_reasoning_value(effort, supports_minimal) {
                obj.insert("reasoning_effort".into(), Value::String(value.into()));
            }
        }
        ThinkingFamily::ClaudeThinking => {
            if obj.contains_key("thinking") {
                return;
            }
            if let Some(budget) = claude_budget_tokens(effort) {
                obj.insert(
                    "thinking".into(),
                    serde_json::json!({ "type": "enabled", "budget_tokens": budget }),
                );
            }
        }
        ThinkingFamily::QwenThinking => {
            if obj.contains_key("enable_thinking") {
                return;
            }
            // 非 off 档一律开启（off 已在入口 return）。
            obj.insert("enable_thinking".into(), Value::Bool(true));
        }
        ThinkingFamily::None => {}
    }
}

/// 剥离 `tools[].function.strict`。
///
/// 该字段是 OpenAI Structured Outputs 特性，部分兼容上游不支持，原样透传会报
/// `tool.function.strict is not supported`。移除仅关闭上游侧严格 schema 校验，
/// 不改变工具定义与调用语义（下游仍返回 JSON 字符串 arguments，客户端照常解析）。
fn strip_tool_strict(obj: &mut serde_json::Map<String, Value>) {
    let Some(tools) = obj.get_mut("tools").and_then(|t| t.as_array_mut()) else {
        return;
    };
    for tool in tools.iter_mut() {
        if let Some(function) = tool.get_mut("function").and_then(|f| f.as_object_mut()) {
            function.remove("strict");
        }
    }
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
    effort: &str,
) -> Result<(StatusCode, HeaderMap, Bytes), AttemptError> {
    let url = chat_url(&candidate.provider.base_url);
    let payload = rewrite_model(body, &candidate.upstream_model, effort);
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
    effort: &str,
) -> Result<StreamPrimeOk, AttemptError> {
    let url = chat_url(&candidate.provider.base_url);
    let payload = rewrite_model(body, &candidate.upstream_model, effort);
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

/// 构造 JSON 网关错误响应（OpenAI 兼容格式），用于 exhausted 路径升级 2xx 错误信封。
fn build_gateway_error_response(status: u16, message: &str) -> Response {
    let body = serde_json::json!({
        "message": message,
        "error": {
            "message": message
        }
    });
    let mut builder =
        Response::builder().status(StatusCode::from_u16(status).unwrap_or(StatusCode::BAD_GATEWAY));
    builder = builder.header("Content-Type", "application/json");
    builder
        .body(Body::from(serde_json::to_vec(&body).unwrap_or_default()))
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
    effort: &str,
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
            match attempt_stream_prime(clients, candidate, body, effort).await {
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
            match attempt_non_stream(clients, candidate, body, effort).await {
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
    // 特别注意：若最后响应是 2xx 结构化错误信封（如 HTTP 200 + JSON 错误体），
    // 不应当原样透传误导客户端，而应升级为 502 网关错误，附带上游原始错误详情。
    if let Some((status, headers, body, provider_name, model, safe_error)) = last_http {
        // 2xx 错误信封 → 转换为 502，避免客户端误认为请求成功。
        if (200..300).contains(&status) && is_structured_error_body(&body).is_some() {
            let response = build_gateway_error_response(
                502,
                &format!("所有上游均返回错误，最后错误：{safe_error}"),
            );
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
    fn sse_data_error_string_failovers() {
        let body = b"data: {\"error\":\"Invalid or expired credentials\"}\n\n";
        let msg = is_structured_error_body(body).expect("SSE 帧字符串 error 应识别");
        assert!(msg.contains("Invalid or expired credentials"));
    }

    #[test]
    fn sse_data_error_object_message() {
        let body =
            b"data: {\"error\":{\"message\":\"No available accounts. Add an account first.\"}}\n\n";
        let msg = is_structured_error_body(body).expect("SSE 帧 error.message 应识别");
        assert!(msg.contains("No available accounts"));
    }

    #[test]
    fn sse_data_type_error_failovers() {
        let body = b"data: {\"type\":\"error\",\"error\":{\"message\":\"stream failed\"}}\n\n";
        let msg = is_structured_error_body(body).expect("SSE 帧 type=error 应识别");
        assert!(msg.contains("stream failed"));
    }

    #[test]
    fn sse_done_not_error() {
        let body = b"data: [DONE]\n\n";
        assert!(is_structured_error_body(body).is_none());
    }

    #[test]
    fn sse_comment_only_not_error() {
        let body = b": ping\n\n";
        assert!(is_structured_error_body(body).is_none());
    }

    #[test]
    fn sse_event_only_not_error() {
        let body = b"event: message\nid: 1\n\n";
        assert!(is_structured_error_body(body).is_none());
    }

    #[test]
    fn sse_crlf_data_error_failovers() {
        let body = b"data: {\"error\":\"bad key\"}\r\n\r\n";
        let msg = is_structured_error_body(body).expect("CRLF SSE 帧 error 应识别");
        assert!(msg.contains("bad key"));
    }

    #[test]
    fn sse_multiline_data_error_failovers() {
        // 多个行首 data: 行按 \n 拼接为完整 JSON 错误信封（SSE 规范多 data 行做法）。
        let body = b"data: {\"error\":\ndata: \"split across lines\"}\n\n";
        let msg = is_structured_error_body(body).expect("多行 data 拼接后应识别");
        assert!(msg.contains("split across lines"));
    }

    #[test]
    fn sse_data_completion_delta_not_error() {
        // 首包为正常 delta，即使无 choices 外层也不应误判（有 choices）。
        let body = b"data: {\"id\":\"c1\",\"choices\":[{\"delta\":{\"content\":\"hi\"}}]}\n\n";
        assert!(is_structured_error_body(body).is_none());
    }

    #[test]
    fn sse_data_non_json_text_not_error() {
        // SSE 里的非 JSON 文本不当错误信封（避免误伤）。
        let body = b"data: keep-alive\n\n";
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
        let out = rewrite_model(&body, "gpt-4o", "off");
        assert_eq!(out["model"], "gpt-4o");
    }

    #[test]
    fn rewrite_model_strips_tool_strict() {
        let body = serde_json::json!({
            "model": "group",
            "messages": [],
            "tools": [
                {
                    "type": "function",
                    "function": {
                        "name": "get_weather",
                        "strict": true,
                        "parameters": {"type": "object"}
                    }
                },
                {
                    "type": "function",
                    "function": {
                        "name": "no_strict",
                        "parameters": {"type": "object"}
                    }
                }
            ]
        });
        let out = rewrite_model(&body, "gpt-4o", "off");
        // strict 已剥离，其余字段保留。
        assert!(out["tools"][0]["function"].get("strict").is_none());
        assert_eq!(out["tools"][0]["function"]["name"], "get_weather");
        assert_eq!(out["tools"][0]["function"]["parameters"]["type"], "object");
        assert!(out["tools"][1]["function"].get("strict").is_none());
        assert_eq!(out["tools"][1]["function"]["name"], "no_strict");
    }

    #[test]
    fn rewrite_model_without_tools_is_noop() {
        let body = serde_json::json!({"model":"group","messages":[]});
        let out = rewrite_model(&body, "gpt-4o", "off");
        assert!(out.get("tools").is_none());
    }

    // ---- 思考强度：家族识别 ----

    #[test]
    fn family_gpt5_supports_minimal() {
        assert!(matches!(
            thinking_family("gpt-5"),
            ThinkingFamily::OpenAiReasoning { supports_minimal: true }
        ));
        assert!(matches!(
            thinking_family("gpt-5-mini"),
            ThinkingFamily::OpenAiReasoning { supports_minimal: true }
        ));
    }

    #[test]
    fn family_o_series_no_minimal() {
        assert!(matches!(
            thinking_family("o3"),
            ThinkingFamily::OpenAiReasoning { supports_minimal: false }
        ));
        assert!(matches!(
            thinking_family("o1-preview"),
            ThinkingFamily::OpenAiReasoning { supports_minimal: false }
        ));
        // 词界：o1 出现在其它 token 中间不误伤。
        assert!(matches!(thinking_family("model-o1x"), ThinkingFamily::None));
    }

    #[test]
    fn family_claude_thinking() {
        assert!(matches!(
            thinking_family("claude-sonnet-4-20250514"),
            ThinkingFamily::ClaudeThinking
        ));
        assert!(matches!(
            thinking_family("claude-3-7-sonnet"),
            ThinkingFamily::ClaudeThinking
        ));
        // Claude haiku 不注入。
        assert!(matches!(thinking_family("claude-3-haiku"), ThinkingFamily::None));
    }

    #[test]
    fn family_qwen3_thinking() {
        assert!(matches!(thinking_family("qwen3-32b"), ThinkingFamily::QwenThinking));
        assert!(matches!(thinking_family("qwen3-235b-a22b"), ThinkingFamily::QwenThinking));
        // qwen-turbo 不注入。
        assert!(matches!(thinking_family("qwen-turbo"), ThinkingFamily::None));
    }

    // ---- 思考强度：注入行为 ----

    #[test]
    fn off_never_injects_any_family() {
        for model in ["gpt-5", "o3", "claude-sonnet-4", "qwen3-32b"] {
            let out = rewrite_model(&serde_json::json!({"model":"g","messages":[]}), model, "off");
            let obj = out.as_object().unwrap();
            assert!(obj.get("reasoning_effort").is_none());
            assert!(obj.get("thinking").is_none());
            assert!(obj.get("enable_thinking").is_none());
        }
    }

    #[test]
    fn openai_injects_reasoning_effort() {
        let out = rewrite_model(&serde_json::json!({"model":"g","messages":[]}), "gpt-5", "high");
        assert_eq!(out["reasoning_effort"], "high");
        // auto → medium。
        let out = rewrite_model(&serde_json::json!({"model":"g"}), "gpt-5", "auto");
        assert_eq!(out["reasoning_effort"], "medium");
    }

    #[test]
    fn o_series_minimal_downgrades_to_low() {
        let out = rewrite_model(&serde_json::json!({"model":"g"}), "o3", "minimal");
        assert_eq!(out["reasoning_effort"], "low");
        // gpt-5 保留 minimal。
        let out = rewrite_model(&serde_json::json!({"model":"g"}), "gpt-5", "minimal");
        assert_eq!(out["reasoning_effort"], "minimal");
    }

    #[test]
    fn claude_injects_thinking_budget() {
        let out = rewrite_model(&serde_json::json!({"model":"g"}), "claude-sonnet-4", "medium");
        assert_eq!(out["thinking"]["type"], "enabled");
        assert_eq!(out["thinking"]["budget_tokens"], 8192);
    }

    #[test]
    fn qwen_injects_enable_thinking_true() {
        let out = rewrite_model(&serde_json::json!({"model":"g"}), "qwen3-32b", "low");
        assert_eq!(out["enable_thinking"], true);
    }

    #[test]
    fn client_field_not_overwritten() {
        // 客户端已带 reasoning_effort，保留不覆盖。
        let out = rewrite_model(
            &serde_json::json!({"model":"g","reasoning_effort":"low"}),
            "gpt-5",
            "high",
        );
        assert_eq!(out["reasoning_effort"], "low");
        // 客户端已带 thinking，保留。
        let out = rewrite_model(
            &serde_json::json!({"model":"g","thinking":{"type":"enabled","budget_tokens":100}}),
            "claude-sonnet-4",
            "high",
        );
        assert_eq!(out["thinking"]["budget_tokens"], 100);
    }

    #[test]
    fn non_reasoning_family_never_injects() {
        let out = rewrite_model(&serde_json::json!({"model":"g"}), "gpt-4o", "high");
        let obj = out.as_object().unwrap();
        assert!(obj.get("reasoning_effort").is_none());
        assert!(obj.get("thinking").is_none());
        assert!(obj.get("enable_thinking").is_none());
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
