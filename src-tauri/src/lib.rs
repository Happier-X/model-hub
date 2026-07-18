mod error;
mod gateway;
mod paths;
mod tray;

use tauri::{Manager, WindowEvent};

use crate::gateway::GatewayHandle;
use crate::tray::AppExitState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let app_paths = paths::resolve_paths(app.handle()).map_err(|error| {
                let message = error.to_string();
                Box::<dyn std::error::Error>::from(message)
            })?;

            let gateway_port =
                gateway::load_configured_port(std::path::Path::new(&app_paths.config_dir))
                    .map_err(|error| {
                        let message = error.to_string();
                        Box::<dyn std::error::Error>::from(message)
                    })?;
            app.manage(GatewayHandle::new(
                app_paths.gateway_dir.clone(),
                gateway_port,
            ));
            app.manage(AppExitState::new());

            tray::setup_tray(app)?;

            // 自动尝试启动侧车：二进制缺失时保持 error/idle 语义，不阻止窗口打开。
            gateway::try_autostart(app.handle());
            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() != "main" {
                return;
            }
            if let WindowEvent::CloseRequested { api, .. } = event {
                let exiting = window
                    .app_handle()
                    .try_state::<AppExitState>()
                    .map(|s| s.is_exiting())
                    .unwrap_or(false);
                if !exiting {
                    // 关窗 → 隐藏到托盘，不停止网关
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            paths::get_paths,
            gateway::gateway_start,
            gateway::gateway_stop,
            gateway::gateway_status,
            gateway::gateway_set_port,
        ])
        .build(tauri::generate_context!())
        .expect("构建 Model Hub 桌面应用失败")
        .run(|app_handle, event| {
            if let tauri::RunEvent::Exit = event {
                if let Some(gateway) = app_handle.try_state::<GatewayHandle>() {
                    gateway::stop_managed(gateway.inner());
                }
            }
        });
}
