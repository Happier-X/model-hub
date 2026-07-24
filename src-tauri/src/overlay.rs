//! 桌面悬浮状态条窗口：动态创建、显隐与默认定位。

use tauri::{
    AppHandle, Manager, PhysicalPosition, Position, WebviewUrl, WebviewWindow, WebviewWindowBuilder,
};

use crate::{error::AppError, paths, settings};

pub const OVERLAY_LABEL: &str = "overlay";
const OVERLAY_WIDTH: f64 = 420.0;
const OVERLAY_HEIGHT: f64 = 68.0;
const EDGE_MARGIN: i32 = 16;

fn create_overlay(app: &AppHandle) -> Result<WebviewWindow, AppError> {
    WebviewWindowBuilder::new(
        app,
        OVERLAY_LABEL,
        WebviewUrl::App("index.html?overlay=1".into()),
    )
    .title("Model Hub 当前模型")
    .inner_size(OVERLAY_WIDTH, OVERLAY_HEIGHT)
    .min_inner_size(OVERLAY_WIDTH, OVERLAY_HEIGHT)
    .max_inner_size(OVERLAY_WIDTH, OVERLAY_HEIGHT)
    .resizable(false)
    .decorations(false)
    .shadow(false)
    .always_on_top(true)
    .skip_taskbar(true)
    .focused(false)
    .focusable(false)
    .visible(false)
    .build()
    .map_err(AppError::from)
}

fn clamp_to_primary_work_area(
    app: &AppHandle,
    position: PhysicalPosition<i32>,
) -> Result<PhysicalPosition<i32>, AppError> {
    let monitor = app
        .primary_monitor()
        .map_err(AppError::from)?
        .ok_or_else(|| AppError::Business("无法获取主显示器信息".into()))?;
    // work_area 是物理像素，窗口尺寸是逻辑像素，须按 DPI 缩放后再计算边界。
    let work_area = monitor.work_area();
    let scale = monitor.scale_factor();
    let width = (OVERLAY_WIDTH * scale).round() as i32;
    let height = (OVERLAY_HEIGHT * scale).round() as i32;
    let min_x = work_area.position.x;
    let min_y = work_area.position.y;
    let max_x = min_x + work_area.size.width as i32 - width;
    let max_y = min_y + work_area.size.height as i32 - height;

    Ok(PhysicalPosition::new(
        position.x.clamp(min_x, max_x.max(min_x)),
        position.y.clamp(min_y, max_y.max(min_y)),
    ))
}

fn default_position(app: &AppHandle) -> Result<PhysicalPosition<i32>, AppError> {
    let monitor = app
        .primary_monitor()
        .map_err(AppError::from)?
        .ok_or_else(|| AppError::Business("无法获取主显示器信息".into()))?;
    let work_area = monitor.work_area();
    let scale = monitor.scale_factor();
    let width = (OVERLAY_WIDTH * scale).round() as i32;
    let height = (OVERLAY_HEIGHT * scale).round() as i32;

    Ok(PhysicalPosition::new(
        work_area.position.x + work_area.size.width as i32 - width - EDGE_MARGIN,
        work_area.position.y + work_area.size.height as i32 - height - EDGE_MARGIN,
    ))
}

fn configured_position(app: &AppHandle) -> Result<Option<PhysicalPosition<i32>>, AppError> {
    let app_paths = paths::resolve_paths(app)?;
    let config = settings::load_shell_config(std::path::Path::new(&app_paths.config_dir))?;
    Ok(match (config.overlay_x, config.overlay_y) {
        (Some(x), Some(y)) => Some(PhysicalPosition::new(x, y)),
        _ => None,
    })
}

pub fn ensure_overlay(app: &AppHandle) -> Result<WebviewWindow, AppError> {
    if let Some(window) = app.get_webview_window(OVERLAY_LABEL) {
        return Ok(window);
    }

    let window = create_overlay(app)?;
    let position = configured_position(app)?.unwrap_or(default_position(app)?);
    let position = clamp_to_primary_work_area(app, position)?;
    window
        .set_position(Position::Physical(position))
        .map_err(AppError::from)?;
    Ok(window)
}

pub fn set_overlay_visible(app: &AppHandle, visible: bool) -> Result<(), AppError> {
    if visible {
        let window = ensure_overlay(app)?;
        // 显示前重新校正位置：避免 DPI / 任务栏工作区变化后恢复到屏幕外。
        if let Ok(current) = window.outer_position() {
            let clamped =
                clamp_to_primary_work_area(app, PhysicalPosition::new(current.x, current.y))?;
            window
                .set_position(Position::Physical(clamped))
                .map_err(AppError::from)?;
        }
        window.show().map_err(AppError::from)?;
        window.set_always_on_top(true).map_err(AppError::from)?;
    } else if let Some(window) = app.get_webview_window(OVERLAY_LABEL) {
        window.hide().map_err(AppError::from)?;
    }
    Ok(())
}

pub fn restore_overlay_on_start(app: &AppHandle) -> Result<(), AppError> {
    let app_paths = paths::resolve_paths(app)?;
    let config = settings::load_shell_config(std::path::Path::new(&app_paths.config_dir))?;
    if config.overlay_enabled {
        set_overlay_visible(app, true)?;
    }
    Ok(())
}

pub fn save_overlay_position(app: &AppHandle, x: i32, y: i32) -> Result<(), AppError> {
    let app_paths = paths::resolve_paths(app)?;
    let config_dir = std::path::Path::new(&app_paths.config_dir);
    let mut config = settings::load_shell_config(config_dir)?;
    // 前端上报的坐标可能超出主屏工作区，落库前先 clamp，防止重启后恢复到不可见位置。
    let clamped = clamp_to_primary_work_area(app, PhysicalPosition::new(x, y))?;
    config.overlay_x = Some(clamped.x);
    config.overlay_y = Some(clamped.y);
    settings::save_shell_config(config_dir, &config)
}
