import { useCallback, useEffect, useState, type FormEvent } from "react";
import { applyManualAdminToken, ensureAdminSession } from "./api/auth";
import { setBaseUrlProvider } from "./api/gatewayHttp";
import {
  gatewayStart,
  gatewayStatus,
  gatewayStop,
  getPaths,
  type AppPaths,
  type GatewayStatus,
} from "./api/tauri";
import {
  Sidebar,
  type NavigationItem,
} from "./components/layout/Sidebar";
import { StatusBar } from "./components/layout/StatusBar";
import { ApiKeysPage } from "./pages/ApiKeysPage";
import { ChannelsPage } from "./pages/ChannelsPage";
import { DashboardPage } from "./pages/DashboardPage";
import { GroupsPage } from "./pages/GroupsPage";
import { LogsPage } from "./pages/LogsPage";

const pathLabels: Array<[keyof AppPaths, string]> = [
  ["app_data_dir", "应用数据根目录"],
  ["config_dir", "配置目录"],
  ["gateway_dir", "网关数据目录"],
  ["bin_dir", "程序目录"],
];

function PathsPanel({
  paths,
  pathError,
}: {
  paths: AppPaths | null;
  pathError: string | null;
}) {
  return (
    <section className="rounded-2xl border border-slate-200 bg-white p-6 shadow-sm">
      <div className="flex items-start justify-between gap-4">
        <div>
          <h3 className="text-lg font-semibold">本机数据目录</h3>
          <p className="mt-1 text-sm text-slate-500">
            配置、网关数据和程序文件将分目录存放。
          </p>
        </div>
        <span className="rounded-full bg-emerald-50 px-3 py-1 text-xs font-semibold text-emerald-700">
          路径契约已启用
        </span>
      </div>

      {pathError ? (
        <div role="alert" className="mt-5 rounded-lg bg-red-50 p-4 text-sm text-red-700">
          {pathError}
        </div>
      ) : paths ? (
        <dl className="mt-5 divide-y divide-slate-100 rounded-xl border border-slate-200">
          {pathLabels.map(([key, label]) => (
            <div
              key={key}
              className="grid gap-1 px-4 py-3 md:grid-cols-[10rem_1fr] md:gap-4"
            >
              <dt className="text-sm font-medium text-slate-600">{label}</dt>
              <dd className="break-all font-mono text-sm text-slate-900">
                {paths[key]}
              </dd>
            </div>
          ))}
        </dl>
      ) : (
        <p className="mt-5 text-sm text-slate-500" aria-live="polite">
          正在读取本机数据目录…
        </p>
      )}
    </section>
  );
}

function GatewayPanel({
  gateway,
  busy,
  actionError,
  onStart,
  onStop,
  onRefresh,
}: {
  gateway: GatewayStatus | null;
  busy: boolean;
  actionError: string | null;
  onStart: () => void;
  onStop: () => void;
  onRefresh: () => void;
}) {
  return (
    <section className="rounded-2xl border border-slate-200 bg-white p-6 shadow-sm">
      <div className="flex flex-wrap items-start justify-between gap-4">
        <div>
          <h3 className="text-lg font-semibold">网关侧车</h3>
          <p className="mt-1 text-sm text-slate-500">
            默认监听 127.0.0.1。请将 octopus.exe 放到程序目录，详见 gateway/README.md。
          </p>
        </div>
        <div className="flex flex-wrap gap-2">
          <button
            type="button"
            onClick={onRefresh}
            disabled={busy}
            className="rounded-lg border border-slate-200 px-3 py-2 text-sm font-medium text-slate-700 hover:bg-slate-50 disabled:opacity-50"
          >
            刷新状态
          </button>
          <button
            type="button"
            onClick={onStart}
            disabled={
              busy || gateway?.state === "running" || gateway?.state === "starting"
            }
            className="rounded-lg bg-cyan-600 px-3 py-2 text-sm font-medium text-white hover:bg-cyan-500 disabled:opacity-50"
          >
            启动网关
          </button>
          <button
            type="button"
            onClick={onStop}
            disabled={busy || gateway?.state === "idle" || gateway?.state === "stopping"}
            className="rounded-lg border border-red-200 bg-red-50 px-3 py-2 text-sm font-medium text-red-700 hover:bg-red-100 disabled:opacity-50"
          >
            停止网关
          </button>
        </div>
      </div>

      {actionError ? (
        <div role="alert" className="mt-4 rounded-lg bg-red-50 p-4 text-sm text-red-700">
          {actionError}
        </div>
      ) : null}

      {gateway ? (
        <dl className="mt-5 grid gap-3 rounded-xl border border-slate-200 p-4 text-sm md:grid-cols-2">
          <div>
            <dt className="text-slate-500">状态</dt>
            <dd className="font-medium text-slate-900">{gateway.state}</dd>
          </div>
          <div>
            <dt className="text-slate-500">Base URL</dt>
            <dd className="break-all font-mono text-slate-900">{gateway.base_url}</dd>
          </div>
          <div>
            <dt className="text-slate-500">监听</dt>
            <dd className="font-mono text-slate-900">
              {gateway.host}:{gateway.port}
            </dd>
          </div>
          <div>
            <dt className="text-slate-500">PID</dt>
            <dd className="font-mono text-slate-900">{gateway.pid ?? "—"}</dd>
          </div>
          <div className="md:col-span-2">
            <dt className="text-slate-500">数据目录</dt>
            <dd className="break-all font-mono text-slate-900">{gateway.data_dir}</dd>
          </div>
          <div className="md:col-span-2">
            <dt className="text-slate-500">二进制</dt>
            <dd className="break-all font-mono text-slate-900">
              {gateway.binary_path ?? "未解析（请放置 octopus.exe）"}
            </dd>
          </div>
          {gateway.last_error ? (
            <div className="md:col-span-2">
              <dt className="text-slate-500">最近错误</dt>
              <dd className="text-red-700">{gateway.last_error}</dd>
            </div>
          ) : null}
        </dl>
      ) : (
        <p className="mt-5 text-sm text-slate-500">正在读取网关状态…</p>
      )}
    </section>
  );
}

