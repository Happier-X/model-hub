mod commands;
pub mod db;
pub mod domain;
mod error;
mod paths;
mod pi_export;
pub mod proxy;
mod settings;
mod tray;

use tauri::{Manager, WindowEvent};

use crate::proxy::ProxyHandle;
use crate::tray::AppExitState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .try_init();

    tauri::Builder::default()
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            let app_paths = paths::resolve_paths(app.handle()).map_err(|error| {
                Box::<dyn std::error::Error>::from(error.to_string())
            })?;

            let port = settings::load_shell_config(std::path::Path::new(&app_paths.config_dir))
                .map(|c| c.gateway_port)
                .unwrap_or(settings::DEFAULT_PORT);

            let proxy = ProxyHandle::new(app_paths.gateway_dir.clone(), port)
                .map_err(|e| Box::<dyn std::error::Error>::from(e.to_string()))?;

            // 打开应用即尝试启动内嵌代理；失败写入 last_error，不阻止窗口。
            if let Err(err) = proxy.start() {
                tracing::warn!(error = %err, "自动启动代理失败");
            }

            app.manage(proxy);
            app.manage(AppExitState::new());
            tray::setup_tray(app)?;
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
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            paths::get_paths,
            commands::proxy_start,
            commands::proxy_stop,
            commands::proxy_status,
            commands::proxy_set_port,
            commands::get_shell_prefs,
            commands::set_check_update_on_startup,
            commands::list_providers,
            commands::create_provider,
            commands::update_provider,
            commands::delete_provider,
            commands::fetch_provider_models,
            commands::list_groups,
            commands::create_group,
            commands::update_group,
            commands::delete_group,
            commands::list_api_keys,
            commands::create_api_key,
            commands::update_api_key,
            commands::delete_api_key,
            commands::list_logs,
            commands::clear_logs,
            commands::get_request_stats,
            commands::export_to_pi_agent,
            commands::list_health,
        ])
        .build(tauri::generate_context!())
        .expect("构建 Model Hub 桌面应用失败")
        .run(|app_handle, event| {
            if let tauri::RunEvent::Exit = event {
                if let Some(proxy) = app_handle.try_state::<ProxyHandle>() {
                    let _ = proxy.stop();
                }
            }
        });
}
