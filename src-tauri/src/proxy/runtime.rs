//! 代理生命周期：start/stop/status/set_port。

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;

use serde::Serialize;
use tokio::sync::oneshot;

use crate::db::{default_db_path, open_db};
use crate::domain::Stores;
use crate::error::AppError;
use crate::proxy::forward::{ForwardPolicy, UpstreamClients};
use crate::proxy::server::{self, AppState};
use crate::settings::{self, DEFAULT_PORT};

pub const DEFAULT_HOST: &str = "127.0.0.1";
/// 首选端口被占用时，向后连续尝试的端口数量（含首选）。
pub const PORT_SCAN_ATTEMPTS: u16 = 50;
/// `stop` 时等待 graceful shutdown 的最长时间；超时则 abort 服务任务以释放端口。
pub const PROXY_STOP_GRACE: Duration = Duration::from_secs(3);

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProxyState {
    Idle,
    Starting,
    Running,
    Stopping,
    Error,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProxyStatus {
    pub state: ProxyState,
    pub host: String,
    pub port: u16,
    pub last_error: Option<String>,
    pub base_url: String,
    pub data_dir: String,
    /// 启动时若因占用自动改口，给出中文说明；否则为空。
    #[serde(default)]
    pub port_note: Option<String>,
}

struct LiveProxy {
    shutdown_tx: oneshot::Sender<()>,
    join: tokio::task::JoinHandle<()>,
}

struct RuntimeInner {
    host: String,
    port: u16,
    data_dir: String,
    /// 用于启动成功后回写 `shell.json`；可为空（测试）。
    config_dir: Option<PathBuf>,
    state: ProxyState,
    last_error: Option<String>,
    port_note: Option<String>,
    live: Option<LiveProxy>,
    stores: Option<Stores>,
    clients: UpstreamClients,
}

pub struct ProxyHandle {
    inner: Mutex<RuntimeInner>,
    /// 独立 tokio runtime，承载 axum
    tokio_rt: tokio::runtime::Runtime,
}

impl ProxyHandle {
    pub fn new(data_dir: String, port: u16) -> Result<Self, AppError> {
        Self::new_with_config_dir(data_dir, port, None)
    }

    pub fn new_with_config_dir(
        data_dir: String,
        port: u16,
        config_dir: Option<PathBuf>,
    ) -> Result<Self, AppError> {
        let tokio_rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .thread_name("model-hub-proxy")
            .build()
            .map_err(|e| AppError::ProxyStart(format!("创建 tokio runtime 失败: {e}")))?;
        Ok(Self {
            inner: Mutex::new(RuntimeInner {
                host: DEFAULT_HOST.to_string(),
                port: if port == 0 { DEFAULT_PORT } else { port },
                data_dir,
                config_dir,
                state: ProxyState::Idle,
                last_error: None,
                port_note: None,
                live: None,
                stores: None,
                clients: UpstreamClients::new(),
            }),
            tokio_rt,
        })
    }

    fn with_inner<T>(
        &self,
        f: impl FnOnce(&mut RuntimeInner) -> Result<T, AppError>,
    ) -> Result<T, AppError> {
        let mut guard = self.inner.lock().map_err(|_| AppError::LockPoisoned)?;
        f(&mut guard)
    }

    pub fn ensure_stores(&self) -> Result<Stores, AppError> {
        self.with_inner(|inner| {
            if let Some(s) = &inner.stores {
                return Ok(s.clone());
            }
            let path = default_db_path(&inner.data_dir);
            let db = open_db(&path)?;
            let stores = Stores::new(db);
            // 启动打开库时清理过期请求日志，控制体积。
            stores.purge_expired_logs_best_effort();
            inner.stores = Some(stores.clone());
            Ok(stores)
        })
    }

    pub fn status_snapshot(&self) -> Result<ProxyStatus, AppError> {
        self.with_inner(|inner| Ok(status_of(inner)))
    }

    pub fn start(&self) -> Result<ProxyStatus, AppError> {
        let stores = self.ensure_stores()?;

        // 已在运行则直接返回
        {
            let status = self.status_snapshot()?;
            if status.state == ProxyState::Running {
                return Ok(status);
            }
            if matches!(status.state, ProxyState::Starting | ProxyState::Stopping) {
                return Err(AppError::PortChangeWhileActive);
            }
        }

        let (clients, host, preferred, config_dir) = self.with_inner(|inner| {
            inner.state = ProxyState::Starting;
            inner.last_error = None;
            inner.port_note = None;
            Ok((
                inner.clients.clone(),
                inner.host.clone(),
                inner.port,
                inner.config_dir.clone(),
            ))
        })?;

        let preferred = if preferred == 0 {
            DEFAULT_PORT
        } else {
            preferred
        };
        let bind =
            match self
                .tokio_rt
                .block_on(bind_first_available(&host, preferred, PORT_SCAN_ATTEMPTS))
            {
                Ok(b) => b,
                Err(err) => {
                    let msg = err.to_string();
                    let _ = self.with_inner(|inner| {
                        inner.state = ProxyState::Error;
                        inner.last_error = Some(msg);
                        inner.port_note = None;
                        Ok(())
                    });
                    return Err(err);
                }
            };

        let chosen = bind.port;
        let port_note = if chosen != preferred {
            Some(format!(
                "端口 {preferred} 已被占用，已自动改用 {chosen} 并写入配置。不会结束占用进程；若意外多开旧实例，请托盘「退出」旧进程；若已导出 Pi，请在分组页重新「配置到 Pi」。"
            ))
        } else {
            None
        };

        if chosen != preferred {
            if let Some(dir) = config_dir.as_ref() {
                if let Err(e) = persist_gateway_port(dir, chosen) {
                    tracing::warn!(error = %e, port = chosen, "自动改口后写入 shell.json 失败");
                }
            }
        }

        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        let state = AppState {
            stores,
            clients,
            forward_policy: ForwardPolicy::default(),
        };

        let join = self.tokio_rt.spawn(async move {
            server::serve(bind.listener, state, shutdown_rx).await;
        });

        self.with_inner(|inner| {
            inner.port = chosen;
            inner.live = Some(LiveProxy { shutdown_tx, join });
            inner.state = ProxyState::Running;
            inner.last_error = None;
            inner.port_note = port_note;
            Ok(status_of(inner))
        })
    }

    pub fn stop(&self) -> Result<ProxyStatus, AppError> {
        let live = self.with_inner(|inner| {
            if inner.live.is_none() {
                inner.state = ProxyState::Idle;
                return Ok(None);
            }
            inner.state = ProxyState::Stopping;
            Ok(inner.live.take())
        })?;

        if let Some(live) = live {
            let _ = live.shutdown_tx.send(());
            let mut join = live.join;
            // 超时必须 abort：JoinHandle drop 只会 detach，任务可能继续占端口。
            self.tokio_rt.block_on(async {
                tokio::select! {
                    r = &mut join => {
                        if let Err(e) = r {
                            if !e.is_cancelled() {
                                tracing::warn!(error = %e, "代理服务任务结束时出错");
                            }
                        }
                    }
                    _ = tokio::time::sleep(PROXY_STOP_GRACE) => {
                        tracing::warn!(
                            grace_ms = PROXY_STOP_GRACE.as_millis(),
                            "代理 graceful stop 超时，abort 服务任务以释放端口"
                        );
                        join.abort();
                        let _ = join.await;
                    }
                }
            });
        }

        self.with_inner(|inner| {
            inner.state = ProxyState::Idle;
            inner.last_error = None;
            // 停止时清空 port_note。
            inner.port_note = None;
            Ok(status_of(inner))
        })
    }

    pub fn set_port(
        &self,
        config_dir: &std::path::Path,
        port: u16,
    ) -> Result<ProxyStatus, AppError> {
        if port == 0 {
            return Err(AppError::InvalidPort);
        }
        let mut cfg = settings::load_shell_config(config_dir)?;
        cfg.gateway_port = port;
        settings::save_shell_config(config_dir, &cfg)?;

        let was_running = self.with_inner(|inner| {
            if matches!(inner.state, ProxyState::Starting | ProxyState::Stopping) {
                return Err(AppError::PortChangeWhileActive);
            }
            let running = matches!(inner.state, ProxyState::Running);
            inner.port = port;
            inner.config_dir = Some(config_dir.to_path_buf());
            inner.port_note = None;
            Ok(running)
        })?;

        if was_running {
            let _ = self.stop();
            return self.start();
        }
        self.status_snapshot()
    }
}

impl Drop for ProxyHandle {
    fn drop(&mut self) {
        // best-effort：析构时仍有 live 则 stop，释放监听端口。
        let has_live = self.inner.lock().map(|g| g.live.is_some()).unwrap_or(false);
        if has_live {
            let _ = self.stop();
        }
    }
}

struct BoundListener {
    port: u16,
    listener: tokio::net::TcpListener,
}

fn next_port_candidate(preferred: u16, offset: u16) -> Option<u16> {
    if preferred == 0 {
        return None;
    }
    preferred.checked_add(offset).filter(|p| *p != 0)
}

fn persist_gateway_port(config_dir: &std::path::Path, port: u16) -> Result<(), AppError> {
    let mut cfg = settings::load_shell_config(config_dir)?;
    cfg.gateway_port = port;
    settings::save_shell_config(config_dir, &cfg)
}

async fn bind_first_available(
    host: &str,
    preferred: u16,
    attempts: u16,
) -> Result<BoundListener, AppError> {
    let attempts = attempts.max(1);
    let mut last_tried = preferred;
    let mut last_non_busy: Option<String> = None;

    for offset in 0..attempts {
        let Some(port) = next_port_candidate(preferred, offset) else {
            break;
        };
        last_tried = port;
        let addr: SocketAddr = format!("{host}:{port}")
            .parse()
            .map_err(|e| AppError::ProxyStart(format!("非法地址: {e}")))?;
        match tokio::net::TcpListener::bind(addr).await {
            Ok(listener) => {
                return Ok(BoundListener { port, listener });
            }
            Err(e) if e.kind() == std::io::ErrorKind::AddrInUse => {
                tracing::info!(port, "监听端口占用，尝试下一端口");
                continue;
            }
            Err(e) => {
                // 权限/其它错误：继续扫，但记住最后一条非占用错误
                last_non_busy = Some(format!("绑定 {addr} 失败: {e}"));
                continue;
            }
        }
    }

    if let Some(msg) = last_non_busy {
        // 若全程都是非占用错误，更贴近真实原因
        return Err(AppError::ProxyStart(msg));
    }
    Err(AppError::NoAvailablePort {
        preferred,
        attempts,
        last_tried,
    })
}

fn status_of(inner: &RuntimeInner) -> ProxyStatus {
    ProxyStatus {
        state: inner.state.clone(),
        host: inner.host.clone(),
        port: inner.port,
        last_error: inner.last_error.clone(),
        base_url: format!("http://{}:{}", inner.host, inner.port),
        data_dir: inner.data_dir.clone(),
        port_note: inner.port_note.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener as StdTcpListener;
    use tempfile::tempdir;

    #[test]
    fn next_port_candidate_increments() {
        assert_eq!(next_port_candidate(8084, 0), Some(8084));
        assert_eq!(next_port_candidate(8084, 1), Some(8085));
        assert_eq!(next_port_candidate(u16::MAX, 1), None);
    }

    #[test]
    fn start_skips_busy_port_and_persists() {
        let dir = tempdir().unwrap();
        let config_dir = dir.path().join("config");
        let data_dir = dir.path().join("gateway");
        std::fs::create_dir_all(&config_dir).unwrap();
        std::fs::create_dir_all(&data_dir).unwrap();

        // 占用一个临时端口，让代理从该端口起扫
        let blocker = StdTcpListener::bind("127.0.0.1:0").unwrap();
        let preferred = blocker.local_addr().unwrap().port();

        settings::save_shell_config(
            &config_dir,
            &settings::ShellConfig {
                gateway_port: preferred,
                ..Default::default()
            },
        )
        .unwrap();

        let proxy = ProxyHandle::new_with_config_dir(
            data_dir.display().to_string(),
            preferred,
            Some(config_dir.clone()),
        )
        .unwrap();

        let status = proxy.start().expect("应自动改口启动");
        assert_eq!(status.state, ProxyState::Running);
        assert_ne!(status.port, preferred);
        assert!(status.port_note.as_ref().unwrap().contains("自动改用"));

        let cfg = settings::load_shell_config(&config_dir).unwrap();
        assert_eq!(cfg.gateway_port, status.port);

        let _ = proxy.stop();
        drop(blocker);
    }

    #[test]
    fn stop_sets_idle_and_releases_port() {
        let dir = tempdir().unwrap();
        let data_dir = dir.path().join("gateway");
        std::fs::create_dir_all(&data_dir).unwrap();

        let proxy = ProxyHandle::new(data_dir.display().to_string(), 0).unwrap();
        let status = proxy.start().expect("应启动");
        assert_eq!(status.state, ProxyState::Running);
        let port = status.port;

        let stopped = proxy.stop().expect("应停止");
        assert_eq!(stopped.state, ProxyState::Idle);

        // 停止后原端口应可重新 bind
        let rebind = StdTcpListener::bind(format!("127.0.0.1:{port}"));
        assert!(rebind.is_ok(), "stop 后端口应可重新绑定");
    }

    #[test]
    fn drop_stops_live_proxy() {
        let dir = tempdir().unwrap();
        let data_dir = dir.path().join("gateway");
        std::fs::create_dir_all(&data_dir).unwrap();

        let port = {
            let proxy = ProxyHandle::new(data_dir.display().to_string(), 0).unwrap();
            let status = proxy.start().expect("应启动");
            assert_eq!(status.state, ProxyState::Running);
            let port = status.port;
            drop(proxy);
            port
        };

        let rebind = StdTcpListener::bind(format!("127.0.0.1:{port}"));
        assert!(rebind.is_ok(), "Drop 后端口应可重新绑定");
    }
}
