import { useCallback, useEffect, useState } from "react";
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
            disabled={busy || gateway?.state === "running" || gateway?.state === "starting"}
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

export function App() {
  const [activeItem, setActiveItem] = useState<NavigationItem>("仪表盘");
  const [paths, setPaths] = useState<AppPaths | null>(null);
  const [pathError, setPathError] = useState<string | null>(null);
  const [gateway, setGateway] = useState<GatewayStatus | null>(null);
  const [gatewayBusy, setGatewayBusy] = useState(false);
  const [gatewayActionError, setGatewayActionError] = useState<string | null>(null);

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
          <div className="mx-auto flex max-w-5xl flex-col gap-6">
            <div>
              <p className="text-sm font-medium text-cyan-700">{activeItem}</p>
              <h2 className="mt-1 text-3xl font-bold tracking-tight">
                欢迎使用 Model Hub
              </h2>
              <p className="mt-3 max-w-2xl text-sm leading-6 text-slate-600">
                桌面壳已支持网关侧车启停。渠道、分组与日志管理界面将在后续任务接入。
              </p>
            </div>

            {activeItem === "设置" ? (
              <>
                <GatewayPanel
                  gateway={gateway}
                  busy={gatewayBusy}
                  actionError={gatewayActionError}
                  onStart={() => void handleStart()}
                  onStop={() => void handleStop()}
                  onRefresh={() => void refreshGateway()}
                />
                <PathsPanel paths={paths} pathError={pathError} />
              </>
            ) : (
              <>
                <GatewayPanel
                  gateway={gateway}
                  busy={gatewayBusy}
                  actionError={gatewayActionError}
                  onStart={() => void handleStart()}
                  onStop={() => void handleStop()}
                  onRefresh={() => void refreshGateway()}
                />
                <PathsPanel paths={paths} pathError={pathError} />
                <section className="grid gap-4 md:grid-cols-3">
                  {[
                    ["渠道管理", "添加和维护上游模型渠道。"],
                    ["分组路由", "组织渠道并配置转发策略。"],
                    ["运行日志", "查看网关请求与运行状态。"],
                  ].map(([title, description]) => (
                    <article
                      key={title}
                      className="rounded-xl border border-dashed border-slate-300 bg-white/70 p-5"
                    >
                      <h3 className="font-semibold">{title}</h3>
                      <p className="mt-2 text-sm text-slate-500">{description}</p>
                      <span className="mt-4 inline-block text-xs font-medium text-slate-400">
                        即将提供
                      </span>
                    </article>
                  ))}
                </section>
              </>
            )}
          </div>
        </main>
      </div>
    </div>
  );
}
