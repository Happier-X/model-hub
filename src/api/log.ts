import { gatewayHttp } from "./gatewayHttp";

/** 侧车 page_size 上限（v0.9.28） */
export const LOG_PAGE_SIZE_MAX = 100;
export const LOG_PAGE_SIZE_OPTIONS = [20, 50, 100] as const;

export interface RelayLog {
  id: number;
  time: number;
  request_model_name: string;
  channel_name: string;
  actual_model_name: string;
  input_tokens: number;
  output_tokens: number;
  use_time: number;
  cost: number;
  error: string;
}

export function clampLogPageSize(pageSize: number): number {
  if (!Number.isFinite(pageSize) || pageSize < 1) {
    return 20;
  }
  return Math.min(LOG_PAGE_SIZE_MAX, Math.floor(pageSize));
}

export async function listLogs(page = 1, pageSize = 20): Promise<RelayLog[]> {
  const safePage = page < 1 ? 1 : Math.floor(page);
  const safeSize = clampLogPageSize(pageSize);
  const query = new URLSearchParams({
    page: String(safePage),
    page_size: String(safeSize),
  });
  const data = await gatewayHttp.get<RelayLog[] | null>(
    `/api/v1/log/list?${query.toString()}`,
  );
  return data ?? [];
}

export async function clearLogs(): Promise<unknown> {
  return gatewayHttp.delete("/api/v1/log/clear");
}

export function formatLogTime(unixSecondsOrMs: number): string {
  const ms = unixSecondsOrMs > 1e12 ? unixSecondsOrMs : unixSecondsOrMs * 1000;
  try {
    return new Date(ms).toLocaleString();
  } catch {
    return String(unixSecondsOrMs);
  }
}

export function logMatchesFilter(
  log: RelayLog,
  keyword: string,
  onlyErrors: boolean,
): boolean {
  if (onlyErrors && !log.error?.trim()) {
    return false;
  }
  const q = keyword.trim().toLowerCase();
  if (!q) {
    return true;
  }
  const haystack = [
    log.request_model_name,
    log.actual_model_name,
    log.channel_name,
    log.error,
  ]
    .filter(Boolean)
    .join(" ")
    .toLowerCase();
  return haystack.includes(q);
}
