use serde::Serialize;

use super::impl_kind::{resolve_gateway_impl, GatewayImpl};

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
    /// 当前选择的网关实现：`octopus` | `rust`（由环境变量解析，可选字段语义上兼容旧前端）。
    pub impl_name: String,
}

impl GatewayStatus {
    /// 使用当前环境解析的实现构造状态（与 [`resolve_gateway_impl`] 一致）。
    #[allow(dead_code)] // 便捷构造；runtime 路径使用 with_impl 以保证与 impl_kind 同步
    pub fn new(host: impl Into<String>, port: u16, data_dir: impl Into<String>) -> Self {
        Self::with_impl(host, port, data_dir, resolve_gateway_impl())
    }

    pub fn with_impl(
        host: impl Into<String>,
        port: u16,
        data_dir: impl Into<String>,
        impl_kind: GatewayImpl,
    ) -> Self {
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
            impl_name: impl_kind.as_str().to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{GatewayPhase, GatewayStatus};
    use crate::gateway::impl_kind::GatewayImpl;

    #[test]
    fn builds_base_url_and_default_idle_state() {
        let status =
            GatewayStatus::with_impl("127.0.0.1", 8080, "C:/data/gateway", GatewayImpl::Octopus);
        assert_eq!(status.state, GatewayPhase::Idle);
        assert_eq!(status.base_url, "http://127.0.0.1:8080");
        assert_eq!(status.data_dir, "C:/data/gateway");
        assert_eq!(status.pid, None);
        assert_eq!(status.impl_name, "octopus");
    }

    #[test]
    fn rust_impl_name_is_exposed() {
        let status =
            GatewayStatus::with_impl("127.0.0.1", 8080, "C:/data/gateway", GatewayImpl::Rust);
        assert_eq!(status.impl_name, "rust");
    }

    #[test]
    fn phase_variants_exist_for_state_machine() {
        assert_ne!(GatewayPhase::Idle, GatewayPhase::Running);
        assert_ne!(GatewayPhase::Error, GatewayPhase::Starting);
    }
}