function AuthPanel({
  authOk,
  authMessage,
  onRetry,
}: {
  authOk: boolean;
  authMessage: string;
  onRetry: () => void;
}) {
  const [tokenDraft, setTokenDraft] = useState("");

  const onSaveToken = (event: FormEvent) => {
    event.preventDefault();
    applyManualAdminToken(tokenDraft);
    onRetry();
  };

  return (
    <section className="rounded-2xl border border-slate-200 bg-white p-6 shadow-sm">
      <h3 className="text-lg font-semibold">管理 API 鉴权（无登录页）</h3>
      <p className="mt-1 text-sm text-slate-500">
        网关运行后会静默使用侧车默认 admin 账号换取 Bearer Token。失败时可粘贴 Token 兜底。
      </p>
      <p
        className={`mt-4 rounded-lg p-3 text-sm ${
          authOk ? "bg-emerald-50 text-emerald-800" : "bg-amber-50 text-amber-900"
        }`}
      >
        {authMessage || (authOk ? "管理 API 已就绪" : "尚未鉴权")}
      </p>
      <form onSubmit={onSaveToken} className="mt-4 space-y-3">
        <label className="block text-sm">
          <span className="font-medium text-slate-700">管理 Token（可选兜底）</span>
          <input
            className="mt-1 w-full rounded-lg border border-slate-300 px-3 py-2 font-mono text-sm"
            value={tokenDraft}
            onChange={(e) => setTokenDraft(e.target.value)}
            placeholder="粘贴 JWT / 管理令牌"
            autoComplete="off"
          />
        </label>
        <div className="flex flex-wrap gap-2">
          <button
            type="submit"
            className="rounded-lg bg-slate-900 px-3 py-2 text-sm font-medium text-white"
          >
            保存 Token 并重试
          </button>
          <button
            type="button"
            className="rounded-lg border border-slate-200 px-3 py-2 text-sm"
            onClick={onRetry}
          >
            重新静默鉴权
          </button>
        </div>
      </form>
    </section>
  );
}

function ClientHintPanel({
  baseUrl,
  onOpenApiKeys,
}: {
  baseUrl: string;
  onOpenApiKeys: () => void;
}) {
  return (
    <section className="rounded-2xl border border-slate-200 bg-white p-6 shadow-sm">
      <h3 className="text-lg font-semibold">客户端对接提示</h3>
      <ul className="mt-3 list-disc space-y-2 pl-5 text-sm text-slate-600">
        <li>
          Base URL：
          <code className="ml-1 rounded bg-slate-100 px-1 font-mono text-xs">
            {baseUrl}
          </code>
        </li>
        <li>
          OpenAI 兼容根：
          <code className="ml-1 rounded bg-slate-100 px-1 font-mono text-xs">
            {baseUrl}/v1
          </code>
        </li>
        <li>
          <code className="rounded bg-slate-100 px-1">model</code> 填分组名（不是上游模型名）
        </li>
        <li>
          客户端必须使用网关 API Key（前缀{" "}
          <code className="rounded bg-slate-100 px-1">sk-octopus-</code>
          ），与上方管理 JWT 不是同一套。请到{" "}
          <button
            type="button"
            className="font-medium text-cyan-700 underline"
            onClick={onOpenApiKeys}
          >
            API 密钥
          </button>{" "}
          页创建并复制完整 Key。
        </li>
      </ul>
    </section>
  );
}

