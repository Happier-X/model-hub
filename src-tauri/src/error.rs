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
    #[error("无法写入网关配置“{path}”：{source}")]
    WriteConfig {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("网关正在切换状态，请稍候再修改监听端口")]
    PortChangeWhileActive,
    #[error("无法序列化网关配置“{path}”：{source}")]
    SerializeConfig {
        path: String,
        #[source]
        source: serde_json::Error,
    },
    #[error("未找到网关程序“{path}”。{hint}")]
    BinaryMissing { path: String, hint: String },
    #[error("无法部署内置网关到“{path}”：{source}")]
    BinaryDeployFailed {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("端口 {port} 已被占用。请到设置页修改“网关监听端口”后保存（将自动重启）；应用不会自动结束占用端口的进程。")]
    PortInUse { port: u16 },
    #[error("端口必须是 1 到 65535 之间的整数")]
    InvalidPort,

    #[error("启动网关失败（{path}）：{source}")]
    SpawnFailed {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("网关未在超时内监听 {host}:{port}")]
    HealthTimeout { host: String, port: u16 },
    #[error("无法读取网关进程状态：{source}")]
    ProcessStatus {
        #[source]
        source: std::io::Error,
    },
    #[error("网关状态锁已损坏，请重启应用")]
    GatewayLockPoisoned,
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
            Self::WriteConfig { .. } | Self::SerializeConfig { .. } => "GATEWAY_CONFIG_FAILED",
            Self::PortInUse { .. } => "GATEWAY_PORT_IN_USE",
            Self::InvalidPort => "GATEWAY_INVALID_PORT",
            Self::PortChangeWhileActive => "GATEWAY_PORT_CHANGE_BLOCKED",
            Self::BinaryMissing { .. } => "GATEWAY_BINARY_MISSING",
            Self::BinaryDeployFailed { .. } => "GATEWAY_BINARY_DEPLOY_FAILED",
            Self::SpawnFailed { .. } => "GATEWAY_SPAWN_FAILED",
            Self::HealthTimeout { .. } => "GATEWAY_HEALTH_TIMEOUT",
            Self::ProcessStatus { .. } => "GATEWAY_PROCESS_ERROR",
            Self::GatewayLockPoisoned => "GATEWAY_LOCK_POISONED",
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
