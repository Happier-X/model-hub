//! 请求日志模型（对齐 UI `RelayLog`）。

use serde::Serialize;

/// 列表返回的单条请求日志。
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct RelayLog {
    pub id: i64,
    /// Unix 秒。
    pub time: i64,
    pub request_model_name: String,
    pub channel_name: String,
    pub actual_model_name: String,
    pub input_tokens: i64,
    pub output_tokens: i64,
    /// 耗时（秒）。
    pub use_time: i64,
    pub cost: f64,
    pub error: String,
}

/// 插入用的新日志（无 id）。
#[derive(Debug, Clone)]
pub struct NewRelayLog {
    pub time: i64,
    pub request_model_name: String,
    pub channel_name: String,
    pub actual_model_name: String,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub use_time: i64,
    pub cost: f64,
    pub error: String,
}

impl NewRelayLog {
    pub fn simple(
        request_model_name: impl Into<String>,
        channel_name: impl Into<String>,
        actual_model_name: impl Into<String>,
        use_time: i64,
        error: impl Into<String>,
    ) -> Self {
        Self {
            time: chrono::Utc::now().timestamp(),
            request_model_name: request_model_name.into(),
            channel_name: channel_name.into(),
            actual_model_name: actual_model_name.into(),
            input_tokens: 0,
            output_tokens: 0,
            use_time,
            cost: 0.0,
            error: error.into(),
        }
    }
}

/// 错误消息截断上限。
pub const ERROR_TRUNCATE_CHARS: usize = 512;

/// 截断错误摘要（不落完整 messages / 密钥）。
pub fn truncate_error(msg: &str) -> String {
    let msg = msg.trim();
    if msg.chars().count() <= ERROR_TRUNCATE_CHARS {
        return msg.to_string();
    }
    msg.chars().take(ERROR_TRUNCATE_CHARS).collect()
}
