mod error;
mod paths;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            paths::resolve_paths(app.handle()).map_err(|error| {
                let message = error.to_string();
                Box::<dyn std::error::Error>::from(message)
            })?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![paths::get_paths])
        .run(tauri::generate_context!())
        .expect("运行 Model Hub 桌面应用失败");
}
