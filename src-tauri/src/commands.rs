//! Tauri IPC 命令：代理启停 + 领域 CRUD。

use serde::Serialize;
use tauri::{AppHandle, State};

use crate::domain::group::{CreateGroupPayload, Group, UpdateGroupPayload};
use crate::domain::leaderboard::ModelLeaderboardSnapshot;
use crate::domain::provider::{CreateProviderPayload, Provider, UpdateProviderPayload};
use crate::domain::upstream_models::{fetch_upstream_model_ids, FetchProviderModelsPayload};
use crate::error::{AppError, InvokeError};
use crate::paths;
use crate::proxy::{ProxyHandle, ProxyStatus};

fn stores(proxy: &ProxyHandle) -> Result<crate::domain::Stores, InvokeError> {
    proxy.ensure_stores().map_err(Into::into)
}

#[tauri::command]
pub fn proxy_status(proxy: State<'_, ProxyHandle>) -> Result<ProxyStatus, InvokeError> {
    proxy.status_snapshot().map_err(Into::into)
}

#[tauri::command]
pub fn proxy_start(proxy: State<'_, ProxyHandle>) -> Result<ProxyStatus, InvokeError> {
    proxy.start().map_err(Into::into)
}

#[tauri::command]
pub fn proxy_stop(proxy: State<'_, ProxyHandle>) -> Result<ProxyStatus, InvokeError> {
    proxy.stop().map_err(Into::into)
}

#[tauri::command]
pub fn proxy_set_port(
    app: AppHandle,
    proxy: State<'_, ProxyHandle>,
    port: u32,
) -> Result<ProxyStatus, InvokeError> {
    let port = u16::try_from(port).map_err(|_| crate::error::AppError::InvalidPort)?;
    let paths = paths::resolve_paths(&app).map_err(InvokeError::from)?;
    proxy
        .set_port(std::path::Path::new(&paths.config_dir), port)
        .map_err(Into::into)
}

#[derive(Debug, Serialize)]
pub struct ShellPrefs {
    pub gateway_port: u16,
    pub check_update_on_startup: bool,
    pub overlay_enabled: bool,
}

fn shell_prefs(config: &crate::settings::ShellConfig) -> ShellPrefs {
    ShellPrefs {
        gateway_port: config.gateway_port,
        check_update_on_startup: config.check_update_on_startup,
        overlay_enabled: config.overlay_enabled,
    }
}

#[tauri::command]
pub fn get_shell_prefs(app: AppHandle) -> Result<ShellPrefs, InvokeError> {
    let paths = paths::resolve_paths(&app).map_err(InvokeError::from)?;
    let cfg = crate::settings::load_shell_config(std::path::Path::new(&paths.config_dir))
        .map_err(InvokeError::from)?;
    Ok(shell_prefs(&cfg))
}

#[tauri::command]
pub fn set_check_update_on_startup(
    app: AppHandle,
    enabled: bool,
) -> Result<ShellPrefs, InvokeError> {
    let paths = paths::resolve_paths(&app).map_err(InvokeError::from)?;
    let config_dir = std::path::Path::new(&paths.config_dir);
    let mut cfg = crate::settings::load_shell_config(config_dir).map_err(InvokeError::from)?;
    cfg.check_update_on_startup = enabled;
    crate::settings::save_shell_config(config_dir, &cfg).map_err(InvokeError::from)?;
    Ok(shell_prefs(&cfg))
}

#[tauri::command]
pub fn set_overlay_enabled(app: AppHandle, enabled: bool) -> Result<ShellPrefs, InvokeError> {
    let paths = paths::resolve_paths(&app).map_err(InvokeError::from)?;
    let config_dir = std::path::Path::new(&paths.config_dir);
    let mut cfg = crate::settings::load_shell_config(config_dir).map_err(InvokeError::from)?;
    let previous = cfg.overlay_enabled;

    cfg.overlay_enabled = enabled;
    crate::settings::save_shell_config(config_dir, &cfg).map_err(InvokeError::from)?;

    if let Err(error) = crate::overlay::set_overlay_visible(&app, enabled) {
        cfg.overlay_enabled = previous;
        if let Err(rollback_error) = crate::settings::save_shell_config(config_dir, &cfg) {
            tracing::warn!(error = %rollback_error, "回滚悬浮状态条开关失败");
        }
        return Err(InvokeError::from(error));
    }

    Ok(shell_prefs(&cfg))
}

#[tauri::command]
pub fn save_overlay_position(app: AppHandle, x: i32, y: i32) -> Result<(), InvokeError> {
    crate::overlay::save_overlay_position(&app, x, y).map_err(Into::into)
}

#[tauri::command]
pub fn show_main_window(app: AppHandle) {
    crate::tray::show_main_window(&app);
}

#[tauri::command]
pub fn list_providers(proxy: State<'_, ProxyHandle>) -> Result<Vec<Provider>, InvokeError> {
    stores(&proxy)?.list_providers().map_err(Into::into)
}

#[tauri::command]
pub fn create_provider(
    proxy: State<'_, ProxyHandle>,
    payload: CreateProviderPayload,
) -> Result<Provider, InvokeError> {
    stores(&proxy)?.create_provider(payload).map_err(Into::into)
}

#[tauri::command]
pub fn update_provider(
    proxy: State<'_, ProxyHandle>,
    payload: UpdateProviderPayload,
) -> Result<Provider, InvokeError> {
    stores(&proxy)?.update_provider(payload).map_err(Into::into)
}

#[tauri::command]
pub fn delete_provider(proxy: State<'_, ProxyHandle>, id: i64) -> Result<(), InvokeError> {
    stores(&proxy)?.delete_provider(id).map_err(Into::into)
}

