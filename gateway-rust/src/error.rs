use std::io;
use std::path::PathBuf;

use thiserror::Error;

/// 网关启动与配置相关错误。
#[derive(Debug, Error)]
pub enum GatewayError {
    #[error("配置文件不存在: {path}")]
    ConfigNotFound { path: PathBuf },

    #[error("读取配置文件失败 ({path}): {source}")]
    ConfigRead {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("解析配置文件失败 ({path}): {source}")]
    ConfigParse {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },

    #[error("配置无效: {message}")]
    ConfigInvalid { message: String },

    #[error("绑定监听地址失败 ({addr}): {source}")]
    Bind {
        addr: String,
        #[source]
        source: io::Error,
    },

    #[error("HTTP 服务运行失败: {source}")]
    Serve {
        #[source]
        source: io::Error,
    },

    #[error("数据库错误: {message}")]
    Database { message: String },
}

impl GatewayError {
    pub fn invalid_config(message: impl Into<String>) -> Self {
        Self::ConfigInvalid {
            message: message.into(),
        }
    }

    pub fn database(message: impl Into<String>) -> Self {
        Self::Database {
            message: message.into(),
        }
    }
}
