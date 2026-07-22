use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("无法解析应用数据目录：{0}")]
    Path(#[from] tauri::Error),
    #[error("无法创建数据目录“{path}”：{source}")]
    CreateDirectory {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("无法读取壳配置“{path}”：{source}")]
    ReadShellConfig {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("无法写入壳配置“{path}”：{source}")]
    WriteShellConfig {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("无法序列化壳配置“{path}”：{source}")]
    SerializeShellConfig {
        path: String,
        #[source]
        source: serde_json::Error,
    },
    #[error("端口必须是 1 到 65535 之间的整数")]
    InvalidPort,
    #[error("代理正在切换状态，请稍候再修改监听端口")]
    PortChangeWhileActive,
    #[error("端口 {port} 已被占用。应用会自动尝试后续端口；若全部失败请手动修改监听端口。不会结束占用端口的进程。")]
    PortInUse { port: u16 },
    #[error(
        "端口 {preferred} 起连续 {attempts} 个端口均不可用（试到 {last_tried}）。请手动指定可用端口后保存；不会结束占用进程。"
    )]
    NoAvailablePort {
        preferred: u16,
        attempts: u16,
        last_tried: u16,
    },
    #[error("代理启动失败：{0}")]
    ProxyStart(String),
    #[error("数据库错误：{0}")]
    Database(String),
    #[error("业务错误：{0}")]
    Business(String),
    #[error("代理状态锁已损坏，请重启应用")]
    LockPoisoned,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InvokeError {
    pub code: &'static str,
    pub message: String,
}

impl AppError {
    fn code(&self) -> &'static str {
        match self {
            Self::Path(_) | Self::CreateDirectory { .. } => "PATH_INITIALIZATION_FAILED",
            Self::ReadShellConfig { .. } => "SHELL_CONFIG_READ_FAILED",
            Self::WriteShellConfig { .. } | Self::SerializeShellConfig { .. } => {
                "SHELL_CONFIG_FAILED"
            }
            Self::InvalidPort => "PROXY_INVALID_PORT",
            Self::PortChangeWhileActive => "PROXY_PORT_CHANGE_BLOCKED",
            Self::PortInUse { .. } => "PROXY_PORT_IN_USE",
            Self::NoAvailablePort { .. } => "PROXY_NO_AVAILABLE_PORT",
            Self::ProxyStart(_) => "PROXY_START_FAILED",
            Self::Database(_) => "DATABASE_ERROR",
            Self::Business(_) => "BUSINESS_ERROR",
            Self::LockPoisoned => "PROXY_LOCK_POISONED",
        }
    }
}

impl From<AppError> for InvokeError {
    fn from(error: AppError) -> Self {
        Self {
            code: error.code(),
            message: error.to_string(),
        }
    }
}
