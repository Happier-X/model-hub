//! 代理生命周期：start/stop/status/set_port。

use std::net::SocketAddr;
use std::sync::Mutex;

use serde::Serialize;
use tokio::sync::oneshot;

use crate::db::{default_db_path, open_db};
use crate::domain::Stores;
use crate::error::AppError;
use crate::proxy::circuit::CircuitRegistry;
use crate::proxy::forward::{ForwardPolicy, UpstreamClients};
use crate::proxy::server::{self, AppState};
use crate::settings::{self, ShellConfig, DEFAULT_PORT};

pub const DEFAULT_HOST: &str = "127.0.0.1";

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
}

struct LiveProxy {
    shutdown_tx: oneshot::Sender<()>,
    join: tokio::task::JoinHandle<()>,
}

struct RuntimeInner {
    host: String,
    port: u16,
    data_dir: String,
    state: ProxyState,
    last_error: Option<String>,
    live: Option<LiveProxy>,
    stores: Option<Stores>,
    circuits: CircuitRegistry,
    clients: UpstreamClients,
}

pub struct ProxyHandle {
    inner: Mutex<RuntimeInner>,
    /// 独立 tokio runtime，承载 axum
    tokio_rt: tokio::runtime::Runtime,
}

impl ProxyHandle {
    pub fn new(data_dir: String, port: u16) -> Result<Self, AppError> {
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
                state: ProxyState::Idle,
                last_error: None,
                live: None,
                stores: None,
                circuits: CircuitRegistry::new(),
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
            inner.stores = Some(stores.clone());
            Ok(stores)
        })
    }

    pub fn circuits(&self) -> Result<CircuitRegistry, AppError> {
        self.with_inner(|inner| Ok(inner.circuits.clone()))
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

        let (circuits, clients, host, port) = self.with_inner(|inner| {
            inner.state = ProxyState::Starting;
            inner.last_error = None;
            Ok((
                inner.circuits.clone(),
                inner.clients.clone(),
                inner.host.clone(),
                inner.port,
            ))
        })?;

        let addr: SocketAddr = format!("{host}:{port}")
            .parse()
            .map_err(|e| AppError::ProxyStart(format!("非法地址: {e}")))?;

        let bind_result = self
            .tokio_rt
            .block_on(async { tokio::net::TcpListener::bind(addr).await });

        let listener = match bind_result {
            Ok(l) => l,
            Err(e) => {
                let _ = self.with_inner(|inner| {
                    inner.state = ProxyState::Error;
                    inner.last_error = Some(e.to_string());
                    Ok(())
                });
                return Err(if e.kind() == std::io::ErrorKind::AddrInUse {
                    AppError::PortInUse { port }
                } else {
                    AppError::ProxyStart(format!("绑定 {addr} 失败: {e}"))
                });
            }
        };

        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        let state = AppState {
            stores,
            circuits,
            clients,
            forward_policy: ForwardPolicy::default(),
        };

        let join = self.tokio_rt.spawn(async move {
            server::serve(listener, state, shutdown_rx).await;
        });

        self.with_inner(|inner| {
            inner.live = Some(LiveProxy { shutdown_tx, join });
            inner.state = ProxyState::Running;
            inner.last_error = None;
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
            let _ = self.tokio_rt.block_on(async {
                tokio::time::timeout(std::time::Duration::from_secs(3), live.join).await
            });
        }

        self.with_inner(|inner| {
            inner.state = ProxyState::Idle;
            inner.last_error = None;
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
        settings::save_shell_config(config_dir, &ShellConfig { gateway_port: port })?;

        let was_running = self.with_inner(|inner| {
            if matches!(inner.state, ProxyState::Starting | ProxyState::Stopping) {
                return Err(AppError::PortChangeWhileActive);
            }
            let running = matches!(inner.state, ProxyState::Running);
            inner.port = port;
            Ok(running)
        })?;

        if was_running {
            let _ = self.stop();
            return self.start();
        }
        self.status_snapshot()
    }
}

fn status_of(inner: &RuntimeInner) -> ProxyStatus {
    ProxyStatus {
        state: inner.state.clone(),
        host: inner.host.clone(),
        port: inner.port,
        last_error: inner.last_error.clone(),
        base_url: format!("http://{}:{}", inner.host, inner.port),
        data_dir: inner.data_dir.clone(),
    }
}
