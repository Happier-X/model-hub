import { useCallback, useEffect, useState, type FormEvent } from "react";
import { setBaseUrlProvider } from "./api/gatewayHttp";
import {
  gatewaySetPort,
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
import { ChannelsPage } from "./pages/ChannelsPage";
import { DashboardPage } from "./pages/DashboardPage";
import { GroupsPage } from "./pages/GroupsPage";
import { LogsPage } from "./pages/LogsPage";
import {
  checkForUpdate,
  downloadAndInstallUpdate,
  getCurrentVersion,
  relaunchAfterUpdate,
  type UpdateInfo,
} from "./api/updater";

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
          <h3 className="text-lg font-semibold">本地网关（默认 Rust）</h3>
          <p className="mt-1 text-sm text-slate-500">
            默认监听 127.0.0.1。正式安装包已内嵌 model-hub-gateway，启动时自动部署。开发请运行
            pnpm prepare:gateway-rust 或设置 MODEL_HUB_GATEWAY_BIN。关闭主窗口会
            <strong>隐藏到系统托盘</strong>，网关继续运行；请从托盘菜单选择「退出」才会停止网关并关闭应用。
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
              {gateway.binary_path ??
                "未解析（安装包应自带 model-hub-gateway；开发请 pnpm prepare:gateway-rust 或设置 MODEL_HUB_GATEWAY_BIN）"}
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

function GatewayPortPanel({
  gateway,
  busy,
  onSaved,
}: {
  gateway: GatewayStatus | null;
  busy: boolean;
  onSaved: (status: GatewayStatus) => void;
}) {
  const [draft, setDraft] = useState("");
  const [dirty, setDirty] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [message, setMessage] = useState<string | null>(null);
  const port = gateway?.port ?? 8080;

  useEffect(() => {
    if (!dirty && gateway) setDraft(String(gateway.port));
  }, [dirty, gateway]);

  const save = async (event: FormEvent) => {
    event.preventDefault();
    if (!/^\d+$/.test(draft)) {
      setError("请输入 1 到 65535 之间的整数端口，例如 18080。");
      return;
    }
    const value = Number(draft);
    if (!Number.isSafeInteger(value) || value < 1 || value > 65535) {
      setError("端口范围必须是 1 到 65535，请输入有效整数。");
      return;
    }
    setError(null);
    setMessage(null);
    try {
      const status = await gatewaySetPort(value);
      onSaved(status);
      setDraft(String(value));
      setDirty(false);
      if (status.state === "running") {
        setMessage(`已保存并按端口 ${value} 自动重启网关。`);
      } else {
        setMessage(`已保存端口 ${value}。若网关未运行，请查看状态条错误后重试启动。`);
      }
    } catch (cause: unknown) {
      setError(cause instanceof Error ? cause.message : String(cause));
    }
  };

  // 启动/停止过渡中禁用，避免并发切换；运行中允许直接改端口并自动重启。
  const transitioning =
    gateway?.state === "starting" || gateway?.state === "stopping";
  return (
    <section className="rounded-2xl border border-slate-200 bg-white p-6 shadow-sm">
      <h3 className="text-lg font-semibold">网关监听端口</h3>
      <p className="mt-1 text-sm text-slate-500">
        默认绑定 127.0.0.1。保存后会自动停止并按新端口重启网关；打开应用时也会默认启动网关。
      </p>
      <form onSubmit={(event) => void save(event)} className="mt-4 flex max-w-md items-end gap-3">
        <label className="flex-1 text-sm">
          <span className="font-medium text-slate-700">端口（当前 {port}）</span>
          <input type="number" min="1" max="65535" step="1" inputMode="numeric" required
            value={draft} onChange={(event) => { setDraft(event.target.value); setDirty(true); setError(null); setMessage(null); }}
            disabled={busy || transitioning} className="mt-1 w-full rounded-lg border border-slate-300 px-3 py-2 font-mono" />
        </label>
        <button type="submit" disabled={busy || transitioning} className="rounded-lg bg-cyan-600 px-3 py-2 text-sm font-medium text-white disabled:opacity-50">保存端口</button>
      </form>
      {transitioning ? (
        <p className="mt-3 text-sm text-amber-700">网关正在切换状态，请稍候再修改端口。</p>
      ) : null}
      {error ? <p role="alert" className="mt-3 text-sm text-red-700">{error}</p> : null}
      {message ? <p className="mt-3 text-sm text-emerald-700" aria-live="polite">{message}</p> : null}
    </section>
  );
}

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

function UpdaterPanel({ gatewayRunning }: { gatewayRunning: boolean }) {
  const [currentVersion, setCurrentVersion] = useState("读取中…");
  const [update, setUpdate] = useState<UpdateInfo | null>(null);
  const [checking, setChecking] = useState(false);
  const [installing, setInstalling] = useState(false);
  const [downloaded, setDownloaded] = useState(0);
  const [total, setTotal] = useState<number | undefined>();
  const [message, setMessage] = useState("仅在点击检查时访问 GitHub，不会在启动时自动联网。");
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    void getCurrentVersion().then(setCurrentVersion);
  }, []);

  const checkUpdate = async () => {
    setChecking(true);
    setError(null);
    if (update) {
      await update.update.close().catch(() => undefined);
    }
    setUpdate(null);
    setMessage("正在检查 GitHub 正式版本…");
    try {
      const result = await checkForUpdate();
      setUpdate(result);
      setMessage(result ? `发现新版本 ${result.version}` : "当前已是最新版本。");
    } catch (cause: unknown) {
      setError(cause instanceof Error ? cause.message : String(cause));
      setMessage("检查更新失败，当前版本不受影响。");
    } finally {
      setChecking(false);
    }
  };

  const installUpdate = async () => {
    if (!update) return;
    const gatewayHint = gatewayRunning
      ? "安装后重启应用会停止当前托管网关。"
      : "安装完成后可重启应用。";
    if (!window.confirm(`确认下载并安装 Model Hub ${update.version}？\n${gatewayHint}`)) return;

    setInstalling(true);
    setDownloaded(0);
    setTotal(undefined);
    setError(null);
    setMessage("正在下载并校验签名…");
    try {
      await downloadAndInstallUpdate(update.update, (progress) => {
        setDownloaded(progress.downloadedBytes);
        setTotal(progress.totalBytes);
        setMessage(progress.finished ? "更新已安装，等待重启。" : "正在下载并校验更新…");
      });
      await update.update.close().catch(() => undefined);
      setUpdate(null);
      if (window.confirm("更新已安装。是否立即重启 Model Hub？重启会安全停止托管网关。")) {
        await relaunchAfterUpdate();
      } else {
        setMessage("更新已安装。请稍后退出并重新打开 Model Hub 以完成升级。");
      }
    } catch (cause: unknown) {
      await update.update.close().catch(() => undefined);
      setUpdate(null);
      setError(cause instanceof Error ? cause.message : String(cause));
      setMessage("更新未完成，当前版本仍可继续使用。");
    } finally {
      setInstalling(false);
    }
  };

  const percent = total && total > 0 ? Math.min(100, Math.round((downloaded / total) * 100)) : null;
  return (
    <section className="rounded-2xl border border-slate-200 bg-white p-6 shadow-sm">
      <div className="flex flex-wrap items-start justify-between gap-4">
        <div>
          <h3 className="text-lg font-semibold">应用更新</h3>
          <p className="mt-1 text-sm text-slate-500">当前版本：v{currentVersion}。仅检查正式 GitHub Release，更新包必须通过 Tauri 签名校验。</p>
        </div>
        <button type="button" onClick={() => void checkUpdate()} disabled={checking || installing}
          className="rounded-lg bg-slate-900 px-3 py-2 text-sm font-medium text-white disabled:opacity-50">
          {checking ? "检查中…" : "检查更新"}
        </button>
      </div>
      <p className="mt-4 text-sm text-slate-600" aria-live="polite">{message}</p>
      {update ? (
        <div className="mt-4 rounded-xl border border-cyan-200 bg-cyan-50 p-4 text-sm">
          <p className="font-semibold text-cyan-900">v{update.currentVersion} → v{update.version}</p>
          {update.date ? <p className="mt-1 text-cyan-800">发布时间：{update.date}</p> : null}
          {update.body ? <pre className="mt-3 max-h-48 overflow-auto whitespace-pre-wrap font-sans text-cyan-950">{update.body}</pre> : null}
          <button type="button" onClick={() => void installUpdate()} disabled={installing}
            className="mt-4 rounded-lg bg-cyan-600 px-3 py-2 font-medium text-white disabled:opacity-50">
            {installing ? "正在安装…" : "下载并安装"}
          </button>
        </div>
      ) : null}
      {installing ? (
        <div className="mt-4">
          <div className="h-2 overflow-hidden rounded bg-slate-200"><div className="h-full bg-cyan-500 transition-all" style={{ width: percent === null ? "20%" : `${percent}%` }} /></div>
          <p className="mt-2 text-xs text-slate-500">已下载 {formatBytes(downloaded)}{total ? ` / ${formatBytes(total)}（${percent}%）` : ""}</p>
        </div>
      ) : null}
      {error ? <p role="alert" className="mt-4 rounded-lg bg-red-50 p-3 text-sm text-red-700">{error}</p> : null}
    </section>
  );
}

