//! 从上游 OpenAI 兼容 `GET …/models` 拉取模型 id 列表（管理侧，不经故障转移）。

use std::collections::HashSet;
use std::time::Duration;

use serde::Deserialize;
use serde_json::Value;

use crate::error::AppError;

/// 拉取上游模型列表的超时（连接 + 整次请求）。
pub const FETCH_MODELS_TIMEOUT: Duration = Duration::from_secs(15);
pub const FETCH_MODELS_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

/// 将供应商 `base_url`（已含 `/v1`、无尾斜杠约定）拼成 models 端点。
pub fn models_url(base_url: &str) -> Result<String, AppError> {
    let base = base_url.trim().trim_end_matches('/');
    if base.is_empty() {
        return Err(AppError::Business("Base URL 不能为空".into()));
    }
    Ok(format!("{base}/models"))
}

/// 解析 OpenAI 风格 `{ "data": [ { "id": "..." }, ... ] }`，返回有序去重的 id 列表。
pub fn parse_models_list(body: &str) -> Result<Vec<String>, AppError> {
    let value: Value = serde_json::from_str(body)
        .map_err(|_| AppError::Business("无法解析模型列表：响应不是有效 JSON".into()))?;

    let data = value
        .get("data")
        .and_then(|d| d.as_array())
        .ok_or_else(|| AppError::Business("无法解析模型列表：缺少 data 数组".into()))?;

    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for item in data {
        let Some(id) = item.get("id").and_then(|v| v.as_str()) else {
            continue;
        };
        let id = id.trim();
        if id.is_empty() {
            continue;
        }
        if seen.insert(id.to_string()) {
            out.push(id.to_string());
        }
    }
    Ok(out)
}

fn map_http_status(status: u16) -> AppError {
    match status {
        401 => AppError::Business("上游返回 401：请检查 API Key".into()),
        403 => {
            AppError::Business("上游返回 403：无权限访问模型列表，请检查 API Key 或账号权限".into())
        }
        404 => AppError::Business(
            "上游返回 404：未找到 /models 接口，请确认 Base URL 是否含 /v1".into(),
        ),
        429 => AppError::Business("上游返回 429：请求过于频繁，请稍后再试".into()),
        s if (500..600).contains(&s) => {
            AppError::Business(format!("上游服务异常（HTTP {s}），请稍后重试"))
        }
        s => AppError::Business(format!("上游返回 HTTP {s}，无法获取模型列表")),
    }
}

fn map_reqwest_error(err: reqwest::Error) -> AppError {
    if err.is_timeout() {
        return AppError::Business(
            "请求超时：上游未在限定时间内响应，请检查网络或 Base URL".into(),
        );
    }
    if err.is_connect() {
        return AppError::Business("无法连接上游：请检查 Base URL 与网络连通性".into());
    }
    // 禁止把可能含 URL 查询串或重定向细节的完整错误原文原样回显；给可行动摘要。
    AppError::Business(format!(
        "请求上游失败：{}",
        sanitize_network_message(&err.to_string())
    ))
}

/// 去掉错误串中疑似密钥片段，避免意外回显。
fn sanitize_network_message(msg: &str) -> String {
    // 常见 Bearer / key= 片段粗略脱敏
    let mut s = msg.to_string();
    for marker in ["Bearer ", "bearer ", "api_key=", "api-key=", "key="] {
        if let Some(idx) = s.find(marker) {
            let start = idx + marker.len();
            let rest = &s[start..];
            let end = rest
                .find(|c: char| c.is_whitespace() || c == '"' || c == '\'' || c == ',' || c == '}')
                .unwrap_or(rest.len());
            if end > 4 {
                s.replace_range(start..start + end, "***");
            }
        }
    }
    s
}

/// 对指定 base_url + api_key 发起 GET `{base}/models`。
pub async fn fetch_upstream_model_ids(
    base_url: &str,
    api_key: &str,
) -> Result<Vec<String>, AppError> {
    let url = models_url(base_url)?;
    if api_key.trim().is_empty() {
        return Err(AppError::Business("API Key 不能为空".into()));
    }

    let client = reqwest::Client::builder()
        .timeout(FETCH_MODELS_TIMEOUT)
        .connect_timeout(FETCH_MODELS_CONNECT_TIMEOUT)
        .build()
        .map_err(|e| AppError::Business(format!("无法创建 HTTP 客户端：{e}")))?;

    let response = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", api_key.trim()))
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(map_reqwest_error)?;

    let status = response.status();
    if !status.is_success() {
        return Err(map_http_status(status.as_u16()));
    }

    let body = response.text().await.map_err(|e| {
        AppError::Business(format!(
            "读取上游响应失败：{}",
            sanitize_network_message(&e.to_string())
        ))
    })?;

    parse_models_list(&body)
}

#[derive(Debug, Deserialize)]
pub struct FetchProviderModelsPayload {
    /// 已保存供应商 id（优先）。
    pub provider_id: Option<i64>,
    /// 表单草稿：未保存时直接探测。
    pub base_url: Option<String>,
    pub api_key: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn models_url_trims_slash() {
        assert_eq!(
            models_url("https://api.openai.com/v1/").unwrap(),
            "https://api.openai.com/v1/models"
        );
        assert_eq!(models_url(" https://x/v1 ").unwrap(), "https://x/v1/models");
    }

    #[test]
    fn models_url_rejects_empty() {
        assert!(models_url("  ").is_err());
        assert!(models_url("///").is_err());
    }

    #[test]
    fn parse_standard_list() {
        let body = r#"{
            "object": "list",
            "data": [
                {"id": "gpt-4o", "object": "model"},
                {"id": "gpt-4o-mini", "object": "model"},
                {"id": "gpt-4o", "object": "model"}
            ]
        }"#;
        let ids = parse_models_list(body).unwrap();
        assert_eq!(ids, vec!["gpt-4o", "gpt-4o-mini"]);
    }

    #[test]
    fn parse_empty_data() {
        let ids = parse_models_list(r#"{"data":[]}"#).unwrap();
        assert!(ids.is_empty());
    }

    #[test]
    fn parse_skips_missing_id() {
        let body = r#"{"data":[{"object":"model"},{"id":"m1"},{"id":"  "}]}"#;
        let ids = parse_models_list(body).unwrap();
        assert_eq!(ids, vec!["m1"]);
    }

    #[test]
    fn parse_rejects_non_json() {
        let err = parse_models_list("not-json").unwrap_err();
        assert!(err.to_string().contains("无法解析"));
    }

    #[test]
    fn parse_rejects_missing_data() {
        let err = parse_models_list(r#"{"object":"list"}"#).unwrap_err();
        assert!(err.to_string().contains("data"));
    }

    #[test]
    fn sanitize_masks_bearer() {
        let s = sanitize_network_message("error Bearer sk-secret-value more");
        assert!(!s.contains("sk-secret-value"));
        assert!(s.contains("***"));
    }

    #[test]
    fn map_status_401_chinese() {
        let e = map_http_status(401);
        assert!(e.to_string().contains("401"));
        assert!(e.to_string().contains("API Key"));
    }
}
