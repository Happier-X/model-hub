import type { HealthSnapshot } from "../api/tauri";

export type HealthState = HealthSnapshot["state"];

/** 健康状态中文文案 */
export function healthStateLabel(state: HealthState | undefined): string {
  switch (state) {
    case "open":
      return "熔断";
    case "half_open":
      return "半开";
    case "warning":
      return "警告";
    case "healthy":
    default:
      return "健康";
  }
}

/** 健康徽章 Tailwind 样式 */
export function healthStateClass(state: HealthState | undefined): string {
  switch (state) {
    case "open":
      return "bg-rose-100 text-rose-700";
    case "half_open":
      return "bg-amber-100 text-amber-800";
    case "warning":
      return "bg-orange-100 text-orange-700";
    case "healthy":
    default:
      return "bg-emerald-100 text-emerald-700";
  }
}

/** 按供应商 ID 查找健康快照 */
export function findHealth(
  snapshots: HealthSnapshot[],
  providerId: number,
): HealthSnapshot | undefined {
  return snapshots.find((x) => x.provider_id === providerId);
}

/** 日志状态码基础配色 */
export function statusCodeClass(code: number | null | undefined): string {
  if (!code) return "bg-slate-100 text-slate-600";
  if (code >= 200 && code < 300) return "bg-emerald-100 text-emerald-700";
  if (code >= 400 && code < 500) return "bg-amber-100 text-amber-800";
  if (code >= 500) return "bg-rose-100 text-rose-700";
  return "bg-slate-100 text-slate-600";
}
