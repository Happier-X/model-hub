import { getVersion } from "@tauri-apps/api/app";
import { invoke, isTauri } from "@tauri-apps/api/core";
import { relaunch } from "@tauri-apps/plugin-process";
import { check, type DownloadEvent, type Update } from "@tauri-apps/plugin-updater";

export type ProxyState = "idle" | "starting" | "running" | "stopping" | "error";

export interface AppPaths {
  app_data_dir: string;
  config_dir: string;
  gateway_dir: string;
  bin_dir: string;
}

export interface ProxyStatus {
  state: ProxyState;
  host: string;
  port: number;
  last_error: string | null;
  base_url: string;
  data_dir: string;
  /** 端口被占用后自动改口时的中文说明 */
  port_note?: string | null;
}

export interface Provider {
  id: number;
  name: string;
  base_url: string;
  api_key: string;
  enabled: boolean;
  created_at: string;
}

export interface GroupItem {
  id: number;
  provider_id: number;
  provider_name?: string;
  upstream_model: string;
  sort_order: number;
}

export interface Group {
  id: number;
  name: string;
  items: GroupItem[];
  created_at: string;
}

export interface RequestLog {
  id: number;
  time: number;
  group_name: string;
  provider_name: string;
  upstream_model: string;
  status_code: number;
  use_time_ms: number;
  error: string;
  failover_from: string;
  failover_to: string;
  failover_reason: string;
}

export type LogStatusClass = "all" | "2xx" | "4xx" | "5xx" | "error";

export interface LogQuery {
  page?: number;
  page_size?: number;
  group_name?: string;
  status_class?: LogStatusClass;
  failover_only?: boolean;
}

export interface LogPage {
  items: RequestLog[];
  /** 当前筛选条件下的条数 */
  total: number;
  page: number;
  page_size: number;
  /** 库内总条数（未筛选） */
  stored_total: number;
  /** 保留天数 */
  retention_days: number;
}

export interface LogPurgeResult {
  deleted: number;
  retained: number;
  retention_days: number;
  cutoff_unix: number;
}

export interface RequestStats {
  total: number;
  success: number;
  failure: number;
  failover: number;
  day_start_unix: number;
  day_end_unix: number;
}

/** 全局最近一条成功请求（2xx 且 error 为空） */
export interface LastSuccessRequest {
  time: number;
  group_name: string;
  provider_name: string;
  upstream_model: string;
  status_code: number;
}

export interface InvokeErrorShape {
  code?: string;
  message?: string;
}

export function extractInvokeError(error: unknown): string {
  if (typeof error === "string") return error;
  if (error && typeof error === "object") {
    const e = error as InvokeErrorShape;
    if (e.message) return e.message;
  }
  return "未知错误";
}

export interface ShellPrefs {
  gateway_port: number;
  check_update_on_startup: boolean;
}

export const getPaths = () => invoke<AppPaths>("get_paths");
export const proxyStart = () => invoke<ProxyStatus>("proxy_start");
export const proxyStop = () => invoke<ProxyStatus>("proxy_stop");
export const proxyStatus = () => invoke<ProxyStatus>("proxy_status");
export const proxySetPort = (port: number) => invoke<ProxyStatus>("proxy_set_port", { port });
export const getShellPrefs = () => invoke<ShellPrefs>("get_shell_prefs");
export const setCheckUpdateOnStartup = (enabled: boolean) =>
  invoke<ShellPrefs>("set_check_update_on_startup", { enabled });

export const listProviders = () => invoke<Provider[]>("list_providers");
export const createProvider = (payload: {
  name: string;
  base_url: string;
  api_key: string;
  enabled: boolean;
}) => invoke<Provider>("create_provider", { payload });
export const updateProvider = (payload: {
  id: number;
  name: string;
  base_url: string;
  api_key: string;
  enabled: boolean;
}) => invoke<Provider>("update_provider", { payload });
export const deleteProvider = (id: number) => invoke<void>("delete_provider", { id });

/** 从上游 OpenAI 兼容 /models 拉取模型 id；已保存供应商或表单草稿二选一 */
export const fetchProviderModels = (payload: {
  provider_id?: number;
  base_url?: string;
  api_key?: string;
}) => invoke<string[]>("fetch_provider_models", { payload });

export const listGroups = () => invoke<Group[]>("list_groups");
export const createGroup = (payload: {
  name: string;
  items: { provider_id: number; upstream_model: string }[];
}) => invoke<Group>("create_group", { payload });
export const updateGroup = (payload: {
  id: number;
  name: string;
  items: { provider_id: number; upstream_model: string }[];
}) => invoke<Group>("update_group", { payload });
export const deleteGroup = (id: number) => invoke<void>("delete_group", { id });

export const listLogs = (query: LogQuery = {}) =>
  invoke<LogPage>("list_logs", {
    query: {
      page: query.page ?? 1,
      page_size: query.page_size ?? 50,
      group_name: query.group_name || undefined,
      status_class: query.status_class || "all",
      failover_only: query.failover_only ?? false,
    },
  });
export const clearLogs = () => invoke<void>("clear_logs");
export const purgeExpiredLogs = () => invoke<LogPurgeResult>("purge_expired_logs");
export const getRequestStats = () => invoke<RequestStats>("get_request_stats");
export const getLastSuccessRequest = () =>
  invoke<LastSuccessRequest | null>("get_last_success_request");

export interface ExportToPiResult {
  path: string;
  provider_id: string;
  model_count: number;
  base_url: string;
  group_name: string;
}

/** 将指定分组 upsert 到 ~/.pi/agent/models.json 的 model-hub（固定占位 Key，无 Key 入参） */
export const exportGroupToPiAgent = (groupId: number) =>
  invoke<ExportToPiResult>("export_group_to_pi_agent", { groupId });

/** OpenRouter 公共榜单模型（白名单字段）。 */
export interface LeaderboardModel {
  id: string;
  canonical_slug?: string | null;
  name?: string | null;
  intelligence_score?: number | null;
  coding_score?: number | null;
  agentic_score?: number | null;
}

export interface ModelLeaderboardSnapshot {
  source: string;
  fetched_at_unix: number;
  stale: boolean;
  cache_hit: boolean;
  models: LeaderboardModel[];
}

/** 获取 OpenRouter 模型榜单；默认用 24h 缓存，forceRefresh 时尝试网络。 */
export const getModelLeaderboard = (forceRefresh = false) =>
  invoke<ModelLeaderboardSnapshot>("get_model_leaderboard", {
    forceRefresh,
  });

/** 浏览器 / 非 Tauri 壳内无法使用更新与进程插件 */
export const DESKTOP_ONLY_UPDATE_HINT = "请在桌面应用内检查更新";

export function ensureDesktopShell(): void {
  if (!isTauri()) {
    throw new Error(DESKTOP_ONLY_UPDATE_HINT);
  }
}

export async function getAppVersion(): Promise<string> {
  ensureDesktopShell();
  return getVersion();
}

export async function checkForUpdate(): Promise<Update | null> {
  ensureDesktopShell();
  return check();
}

export async function downloadAndInstallUpdate(
  update: Update,
  onEvent?: (progress: DownloadEvent) => void,
): Promise<void> {
  ensureDesktopShell();
  await update.downloadAndInstall(onEvent);
}

export async function relaunchApp(): Promise<void> {
  ensureDesktopShell();
  await relaunch();
}

export type { DownloadEvent, Update };
