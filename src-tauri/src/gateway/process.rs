use std::{
    path::Path,
    process::{Child, Command, Stdio},
    time::Duration,
};

use super::{
    binary::resolve_binary_path_with_resource,
    config::{write_config_file, GatewayRuntimeConfig},
    health::wait_until_reachable,
    state::{GatewayPhase, GatewayStatus},
};
use crate::error::AppError;

const START_TIMEOUT: Duration = Duration::from_secs(20);
const STOP_GRACE: Duration = Duration::from_secs(5);
const POLL_INTERVAL: Duration = Duration::from_millis(250);

pub struct GatewayRuntime {
    child: Option<Child>,
    status: GatewayStatus,
}

impl GatewayRuntime {
    pub fn new(host: String, port: u16, data_dir: String) -> Self {
        Self {
            child: None,
            status: GatewayStatus::new(host, port, data_dir),
        }
    }

    pub fn status_snapshot(&mut self) -> GatewayStatus {
        self.reap_if_exited();
        self.status.impl_name = "rust".to_string();
        self.status.clone()
    }

    /// 启动托管 model-hub-gateway；`resource_dir` 存在时从安装资源部署内嵌二进制。
    pub fn start_with_resource(
        &mut self,
        gateway_dir: &Path,
        bin_dir: &Path,
        resource_dir: Option<&Path>,
    ) -> Result<GatewayStatus, AppError> {
        self.status.impl_name = "rust".to_string();
        self.reap_if_exited();
        if matches!(
            self.status.state,
            GatewayPhase::Running | GatewayPhase::Starting
        ) {
            return Ok(self.status.clone());
        }

        let binary = resolve_binary_path_with_resource(bin_dir, resource_dir)?;
        let config = GatewayRuntimeConfig {
            host: self.status.host.clone(),
            port: self.status.port,
            ..GatewayRuntimeConfig::default_local(gateway_dir)
        };

        if is_port_busy(&config.host, config.port) {
            self.status.state = GatewayPhase::Error;
            let error = AppError::PortInUse { port: config.port };
            self.status.last_error = Some(error.to_string());
            return Err(error);
        }

        self.status.state = GatewayPhase::Starting;
        self.status.last_error = None;
        self.status.binary_path = Some(binary.display().to_string());
        self.status.data_dir = gateway_dir.display().to_string();
        self.status.base_url = format!("http://{}:{}", config.host, config.port);

        write_config_file(gateway_dir, &config)?;

        let mut command = Command::new(&binary);
        command
            .arg("--config")
            .arg(&config.config_relative)
            .current_dir(gateway_dir)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped());

        let mut child = command.spawn().map_err(|source| AppError::SpawnFailed {
            path: binary.display().to_string(),
            source,
        })?;

        let pid = child.id();
        self.status.pid = Some(pid);

        if !wait_until_reachable(&config.host, config.port, START_TIMEOUT, POLL_INTERVAL) {
            let _ = child.kill();
            let _ = child.wait();
            self.child = None;
            self.status.state = GatewayPhase::Error;
            self.status.pid = None;
            self.status.last_error = Some(format!(
                "网关进程已启动但在 {} 秒内未监听 {}:{}。请查看 gateway 数据目录日志，或确认二进制版本与 gateway/README.md 一致。",
                START_TIMEOUT.as_secs(),
                config.host,
                config.port
            ));
            return Err(AppError::HealthTimeout {
                host: config.host.clone(),
                port: config.port,
            });
        }

