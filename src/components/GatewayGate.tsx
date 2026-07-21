interface GatewayGateProps {
  running: boolean;
  /** @deprecated 本地开放模式忽略；保留 prop 以免大面积改调用方 */
  authOk?: boolean;
  authMessage?: string;
  children: React.ReactNode;
}

/**
 * 仅检查网关是否运行；管理/客户端鉴权已移除。
 */
export function GatewayGate({ running, children }: GatewayGateProps) {
  if (!running) {
    return (
      <div className="rounded-2xl border border-amber-200 bg-amber-50 px-5 py-6 text-amber-950">
        <h3 className="text-base font-semibold">请先启动网关</h3>
        <p className="mt-2 leading-6">
          打开应用会默认尝试启动网关。也可在设置页手动启动；状态条显示「运行中」后再使用本页功能。
        </p>
      </div>
    );
  }

  return <>{children}</>;
}
