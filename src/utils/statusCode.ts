/** 日志状态码基础配色。 */
export function statusCodeClass(code: number | null | undefined): string {
  if (!code) return "bg-muted text-muted-foreground";
  if (code >= 200 && code < 300) return "bg-success/15 text-success";
  if (code >= 400 && code < 500) return "bg-warning/15 text-warning";
  if (code >= 500) return "bg-destructive/15 text-destructive";
  return "bg-muted text-muted-foreground";
}
