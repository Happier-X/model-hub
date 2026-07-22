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
  auto_failover: boolean;
  items: GroupItem[];
  created_at: string;
}

export interface ApiKeyPublic {
  id: number;
  name: string;
  masked: string;
  enabled: boolean;
  created_at: string;
}

export interface ApiKeyCreated extends ApiKeyPublic {
  raw_key: string;
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

export interface HealthSnapshot {
  provider_id: number;
  provider_name: string;
  state: "healthy" | "warning" | "open" | "half_open";
  consecutive_failures: number;
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

export const getPaths = () => invoke<AppPaths>("get_paths");
export const proxyStart = () => invoke<ProxyStatus>("proxy_start");
export const proxyStop = () => invoke<ProxyStatus>("proxy_stop");
export const proxyStatus = () => invoke<ProxyStatus>("proxy_status");
export const proxySetPort = (port: number) => invoke<ProxyStatus>("proxy_set_port", { port });

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

export const listGroups = () => invoke<Group[]>("list_groups");
export const createGroup = (payload: {
  name: string;
  auto_failover: boolean;
  items: { provider_id: number; upstream_model: string }[];
}) => invoke<Group>("create_group", { payload });
export const updateGroup = (payload: {
  id: number;
  name: string;
  auto_failover: boolean;
  items: { provider_id: number; upstream_model: string }[];
}) => invoke<Group>("update_group", { payload });
export const deleteGroup = (id: number) => invoke<void>("delete_group", { id });

export const listApiKeys = () => invoke<ApiKeyPublic[]>("list_api_keys");
export const createApiKey = (payload: { name: string; enabled: boolean }) =>
  invoke<ApiKeyCreated>("create_api_key", { payload });
export const updateApiKey = (payload: { id: number; name: string; enabled: boolean }) =>
  invoke<ApiKeyPublic>("update_api_key", { payload });
export const deleteApiKey = (id: number) => invoke<void>("delete_api_key", { id });

export const listLogs = (page = 1, pageSize = 50) =>
  invoke<RequestLog[]>("list_logs", { page, page_size: pageSize });
export const clearLogs = () => invoke<void>("clear_logs");
export const listHealth = () => invoke<HealthSnapshot[]>("list_health");

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
