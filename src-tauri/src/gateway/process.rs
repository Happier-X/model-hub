use std::{
    path::Path,
    process::{Child, Command, Stdio},
    time::Duration,
};

use super::{
    binary::resolve_binary_path,
    config::{env_overrides, write_config_file, GatewayRuntimeConfig},
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
        self.status.clone()
    }

    pub fn start(&mut self, gateway_dir: &Path, bin_dir: &Path) -> Result<GatewayStatus, AppError> {
        self.reap_if_exited();
        if matches!(
            self.status.state,
            GatewayPhase::Running | GatewayPhase::Starting
        ) {
            return Ok(self.status.clone());
        }

        let binary = resolve_binary_path(bin_dir)?;
        let config = GatewayRuntimeConfig {
            host: self.status.host.clone(),
            port: self.status.port,
            ..GatewayRuntimeConfig::default_local(gateway_dir)
        };

        if is_port_busy(&config.host, config.port) {
            self.status.state = GatewayPhase::Error;
            self.status.last_error = Some(format!(
                "端口 {} 已被占用。请在设置中更换端口，或结束占用该端口的进程后重试。",
                config.port
            ));
            return Err(AppError::PortInUse { port: config.port });
        }

        self.status.state = GatewayPhase::Starting;
        self.status.last_error = None;
        self.status.binary_path = Some(binary.display().to_string());
        self.status.data_dir = gateway_dir.display().to_string();
        self.status.base_url = format!("http://{}:{}", config.host, config.port);

        write_config_file(gateway_dir, &config)?;

        let mut command = Command::new(&binary);
        command
            .arg("start")
            .arg("--config")
            .arg(&config.config_relative)
            .current_dir(gateway_dir)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped());

        for (key, value) in env_overrides(&config) {
            command.env(key, value);
        }

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

        // 健康检查通过后：父进程可能常驻，也可能守护化后退出（端口仍被占用）。
        match child.try_wait() {
            Ok(Some(_status)) => {
                // 父进程已退出但端口可达 → 视为外部托管的 running（stop 时按端口探测提示）。
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

    pub fn stop(&mut self) -> Result<GatewayStatus, AppError> {
        self.reap_if_exited();
        if self.child.is_none() && self.status.state != GatewayPhase::Running {
            // 若端口仍被占用（守护化残留），给出可行动提示。
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
            // Best-effort graceful request: on Windows Child has no SIGTERM; kill is the API.
            // Document that octopus should flush on process termination when possible.
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
                // Process may already have exited.
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
