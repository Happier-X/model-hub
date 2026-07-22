//! Tauri IPC 命令：代理启停 + 领域 CRUD。

use serde::Serialize;
use tauri::{AppHandle, State};

use crate::domain::apikey::{CreateApiKeyPayload, UpdateApiKeyPayload};
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
}

#[tauri::command]
pub fn get_shell_prefs(app: AppHandle) -> Result<ShellPrefs, InvokeError> {
    let paths = paths::resolve_paths(&app).map_err(InvokeError::from)?;
    let cfg = crate::settings::load_shell_config(std::path::Path::new(&paths.config_dir))
        .map_err(InvokeError::from)?;
    Ok(ShellPrefs {
        gateway_port: cfg.gateway_port,
        check_update_on_startup: cfg.check_update_on_startup,
    })
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
    Ok(ShellPrefs {
        gateway_port: cfg.gateway_port,
        check_update_on_startup: cfg.check_update_on_startup,
    })
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
pub fn list_api_keys(
    proxy: State<'_, ProxyHandle>,
) -> Result<Vec<crate::domain::apikey::ApiKeyPublic>, InvokeError> {
    stores(&proxy)?.list_api_keys().map_err(Into::into)
}

#[tauri::command]
pub fn create_api_key(
    proxy: State<'_, ProxyHandle>,
    payload: CreateApiKeyPayload,
) -> Result<crate::domain::apikey::ApiKeyCreated, InvokeError> {
    stores(&proxy)?.create_api_key(payload).map_err(Into::into)
}

#[tauri::command]
pub fn update_api_key(
    proxy: State<'_, ProxyHandle>,
    payload: UpdateApiKeyPayload,
) -> Result<crate::domain::apikey::ApiKeyPublic, InvokeError> {
    stores(&proxy)?.update_api_key(payload).map_err(Into::into)
}

#[tauri::command]
pub fn delete_api_key(proxy: State<'_, ProxyHandle>, id: i64) -> Result<(), InvokeError> {
    stores(&proxy)?.delete_api_key(id).map_err(Into::into)
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

#[derive(Debug, Serialize)]
pub struct ExportToPiResult {
    pub path: String,
    pub provider_id: String,
    pub model_count: usize,
    pub base_url: String,
    pub used_placeholder_key: bool,
}

/// 将当前分组写入 `~/.pi/agent/models.json` 的 model-hub 供应商。
/// `api_key` 可空：空则写入占位 key。
#[tauri::command]
pub fn export_to_pi_agent(
    proxy: State<'_, ProxyHandle>,
    api_key: Option<String>,
) -> Result<ExportToPiResult, InvokeError> {
    let status = proxy.status_snapshot().map_err(InvokeError::from)?;
    let groups = stores(&proxy)?.list_groups().map_err(InvokeError::from)?;
    let names: Vec<String> = groups.into_iter().map(|g| g.name).collect();
    let key = api_key.unwrap_or_default();
    let used_placeholder = key.trim().is_empty();
    let path = crate::pi_export::default_pi_models_path().map_err(InvokeError::from)?;
    crate::pi_export::export_model_hub_to_path(&path, &status.base_url, &key, &names)
        .map_err(InvokeError::from)?;
    Ok(ExportToPiResult {
        path: path.display().to_string(),
        provider_id: crate::pi_export::PI_PROVIDER_ID.to_string(),
        model_count: names.len(),
        base_url: crate::pi_export::normalize_openai_base_url(&status.base_url),
        used_placeholder_key: used_placeholder,
    })
}

#[derive(Debug, Serialize)]
pub struct HealthSnapshot {
    pub provider_id: i64,
    pub provider_name: String,
    pub state: String,
    pub consecutive_failures: u32,
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

#[tauri::command]
pub fn list_health(proxy: State<'_, ProxyHandle>) -> Result<Vec<HealthSnapshot>, InvokeError> {
    let stores = stores(&proxy)?;
    let circuits = proxy.circuits().map_err(InvokeError::from)?;
    let providers = stores.list_providers().map_err(InvokeError::from)?;
    let mut out = Vec::new();
    for p in providers {
        let label = circuits.health_label(p.id);
        let state = match label {
            crate::proxy::circuit::HealthLabel::Healthy => "healthy",
            crate::proxy::circuit::HealthLabel::Warning => "warning",
            crate::proxy::circuit::HealthLabel::Open => "open",
            crate::proxy::circuit::HealthLabel::HalfOpen => "half_open",
        };
        out.push(HealthSnapshot {
            provider_id: p.id,
            provider_name: p.name,
            state: state.into(),
            consecutive_failures: circuits.consecutive_failures(p.id),
        });
    }
    Ok(out)
}
