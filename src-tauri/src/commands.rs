//! Tauri IPC 命令：代理启停 + 领域 CRUD。

use serde::Serialize;
use tauri::{AppHandle, State};

use crate::domain::group::{CreateGroupPayload, Group, UpdateGroupPayload};
use crate::domain::leaderboard::ModelLeaderboardSnapshot;
use crate::domain::pricing::ModelPrice;
use crate::domain::provider::{CreateProviderPayload, Provider, UpdateProviderPayload};
use crate::domain::upstream_models::{fetch_upstream_model_ids, FetchProviderModelsPayload};
use crate::domain::Stores;
use crate::error::{AppError, InvokeError};
use crate::paths;
use crate::proxy::{ProxyHandle, ProxyStatus};

use std::time::Duration;
use tokio::time::sleep;

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
    pub upstream_proxy_enabled: bool,
    pub upstream_proxy_url: String,
    pub upstream_proxy_user: String,
}

fn shell_prefs(config: &crate::settings::ShellConfig) -> ShellPrefs {
    ShellPrefs {
        gateway_port: config.gateway_port,
        check_update_on_startup: config.check_update_on_startup,
        overlay_enabled: config.overlay_enabled,
        upstream_proxy_enabled: config.upstream_proxy_enabled,
        upstream_proxy_url: config.upstream_proxy_url.clone(),
        upstream_proxy_user: config.upstream_proxy_user.clone(),
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
pub async fn set_overlay_enabled(app: AppHandle, enabled: bool) -> Result<ShellPrefs, InvokeError> {
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
pub fn set_upstream_proxy(
    app: AppHandle,
    proxy: State<'_, ProxyHandle>,
    enabled: bool,
    url: String,
    username: String,
    password: String,
) -> Result<ShellPrefs, InvokeError> {
    let paths = paths::resolve_paths(&app).map_err(InvokeError::from)?;
    let config_dir = std::path::Path::new(&paths.config_dir);
    let mut cfg = crate::settings::load_shell_config(config_dir).map_err(InvokeError::from)?;
    cfg.upstream_proxy_enabled = enabled;
    cfg.upstream_proxy_url = url;
    cfg.upstream_proxy_user = username;
    cfg.upstream_proxy_pass = password;
    crate::settings::save_shell_config(config_dir, &cfg).map_err(InvokeError::from)?;
    proxy
        .set_upstream_proxy(config_dir, &cfg)
        .map_err(InvokeError::from)?;
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

/// 同步单个供应商：拉取上游模型 → 全量替换本地持久化模型 → 记录同步时间。
/// 供应商不存在或未启用时静默跳过（返回 Ok，不视为失败）。
pub async fn perform_sync_provider(stores: &Stores, provider_id: i64) -> Result<(), AppError> {
    let provider = stores
        .get_provider(provider_id)?
        .ok_or_else(|| AppError::Business("供应商不存在".into()))?;

    if !provider.enabled {
        return Ok(());
    }

    let ids = fetch_upstream_model_ids(&provider.base_url, &provider.api_key).await?;

    stores.replace_provider_models(provider_id, &ids)?;

    let now = chrono::Utc::now().timestamp();
    stores.touch_provider_synced_at(provider_id, now)?;

    Ok(())
}

pub const SYNC_STALE_AFTER_SECS: i64 = 24 * 3600;
pub const SYNC_STAGGER: Duration = Duration::from_secs(5);

/// 后台轮询：遍历开启自动同步的供应商，未同步过或超过 24h 到期则逐个同步（5s 错峰）。
/// 单个供应商同步失败只记录 warning，不影响其他供应商。
pub async fn perform_due_provider_syncs(stores: &Stores) {
    let providers = match stores.list_providers() {
        Ok(p) => p,
        Err(e) => {
            tracing::warn!(error = %e, "后台同步：获取供应商列表失败");
            return;
        }
    };

    let now = chrono::Utc::now().timestamp();
    let mut first = true;

    for provider in providers {
        if !provider.enabled || !provider.auto_sync {
            continue;
        }

        let due = match provider.last_sync_at {
            None => true,
            Some(t) => now - t >= SYNC_STALE_AFTER_SECS,
        };

        if !due {
            continue;
        }

        if !first {
            sleep(SYNC_STAGGER).await;
        }
        first = false;

        if let Err(e) = perform_sync_provider(stores, provider.id).await {
            tracing::warn!(
                error = %e,
                provider_id = provider.id,
                provider_name = %provider.name,
                "后台同步供应商失败"
            );
        }
    }
}

/// 立即同步单个供应商（供应商页「立即同步」按钮）。成功后返回更新后的完整 Provider。
#[tauri::command]
pub async fn sync_provider_now(
    proxy: State<'_, ProxyHandle>,
    provider_id: i64,
) -> Result<Provider, InvokeError> {
    let s = stores(&proxy)?;
    perform_sync_provider(&s, provider_id).await?;
    s.get_provider(provider_id)?
        .ok_or_else(|| InvokeError::from(AppError::Business("供应商不存在".into())))
}

/// 读本地持久化的供应商模型列表（分组页左侧离线可用）。
#[tauri::command]
pub fn get_provider_models(
    proxy: State<'_, ProxyHandle>,
    provider_id: i64,
) -> Result<Vec<String>, InvokeError> {
    stores(&proxy)?
        .list_provider_models(provider_id)
        .map_err(Into::into)
}

/// 就地切换供应商自动同步开关（供应商页「自动同步」列）。
#[tauri::command]
pub fn set_provider_auto_sync(
    proxy: State<'_, ProxyHandle>,
    id: i64,
    enabled: bool,
) -> Result<Provider, InvokeError> {
    stores(&proxy)?
        .set_provider_auto_sync(id, enabled)
        .map_err(Into::into)
}

const OPENROUTER_MODELS_URL: &str = "https://openrouter.ai/api/v1/models";
const OPENROUTER_FETCH_TIMEOUT: Duration = Duration::from_secs(15);

/// 拉取 OpenRouter 模型单价（每百万 token 美元）。失败向上抛（后台调用方自行降级）。
pub async fn fetch_openrouter_pricing() -> Result<Vec<ModelPrice>, AppError> {
    let client = reqwest::Client::builder()
        .timeout(OPENROUTER_FETCH_TIMEOUT)
        .build()
        .map_err(|e| AppError::Business(format!("无法创建 HTTP 客户端：{e}")))?;
    let response = client
        .get(OPENROUTER_MODELS_URL)
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| AppError::Business(format!("请求 OpenRouter 模型列表失败：{e}")))?;
    if !response.status().is_success() {
        return Err(AppError::Business(format!(
            "OpenRouter 返回异常状态：{}",
            response.status()
        )));
    }
    let body = response
        .bytes()
        .await
        .map_err(|e| AppError::Business(format!("读取 OpenRouter 响应失败：{e}")))?;
    let prices = crate::domain::pricing::parse_openrouter_pricing(&body);
    if prices.is_empty() {
        return Err(AppError::Business("OpenRouter 响应未解析到任何模型价格".into()));
    }
    Ok(prices)
}

/// 后台轮询：单价从未同步或超过 24h 到期时从 OpenRouter 拉取一次；失败仅记录 warning。
pub async fn perform_due_price_syncs(stores: &Stores) {
    let last = match stores.last_pricing_sync_at() {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!(error = %e, "后台同步：读取单价同步时间失败");
            return;
        }
    };
    let now = chrono::Utc::now().timestamp();
    let due = match last {
        None => true,
        Some(t) => now - t >= SYNC_STALE_AFTER_SECS,
    };
    if !due {
        return;
    }
    match fetch_openrouter_pricing().await {
        Ok(prices) => {
            if let Err(e) = stores.replace_pricing(&prices) {
                tracing::warn!(error = %e, "后台同步：写入单价表失败");
            }
        }
        Err(e) => tracing::warn!(error = %e, "后台同步 OpenRouter 单价失败"),
    }
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

/// 时间序列统计（折线图数据源）：近 30 天按日 + 今日按小时（均为成功口径，含费用）。
#[tauri::command]
pub fn get_timeseries_stats(
    proxy: State<'_, ProxyHandle>,
) -> Result<crate::domain::log::TimeseriesStats, InvokeError> {
    let stores = stores(&proxy)?;
    let daily = stores.request_daily_stats(30)?;
    let hourly = stores.request_hourly_stats()?;
    Ok(crate::domain::log::TimeseriesStats { daily, hourly })
}

#[tauri::command]
pub fn get_request_overview(
    proxy: State<'_, ProxyHandle>,
) -> Result<crate::domain::log::RequestOverview, InvokeError> {
    stores(&proxy)?.request_overview().map_err(Into::into)
}

#[tauri::command]
pub fn get_last_success_request(
    proxy: State<'_, ProxyHandle>,
) -> Result<Option<crate::domain::log::LastSuccessRequest>, InvokeError> {
    stores(&proxy)?.last_success_request().map_err(Into::into)
}

/// 按本地自然日聚合过去 `days` 天（含今日）的请求总量，供首页热力图使用。
/// `days` 不传默认 365；领域层再钳制到 [1, 400]。
#[tauri::command]
pub fn get_request_daily_counts(
    proxy: State<'_, ProxyHandle>,
    days: Option<u32>,
) -> Result<crate::domain::log::RequestDailyCounts, InvokeError> {
    let days = days.unwrap_or(crate::domain::log::DAILY_COUNTS_DEFAULT_DAYS);
    stores(&proxy)?
        .request_daily_counts(days)
        .map_err(Into::into)
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

/// 获取 llm_benchmark 公共模型榜单（24h 缓存；可强制刷新；网络失败时 stale 回退）。
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
