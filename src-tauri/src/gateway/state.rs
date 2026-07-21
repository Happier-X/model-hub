use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GatewayPhase {
    Idle,
    Starting,
    Running,
    Stopping,
    Error,
}

#[derive(Debug, Clone, Serialize)]
pub struct GatewayStatus {
    pub state: GatewayPhase,
    pub host: String,
    pub port: u16,
    pub pid: Option<u32>,
    pub last_error: Option<String>,
    pub base_url: String,
    pub data_dir: String,
    pub binary_path: Option<String>,
    /// 固定为 `rust`（仅原生网关）；保留字段以兼容前端可选读取。
    pub impl_name: String,
}

impl GatewayStatus {
    pub fn new(host: impl Into<String>, port: u16, data_dir: impl Into<String>) -> Self {
        let host = host.into();
        let data_dir = data_dir.into();
        Self {
            state: GatewayPhase::Idle,
            base_url: format!("http://{host}:{port}"),
            host,
            port,
            pid: None,
            last_error: None,
            data_dir,
            binary_path: None,
            impl_name: "rust".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{GatewayPhase, GatewayStatus};

    #[test]
    fn builds_base_url_and_default_idle_state() {
        let status = GatewayStatus::new("127.0.0.1", 8080, "C:/data/gateway");
        assert_eq!(status.state, GatewayPhase::Idle);
        assert_eq!(status.base_url, "http://127.0.0.1:8080");
        assert_eq!(status.data_dir, "C:/data/gateway");
        assert_eq!(status.pid, None);
        assert_eq!(status.impl_name, "rust");
    }

    #[test]
    fn phase_variants_exist_for_state_machine() {
        assert_ne!(GatewayPhase::Idle, GatewayPhase::Running);
        assert_ne!(GatewayPhase::Error, GatewayPhase::Starting);
    }
}
