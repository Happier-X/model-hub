import type { ReactNode } from "react";

interface GatewayGateProps {
  running: boolean;
  authOk: boolean;
  authMessage: string;
  children: ReactNode;
}

export function GatewayGate({
  running,
  authOk,
  authMessage,
  children,
}: GatewayGateProps) {
  if (!running) {
    return (
      <div
        role="status"
        className="rounded-2xl border border-amber-200 bg-amber-50 p-6 text-sm text-amber-900"
      >
        <p className="font-semibold">网关未运行</p>
        <p className="mt-2 leading-6">
          请先在「设置」中启动网关侧车，再管理渠道、分组、API 密钥与日志。业务数据通过本机
          HTTP API 访问，不会在网关停止时假装加载成功。
        </p>
      </div>
    );
  }

  if (!authOk) {
    return (
      <div
        role="alert"
        className="rounded-2xl border border-red-200 bg-red-50 p-6 text-sm text-red-800"
      >
        <p className="font-semibold">管理 API 未就绪</p>
        <p className="mt-2 leading-6">{authMessage}</p>
        <p className="mt-2 leading-6 text-red-700">
          本应用无登录页；将尝试静默使用侧车默认账号。若你已修改密码，请到设置粘贴管理
          Token。
        </p>
      </div>
    );
  }

  return <>{children}</>;
}
