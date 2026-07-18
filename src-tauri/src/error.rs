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
    #[error("无法写入网关配置“{path}”：{source}")]
    WriteConfig {
        path: String,
        #[source]
        source: std::io::Error,
    },
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
    #[error("端口 {port} 已被占用。请更换端口或结束占用进程后重试。")]
    PortInUse { port: u16 },
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
            Self::WriteConfig { .. } | Self::SerializeConfig { .. } => "GATEWAY_CONFIG_FAILED",
            Self::BinaryMissing { .. } => "GATEWAY_BINARY_MISSING",
            Self::BinaryDeployFailed { .. } => "GATEWAY_BINARY_DEPLOY_FAILED",
            Self::PortInUse { .. } => "GATEWAY_PORT_IN_USE",
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