/// 从上游供应商 OpenAI 兼容 `/models` 拉取模型 id 列表。
///
/// 支持已保存 `provider_id`，或表单草稿 `base_url` + `api_key`。
#[tauri::command]
pub async fn fetch_provider_models(
    proxy: State<'_, ProxyHandle>,
    payload: FetchProviderModelsPayload,
) -> Result<Vec<String>, InvokeError> {
    let (base_url, api_key) = if let Some(id) = payload.provider_id {
        let p = stores(&proxy)?
            .get_provider(id)
            .map_err(InvokeError::from)?
            .ok_or_else(|| InvokeError::from(AppError::Business("供应商不存在".into())))?;
        (p.base_url, p.api_key)
    } else {
        let base_url = payload
            .base_url
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| {
                InvokeError::from(AppError::Business(
                    "请提供 provider_id，或同时提供 base_url 与 api_key".into(),
                ))
            })?
            .to_string();
        let api_key = payload
            .api_key
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| {
                InvokeError::from(AppError::Business(
                    "请提供 provider_id，或同时提供 base_url 与 api_key".into(),
                ))
            })?
            .to_string();
        (base_url, api_key)
    };

    fetch_upstream_model_ids(&base_url, &api_key)
        .await
        .map_err(Into::into)
}

#[tauri::command]
pub fn list_groups(proxy: State<'_, ProxyHandle>) -> Result<Vec<Group>, InvokeError> {
    stores(&proxy)?.list_groups().map_err(Into::into)
}

#[tauri::command]
pub fn create_group(
    proxy: State<'_, ProxyHandle>,
    payload: CreateGroupPayload,
) -> Result<Group, InvokeError> {
    stores(&proxy)?.create_group(payload).map_err(Into::into)
}

#[tauri::command]
pub fn update_group(
    proxy: State<'_, ProxyHandle>,
    payload: UpdateGroupPayload,
) -> Result<Group, InvokeError> {
    stores(&proxy)?.update_group(payload).map_err(Into::into)
}

#[tauri::command]
pub fn delete_group(proxy: State<'_, ProxyHandle>, id: i64) -> Result<(), InvokeError> {
    stores(&proxy)?.delete_group(id).map_err(Into::into)
}

#[tauri::command]
pub fn list_logs(
    proxy: State<'_, ProxyHandle>,
    query: Option<crate::domain::log::LogQuery>,
) -> Result<crate::domain::log::LogPage, InvokeError> {
    stores(&proxy)?
        .list_logs(query.unwrap_or_default())
        .map_err(Into::into)
}

#[tauri::command]
pub fn clear_logs(proxy: State<'_, ProxyHandle>) -> Result<(), InvokeError> {
    stores(&proxy)?.clear_logs().map_err(Into::into)
}

#[tauri::command]
pub fn purge_expired_logs(
    proxy: State<'_, ProxyHandle>,
) -> Result<crate::domain::log::LogPurgeResult, InvokeError> {
    stores(&proxy)?.purge_expired_logs().map_err(Into::into)
}

#[tauri::command]
pub fn get_request_stats(
    proxy: State<'_, ProxyHandle>,
) -> Result<crate::domain::log::RequestStats, InvokeError> {
    stores(&proxy)?.request_stats_today().map_err(Into::into)
}

#[tauri::command]
pub fn get_last_success_request(
    proxy: State<'_, ProxyHandle>,
) -> Result<Option<crate::domain::log::LastSuccessRequest>, InvokeError> {
    stores(&proxy)?.last_success_request().map_err(Into::into)
}

#[derive(Debug, Serialize)]
pub struct ExportToPiResult {
    pub path: String,
    pub provider_id: String,
    pub model_count: usize,
    pub base_url: String,
    pub group_name: String,
}

/// 将指定分组写入 `~/.pi/agent/models.json` 的 model-hub 供应商（按分组名 upsert，固定占位 Key）。
#[tauri::command]
pub fn export_group_to_pi_agent(
    proxy: State<'_, ProxyHandle>,
    group_id: i64,
) -> Result<ExportToPiResult, InvokeError> {
    let status = proxy.status_snapshot().map_err(InvokeError::from)?;
    let groups = stores(&proxy)?.list_groups().map_err(InvokeError::from)?;
    let group = groups
        .into_iter()
        .find(|g| g.id == group_id)
        .ok_or_else(|| {
            InvokeError::from(AppError::Business("分组不存在或已删除，刷新后重试".into()))
        })?;
    let path = crate::pi_export::default_pi_models_path().map_err(InvokeError::from)?;
    let model_count =
        crate::pi_export::upsert_model_hub_group(&path, &status.base_url, &group.name)
            .map_err(InvokeError::from)?;
    Ok(ExportToPiResult {
        path: path.display().to_string(),
        provider_id: crate::pi_export::PI_PROVIDER_ID.to_string(),
        model_count,
        base_url: crate::pi_export::normalize_openai_base_url(&status.base_url),
        group_name: group.name,
    })
}

/// 获取 OpenRouter 公共模型榜单（24h 缓存；可强制刷新；网络失败时 stale 回退）。
#[tauri::command]
pub async fn get_model_leaderboard(
    app: AppHandle,
    force_refresh: Option<bool>,
) -> Result<ModelLeaderboardSnapshot, InvokeError> {
    let paths = paths::resolve_paths(&app).map_err(InvokeError::from)?;
    let config_dir = std::path::Path::new(&paths.config_dir);
    crate::domain::leaderboard::get_model_leaderboard(config_dir, force_refresh.unwrap_or(false))
        .await
        .map_err(Into::into)
}