function ClientHintPanel({ baseUrl }: { baseUrl: string }) {
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
          本地开放模式：调用 <code className="rounded bg-slate-100 px-1">/v1/*</code> 无需
          API Key / 管理 Token（默认仅监听 127.0.0.1）。
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
                baseUrl={baseUrl}
                onNavigate={setActiveItem}
              />
            ) : null}
            {activeItem === "渠道" ? <ChannelsPage running={!!running} /> : null}
            {activeItem === "分组" ? <GroupsPage running={!!running} /> : null}
            {activeItem === "日志" ? <LogsPage running={!!running} /> : null}
            {activeItem === "设置" ? (
              <div className="space-y-6">
                <div>
                  <h2 className="text-2xl font-bold">设置</h2>
                  <p className="mt-1 text-sm text-slate-600">
                    网关生命周期、监听端口、应用更新与数据路径。本地模式无需管理登录。
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
                <GatewayPortPanel
                  gateway={gateway}
                  busy={gatewayBusy}
                  onSaved={setGateway}
                />
                <UpdaterPanel gatewayRunning={!!running} />
                <ClientHintPanel baseUrl={baseUrl} />
                <PathsPanel paths={paths} pathError={pathError} />
              </div>
            ) : null}
          </div>
        </main>
      </div>
    </div>
  );
}
