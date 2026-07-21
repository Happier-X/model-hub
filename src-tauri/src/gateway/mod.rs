mod binary;
mod config;
mod health;
mod impl_kind;
mod process;
mod settings;
mod state;

use std::sync::Mutex;

use tauri::{AppHandle, Manager, State};

use crate::{
    error::{AppError, InvokeError},
    paths,
};

use self::{
    config::DEFAULT_HOST,
    process::GatewayRuntime,
    settings::{save_shell_config, ShellConfig},
    state::GatewayStatus,
};

pub struct GatewayHandle(pub Mutex<GatewayRuntime>);

pub fn load_configured_port(config_dir: &std::path::Path) -> Result<u16, AppError> {
    settings::load_shell_config(config_dir).map(|config| config.gateway_port)
}

impl GatewayHandle {
    pub fn new(data_dir: String, port: u16) -> Self {
        Self(Mutex::new(GatewayRuntime::new(
            DEFAULT_HOST.to_string(),
            port,
            data_dir,
        )))
    }
}

fn with_runtime<T>(
    handle: &GatewayHandle,
    f: impl FnOnce(&mut GatewayRuntime) -> Result<T, AppError>,
) -> Result<T, AppError> {
    let mut guard = handle.0.lock().map_err(|_| AppError::GatewayLockPoisoned)?;
    f(&mut guard)
}

#[tauri::command]
pub fn gateway_status(
    app: AppHandle,
    gateway: State<'_, GatewayHandle>,
) -> Result<GatewayStatus, InvokeError> {
    let paths = paths::resolve_paths(&app).map_err(InvokeError::from)?;
    with_runtime(&gateway, |runtime| {
        let mut status = runtime.status_snapshot();
        status.data_dir = paths.gateway_dir.clone();
        Ok(status)
    })
    .map_err(InvokeError::from)
}

#[tauri::command]
pub fn gateway_start(
    app: AppHandle,
    gateway: State<'_, GatewayHandle>,
) -> Result<GatewayStatus, InvokeError> {
    let paths = paths::resolve_paths(&app).map_err(InvokeError::from)?;
    let gateway_dir = std::path::PathBuf::from(&paths.gateway_dir);
    let bin_dir = std::path::PathBuf::from(&paths.bin_dir);
    let resource_dir = app.path().resource_dir().ok().filter(|p| p.exists());
    with_runtime(&gateway, |runtime| {
        runtime.start_with_resource(&gateway_dir, &bin_dir, resource_dir.as_deref())
    })
    .map_err(InvokeError::from)
}

#[tauri::command]
pub fn gateway_stop(gateway: State<'_, GatewayHandle>) -> Result<GatewayStatus, InvokeError> {
    with_runtime(&gateway, |runtime| runtime.stop()).map_err(InvokeError::from)
}

#[tauri::command]
pub fn gateway_set_port(
    app: AppHandle,
    gateway: State<'_, GatewayHandle>,
    port: u32,
) -> Result<GatewayStatus, InvokeError> {
    let port = u16::try_from(port).map_err(|_| AppError::InvalidPort)?;
    if port == 0 {
        return Err(AppError::InvalidPort.into());
    }
    let paths = paths::resolve_paths(&app).map_err(InvokeError::from)?;
    with_runtime(&gateway, |runtime| {
        // 先验证运行态，再持久化；保存失败时恢复原端口，避免内存与磁盘分裂。
        let previous = runtime.status_snapshot();
        let status = runtime.set_port(port)?;
        if let Err(error) = save_shell_config(
            std::path::Path::new(&paths.config_dir),
            &ShellConfig { gateway_port: port },
        ) {
            runtime.restore_status(previous);
            return Err(error);
        }
        Ok(status)
    })
    .map_err(InvokeError::from)
}

pub fn stop_managed(gateway: &GatewayHandle) {
    if let Ok(mut runtime) = gateway.0.lock() {
        let _ = runtime.stop();
    }
}

pub fn try_autostart(app: &AppHandle) {
    start_managed(app);
}

/// 托盘/自动启动共用：尝试启动托管侧车。
pub fn start_managed(app: &AppHandle) {
    let Some(gateway) = app.try_state::<GatewayHandle>() else {
        return;
    };
    let paths = match paths::resolve_paths(app) {
        Ok(paths) => paths,
        Err(_) => return,
    };
    let gateway_dir = std::path::PathBuf::from(&paths.gateway_dir);
    let bin_dir = std::path::PathBuf::from(&paths.bin_dir);
    let resource_dir = app.path().resource_dir().ok().filter(|p| p.exists());
    let _ = with_runtime(gateway.inner(), |runtime| {
        runtime.start_with_resource(&gateway_dir, &bin_dir, resource_dir.as_deref())
    });
}
