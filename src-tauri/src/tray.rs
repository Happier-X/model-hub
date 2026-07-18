//! 系统托盘：显示主窗口、启停网关、真正退出。

use std::sync::atomic::{AtomicBool, Ordering};

use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager,
};

use crate::gateway::{self, GatewayHandle};

/// 标记应用是否进入「真正退出」路径（相对：关窗隐藏到托盘）。
#[derive(Default)]
pub struct AppExitState {
    exiting: AtomicBool,
}

impl AppExitState {
    pub fn new() -> Self {
        Self {
            exiting: AtomicBool::new(false),
        }
    }

    pub fn is_exiting(&self) -> bool {
        self.exiting.load(Ordering::SeqCst)
    }

    pub fn mark_exiting(&self) {
        self.exiting.store(true, Ordering::SeqCst);
    }
}

const MENU_SHOW: &str = "tray-show";
const MENU_GATEWAY_START: &str = "tray-gateway-start";
const MENU_GATEWAY_STOP: &str = "tray-gateway-stop";
const MENU_QUIT: &str = "tray-quit";

pub fn setup_tray(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let handle = app.handle();

    let show = MenuItem::with_id(handle, MENU_SHOW, "显示 Model Hub", true, None::<&str>)?;
    let start = MenuItem::with_id(handle, MENU_GATEWAY_START, "启动网关", true, None::<&str>)?;
    let stop = MenuItem::with_id(handle, MENU_GATEWAY_STOP, "停止网关", true, None::<&str>)?;
    let quit = MenuItem::with_id(handle, MENU_QUIT, "退出", true, None::<&str>)?;

    let menu = Menu::with_items(handle, &[&show, &start, &stop, &quit])?;

    let icon = handle
        .default_window_icon()
        .cloned()
        .ok_or("缺少默认窗口图标，请在 tauri.conf.json 配置 icons/icon.ico")?;

    let _tray = TrayIconBuilder::with_id("model-hub-tray")
        .icon(icon)
        .tooltip("Model Hub — 关闭窗口将隐藏到托盘；托盘「退出」才停止网关")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            MENU_SHOW => {
                show_main_window(app);
            }
            MENU_GATEWAY_START => {
                gateway::start_managed(app);
            }
            MENU_GATEWAY_STOP => {
                if let Some(gateway) = app.try_state::<GatewayHandle>() {
                    gateway::stop_managed(gateway.inner());
                }
            }
            MENU_QUIT => {
                request_quit(app);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main_window(tray.app_handle());
            }
        })
        .build(handle)?;

    Ok(())
}

pub fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

pub fn request_quit(app: &AppHandle) {
    if let Some(state) = app.try_state::<AppExitState>() {
        state.mark_exiting();
    }
    // 先停网关，再 exit；RunEvent::Exit 也会幂等 stop
    if let Some(gateway) = app.try_state::<GatewayHandle>() {
        gateway::stop_managed(gateway.inner());
    }
    app.exit(0);
}

#[cfg(test)]
mod tests {
    use super::AppExitState;

    #[test]
    fn exit_state_starts_false_and_marks_true() {
        let state = AppExitState::new();
        assert!(!state.is_exiting());
        state.mark_exiting();
        assert!(state.is_exiting());
    }
}
