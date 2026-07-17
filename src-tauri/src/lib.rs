mod error;
mod gateway;
mod paths;

use tauri::Manager;

use crate::gateway::GatewayHandle;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let app_paths = paths::resolve_paths(app.handle()).map_err(|error| {
                let message = error.to_string();
                Box::<dyn std::error::Error>::from(message)
            })?;

            app.manage(GatewayHandle::new(app_paths.gateway_dir.clone()));

            // 自动尝试启动侧车：二进制缺失时保持 error/idle 语义，不阻止窗口打开。
            gateway::try_autostart(app.handle());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            paths::get_paths,
            gateway::gateway_start,
            gateway::gateway_stop,
            gateway::gateway_status,
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
