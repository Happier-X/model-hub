/** 日志状态码基础配色。 */
export function statusCodeClass(code: number | null | undefined): string {
  if (!code) return "bg-slate-100 text-slate-600";
  if (code >= 200 && code < 300) return "bg-emerald-100 text-emerald-700";
  if (code >= 400 && code < 500) return "bg-amber-100 text-amber-800";
  if (code >= 500) return "bg-rose-100 text-rose-700";
  return "bg-slate-100 text-slate-600";
}
