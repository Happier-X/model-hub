import type { GatewayStatus } from "../../api/tauri";
import { gatewayStateLabel } from "../../api/tauri";

interface StatusBarProps {
  gateway: GatewayStatus | null;
}

function statusDotClass(state: GatewayStatus["state"] | undefined): string {
  switch (state) {
    case "running":
      return "bg-emerald-500";
    case "starting":
    case "stopping":
      return "bg-amber-400";
    case "error":
      return "bg-red-500";
    default:
      return "bg-slate-400";
  }
}

export function StatusBar({ gateway }: StatusBarProps) {
  const label = gateway ? gatewayStateLabel(gateway.state) : "读取中";
  const detail = gateway
    ? gateway.state === "running"
      ? gateway.base_url
      : gateway.last_error ?? gateway.base_url
    : "正在获取网关状态…";

  return (
    <header className="flex h-16 items-center justify-between border-b border-slate-200 bg-white px-8">
      <div>
        <p className="text-sm font-semibold text-slate-900">本地管理控制台</p>
        <p className="text-xs text-slate-500">无需登录，配置仅保存在本机</p>
      </div>
      <div
        className="flex max-w-xl items-center gap-2 rounded-full border border-slate-200 bg-slate-50 px-3 py-1.5 text-xs font-medium text-slate-600"
        title={detail}
      >
        <span className={`h-2 w-2 shrink-0 rounded-full ${statusDotClass(gateway?.state)}`} />
        <span className="truncate">
          网关状态：{label}
          {gateway?.state === "running" ? ` · ${gateway.port}` : ""}
        </span>
      </div>
    </header>
  );
}