export function App() {
  const [activeItem, setActiveItem] = useState<NavigationItem>("仪表盘");
  const [paths, setPaths] = useState<AppPaths | null>(null);
  const [pathError, setPathError] = useState<string | null>(null);
  const [gateway, setGateway] = useState<GatewayStatus | null>(null);
  const [gatewayBusy, setGatewayBusy] = useState(false);
  const [gatewayActionError, setGatewayActionError] = useState<string | null>(null);
  const [authOk, setAuthOk] = useState(false);
  const [authMessage, setAuthMessage] = useState("等待网关运行后鉴权…");

  useEffect(() => {
    setBaseUrlProvider(() => {
      if (gateway?.state === "running") {
        return gateway.base_url;
      }
      return null;
    });
  }, [gateway]);

  const refreshGateway = useCallback(async () => {
    try {
      const status = await gatewayStatus();
      setGateway(status);
    } catch (error: unknown) {
      const detail = error instanceof Error ? error.message : String(error);
      setGateway((current) =>
        current
          ? { ...current, state: "error", last_error: detail }
          : {
              state: "error",
              host: "127.0.0.1",
              port: 8080,
              pid: null,
              last_error: detail,
              base_url: "http://127.0.0.1:8080",
              data_dir: "—",
              binary_path: null,
            },
      );
    }
  }, []);

  const refreshAuth = useCallback(async () => {
    if (gateway?.state !== "running") {
      setAuthOk(false);
      setAuthMessage("网关未运行，跳过管理 API 鉴权");
      return;
    }
    const result = await ensureAdminSession();
    setAuthOk(result.ok);
    setAuthMessage(result.message);
  }, [gateway?.state]);

  useEffect(() => {
    getPaths()
      .then(setPaths)
      .catch((error: unknown) => {
        const detail = error instanceof Error ? error.message : String(error);
        setPathError(`无法读取数据目录：${detail}。请检查目录权限后重试。`);
      });
  }, []);

  useEffect(() => {
    void refreshGateway();
    const timer = window.setInterval(() => {
      void refreshGateway();
    }, 2000);
    return () => window.clearInterval(timer);
  }, [refreshGateway]);

  useEffect(() => {
    void refreshAuth();
  }, [refreshAuth]);

  const handleStart = async () => {
    setGatewayBusy(true);
    setGatewayActionError(null);
    try {
      const status = await gatewayStart();
      setGateway(status);
    } catch (error: unknown) {
      const detail = error instanceof Error ? error.message : String(error);
      setGatewayActionError(detail);
      await refreshGateway();
    } finally {
      setGatewayBusy(false);
    }
  };

  const handleStop = async () => {
    setGatewayBusy(true);
    setGatewayActionError(null);
    try {
      const status = await gatewayStop();
      setGateway(status);
    } catch (error: unknown) {
      const detail = error instanceof Error ? error.message : String(error);
      setGatewayActionError(detail);
      await refreshGateway();
    } finally {
      setGatewayBusy(false);
    }
  };

  const running = gateway?.state === "running";
  const baseUrl = gateway?.base_url ?? "http://127.0.0.1:8080";

  return (
    <div className="flex min-h-screen bg-slate-100 text-slate-900">
      <Sidebar
        activeItem={activeItem}
        onNavigate={setActiveItem}
        gateway={gateway}
      />
      <div className="flex min-w-0 flex-1 flex-col">
        <StatusBar gateway={gateway} />
        <main className="flex-1 overflow-auto p-8">
          <div className="mx-auto max-w-5xl">
            {activeItem === "仪表盘" ? (
              <DashboardPage
                running={!!running}
                authOk={authOk}
                authMessage={authMessage}
                baseUrl={baseUrl}
                onNavigate={setActiveItem}
              />
            ) : null}
            {activeItem === "渠道" ? (
              <ChannelsPage
                running={!!running}
                authOk={authOk}
                authMessage={authMessage}
              />
            ) : null}
            {activeItem === "分组" ? (
              <GroupsPage
                running={!!running}
                authOk={authOk}
                authMessage={authMessage}
              />
            ) : null}
            {activeItem === "API 密钥" ? (
              <ApiKeysPage
                running={!!running}
                authOk={authOk}
                authMessage={authMessage}
              />
            ) : null}
            {activeItem === "日志" ? (
              <LogsPage
                running={!!running}
                authOk={authOk}
                authMessage={authMessage}
              />
            ) : null}
            {activeItem === "设置" ? (
              <div className="space-y-6">
                <div>
                  <h2 className="text-2xl font-bold">设置</h2>
                  <p className="mt-1 text-sm text-slate-600">
                    网关生命周期、路径契约与管理 API 鉴权适配。
                  </p>
                </div>
                <GatewayPanel
                  gateway={gateway}
                  busy={gatewayBusy}
                  actionError={gatewayActionError}
                  onStart={() => void handleStart()}
                  onStop={() => void handleStop()}
                  onRefresh={() => void refreshGateway()}
                />
                <AuthPanel
                  authOk={authOk}
                  authMessage={authMessage}
                  onRetry={() => void refreshAuth()}
                />
                <ClientHintPanel
                  baseUrl={baseUrl}
                  onOpenApiKeys={() => setActiveItem("API 密钥")}
                />
                <PathsPanel paths={paths} pathError={pathError} />
              </div>
            ) : null}
          </div>
        </main>
      </div>
    </div>
  );
}