        match child.try_wait() {
            Ok(Some(_status)) => {
                self.child = None;
                self.status.state = GatewayPhase::Running;
                self.status.pid = Some(pid);
                self.status.last_error = Some(
                    "网关已监听，但启动进程已退出（可能已守护化）。停止时将尝试清理；若失败请手动结束占用端口的进程。"
                        .to_string(),
                );
                Ok(self.status.clone())
            }
            Ok(None) => {
                self.child = Some(child);
                self.status.state = GatewayPhase::Running;
                self.status.last_error = None;
                Ok(self.status.clone())
            }
            Err(source) => {
                let _ = child.kill();
                self.child = None;
                self.status.state = GatewayPhase::Error;
                self.status.pid = None;
                self.status.last_error = Some(format!("无法确认网关进程状态：{source}"));
                Err(AppError::ProcessStatus { source })
            }
        }
    }

    #[allow(dead_code)] // 单测与将来回滚路径保留
    pub fn restore_status(&mut self, status: GatewayStatus) {
        self.status = status;
    }

    /// 仅在 idle/error 时改端口（内存）。运行中请用 [`Self::apply_port_and_restart`]。
    #[allow(dead_code)] // 单测覆盖 idle/error 路径；生产走 apply_port_and_restart
    pub fn set_port(&mut self, port: u16) -> Result<GatewayStatus, AppError> {
        if !matches!(self.status.state, GatewayPhase::Idle | GatewayPhase::Error) {
            return Err(AppError::PortChangeWhileActive);
        }
        self.status.port = port;
        self.status.base_url = format!("http://{}:{}", self.status.host, port);
        self.status.last_error = None;
        Ok(self.status.clone())
    }

    /// 停止（若在运行）→ 更新端口 → 立即按新端口重新启动。
    pub fn apply_port_and_restart(
        &mut self,
        port: u16,
        gateway_dir: &Path,
        bin_dir: &Path,
        resource_dir: Option<&Path>,
    ) -> Result<GatewayStatus, AppError> {
        self.reap_if_exited();
        // 端口未变且已在运行：无需重启。
        if self.status.port == port && self.status.state == GatewayPhase::Running {
            return Ok(self.status.clone());
        }
        if matches!(
            self.status.state,
            GatewayPhase::Running | GatewayPhase::Starting | GatewayPhase::Stopping
        ) {
            let _ = self.stop();
        }
        // stop 后可能仍为 error（外部占用等）；端口仍应更新，再尝试 start。
        self.status.port = port;
        self.status.base_url = format!("http://{}:{}", self.status.host, port);
        self.status.last_error = None;
        self.status.state = GatewayPhase::Idle;
        self.start_with_resource(gateway_dir, bin_dir, resource_dir)
    }

    pub fn stop(&mut self) -> Result<GatewayStatus, AppError> {
        self.reap_if_exited();
        if self.child.is_none() && self.status.state != GatewayPhase::Running {
            if is_port_busy(&self.status.host, self.status.port) {
                self.status.state = GatewayPhase::Error;
                self.status.last_error = Some(format!(
                    "未托管子进程，但 {}:{} 仍在监听。请在任务管理器中结束对应网关进程后重试。",
                    self.status.host, self.status.port
                ));
                return Ok(self.status.clone());
            }
            self.status.state = GatewayPhase::Idle;
            self.status.pid = None;
            self.status.last_error = None;
            return Ok(self.status.clone());
        }

        self.status.state = GatewayPhase::Stopping;
        if let Some(mut child) = self.child.take() {
            let kill_result = child.kill();
            let wait_deadline = std::time::Instant::now() + STOP_GRACE;
            loop {
                match child.try_wait() {
                    Ok(Some(_)) => break,
                    Ok(None) if std::time::Instant::now() < wait_deadline => {
                        std::thread::sleep(Duration::from_millis(100));
                    }
                    Ok(None) => {
                        let _ = child.kill();
                        let _ = child.wait();
                        break;
                    }
                    Err(_) => {
                        let _ = child.kill();
                        break;
                    }
                }
            }
            if let Err(source) = kill_result {
                if source.kind() != std::io::ErrorKind::InvalidInput {
                    self.status.last_error = Some(format!(
                        "停止网关时出现问题：{source}。若进程仍在，请于任务管理器结束。"
                    ));
                }
            }
        }

        self.status.state = GatewayPhase::Idle;
        self.status.pid = None;
        Ok(self.status.clone())
    }

    fn reap_if_exited(&mut self) {
        let Some(child) = self.child.as_mut() else {
            return;
        };
        match child.try_wait() {
            Ok(Some(status)) => {
                self.child = None;
                if self.status.state == GatewayPhase::Running
                    || self.status.state == GatewayPhase::Starting
                {
                    self.status.state = GatewayPhase::Error;
                    self.status.last_error = Some(format!(
                        "网关进程意外退出（{status}）。请尝试重新启动；若反复失败请检查二进制与端口。"
                    ));
                }
                self.status.pid = None;
            }
            Ok(None) => {}
            Err(_) => {}
        }
    }
}

fn is_port_busy(host: &str, port: u16) -> bool {
    super::health::is_reachable(host, port, Duration::from_millis(150))
}

#[cfg(test)]
mod tests {
    use super::GatewayRuntime;
    use crate::{error::AppError, gateway::state::GatewayPhase};

    #[test]
    fn set_port_updates_status_and_base_url_when_stopped() {
        let mut runtime = GatewayRuntime::new("127.0.0.1".into(), 8080, "gateway".into());
        let status = runtime.set_port(18080).unwrap();
        assert_eq!(status.port, 18080);
        assert_eq!(status.base_url, "http://127.0.0.1:18080");
    }

    #[test]
    fn set_port_allows_recovery_from_error_and_clears_old_error() {
        let mut runtime = GatewayRuntime::new("127.0.0.1".into(), 8080, "gateway".into());
        runtime.status.state = GatewayPhase::Error;
        runtime.status.last_error = Some("端口被占用".into());
        let status = runtime.set_port(18080).unwrap();
        assert_eq!(status.state, GatewayPhase::Error);
        assert_eq!(status.last_error, None);
    }

    #[test]
    fn set_port_is_blocked_while_gateway_is_active() {
        for phase in [
            GatewayPhase::Starting,
            GatewayPhase::Running,
            GatewayPhase::Stopping,
        ] {
            let mut runtime = GatewayRuntime::new("127.0.0.1".into(), 8080, "gateway".into());
            runtime.status.state = phase;
            assert!(matches!(
                runtime.set_port(18080),
                Err(AppError::PortChangeWhileActive)
            ));
            assert_eq!(runtime.status.port, 8080);
        }
    }

    #[test]
    fn apply_port_and_restart_updates_port_even_when_start_fails() {
        let root = std::env::temp_dir().join(format!(
            "model-hub-port-restart-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();

        let mut runtime = GatewayRuntime::new("127.0.0.1".into(), 8080, root.display().to_string());
        runtime.status.state = GatewayPhase::Running;

        let err = runtime
            .apply_port_and_restart(18080, &root, &root, None)
            .unwrap_err();
        assert!(matches!(err, AppError::BinaryMissing { .. }));
        assert_eq!(runtime.status.port, 18080);
        assert_eq!(runtime.status.base_url, "http://127.0.0.1:18080");

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_exposes_rust_impl_name() {
        let mut runtime = GatewayRuntime::new("127.0.0.1".into(), 8080, "gateway".into());
        let status = runtime.status_snapshot();
        assert_eq!(status.impl_name, "rust");
    }
}
