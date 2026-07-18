import { useCallback, useEffect, useMemo, useState } from "react";
import { listApiKeys } from "../api/apikey";
import { listChannels } from "../api/channel";
import { listGroups } from "../api/group";
import type { NavigationItem } from "../components/layout/Sidebar";

type StepStatus = "ok" | "todo" | "blocked" | "error" | "info";

interface DashboardPageProps {
  running: boolean;
  authOk: boolean;
  authMessage: string;
  baseUrl: string;
  onNavigate: (item: NavigationItem) => void;
}

interface Counts {
  channels: number;
  groups: number;
  apiKeys: number;
}

function statusStyles(status: StepStatus): string {
  switch (status) {
    case "ok":
      return "border-emerald-200 bg-emerald-50 text-emerald-900";
    case "todo":
      return "border-amber-200 bg-amber-50 text-amber-950";
    case "blocked":
      return "border-slate-200 bg-slate-50 text-slate-600";
    case "error":
      return "border-red-200 bg-red-50 text-red-800";
    case "info":
    default:
      return "border-cyan-200 bg-cyan-50 text-cyan-950";
  }
}

function statusLabel(status: StepStatus): string {
  switch (status) {
    case "ok":
      return "已完成";
    case "todo":
      return "未完成";
    case "blocked":
      return "等待前置";
    case "error":
      return "检测失败";
    case "info":
      return "说明";
  }
}

export function DashboardPage({
  running,
  authOk,
  authMessage,
  baseUrl,
  onNavigate,
}: DashboardPageProps) {
  const [counts, setCounts] = useState<Counts | null>(null);
  const [loading, setLoading] = useState(false);
  const [listError, setListError] = useState<string | null>(null);
  const [copyHint, setCopyHint] = useState<string | null>(null);

  const canProbe = running && authOk;
  const root = baseUrl.replace(/\/$/, "");
  const v1Root = `${root}/v1`;

  const refresh = useCallback(async () => {
    if (!canProbe) {
      setCounts(null);
      setListError(null);
      return;
    }
    setLoading(true);
    setListError(null);
    try {
      const [channels, groups, keys] = await Promise.all([
        listChannels(),
        listGroups(),
        listApiKeys(),
      ]);
      setCounts({
        channels: channels.length,
        groups: groups.length,
        apiKeys: keys.length,
      });
    } catch (err: unknown) {
      setCounts(null);
      setListError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  }, [canProbe]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const steps = useMemo(() => {
    const step1: StepStatus = running ? "ok" : "todo";
    const step2: StepStatus = !running ? "blocked" : authOk ? "ok" : "todo";

    let step3: StepStatus = "blocked";
    let step4: StepStatus = "blocked";
    let step5: StepStatus = "blocked";
    if (canProbe) {
      if (listError) {
        step3 = step4 = step5 = "error";
      } else if (counts) {
        step3 = counts.channels > 0 ? "ok" : "todo";
        step4 = counts.groups > 0 ? "ok" : "todo";
        step5 = counts.apiKeys > 0 ? "ok" : "todo";
      } else if (loading) {
        step3 = step4 = step5 = "blocked";
      }
    }

    return [
      {
        id: 1,
        title: "启动网关",
        detail: running
          ? "侧车进程运行中"
          : "请到设置启动网关（默认 127.0.0.1:8080）",
        status: step1,
        actionLabel: "去设置",
        target: "设置" as NavigationItem,
      },
      {
        id: 2,
        title: "管理 API 鉴权",
        detail: !running
          ? "等待网关运行"
          : authOk
            ? authMessage || "管理 JWT 已就绪"
            : authMessage || "静默鉴权失败，可到设置粘贴管理 Token",
        status: step2,
        actionLabel: "去设置",
        target: "设置" as NavigationItem,
      },
      {
        id: 3,
        title: "配置渠道",
        detail: !canProbe
          ? "需先完成步骤 1–2"
          : listError
            ? listError
            : counts
              ? counts.channels > 0
                ? `已有 ${counts.channels} 条渠道`
                : "至少创建 1 条 OpenAI Chat 兼容渠道（type=0）"
              : loading
                ? "检测中…"
                : "尚未检测",
        status: step3,
        actionLabel: "去渠道",
        target: "渠道" as NavigationItem,
      },
      {
        id: 4,
        title: "配置分组",
        detail: !canProbe
          ? "需先完成步骤 1–2"
          : listError
            ? listError
            : counts
              ? counts.groups > 0
                ? `已有 ${counts.groups} 条分组（客户端 model 填分组名）`
                : "至少创建 1 条分组并绑定渠道"
              : loading
                ? "检测中…"
                : "尚未检测",
        status: step4,
        actionLabel: "去分组",
        target: "分组" as NavigationItem,
      },
      {
        id: 5,
        title: "创建网关 API Key",
        detail: !canProbe
          ? "需先完成步骤 1–2"
          : listError
            ? listError
            : counts
              ? counts.apiKeys > 0
                ? `已有 ${counts.apiKeys} 条密钥（客户端用 sk-octopus-...，非管理 JWT）`
                : "到 API 密钥页创建并复制完整 Key"
              : loading
                ? "检测中…"
                : "尚未检测",
        status: step5,
        actionLabel: "去 API 密钥",
        target: "API 密钥" as NavigationItem,
      },
    ];
  }, [
    running,
    authOk,
    authMessage,
    canProbe,
    counts,
    listError,
    loading,
  ]);

  const allReady =
    canProbe &&
    !!counts &&
    !listError &&
    counts.channels > 0 &&
    counts.groups > 0 &&
    counts.apiKeys > 0;

  const modelsCurl = `curl -sS "${v1Root}/models" \\\n  -H "Authorization: Bearer sk-octopus-YOUR_KEY"`;
  const chatCurl = `curl -sS "${v1Root}/chat/completions" \\\n  -H "Authorization: Bearer sk-octopus-YOUR_KEY" \\\n  -H "Content-Type: application/json" \\\n  -d "{\\"model\\":\\"your-group-name\\",\\"messages\\":[{\\"role\\":\\"user\\",\\"content\\":\\"hi\\"}]}"`;

  const onCopy = async (text: string, label: string) => {
    try {
      await navigator.clipboard.writeText(text);
      setCopyHint(`已复制：${label}`);
    } catch {
      setCopyHint("复制失败，请手动选中文本");
    }
  };

  return (
    <div className="space-y-6">
      <div className="flex flex-wrap items-start justify-between gap-4">
        <div>
          <h2 className="text-2xl font-bold">仪表盘</h2>
          <p className="mt-1 text-sm text-slate-600">
            按顺序完成配置闭环。管理 JWT 与客户端{" "}
            <code className="rounded bg-slate-100 px-1">sk-octopus-</code> Key
            不是同一套凭证。
          </p>
        </div>
        <button
          type="button"
          onClick={() => void refresh()}
          disabled={loading}
          className="rounded-lg border border-slate-200 bg-white px-3 py-2 text-sm font-medium text-slate-700 hover:bg-slate-50 disabled:opacity-50"
        >
          {loading ? "检测中…" : "刷新检查"}
        </button>
      </div>

      {allReady ? (
        <div
          role="status"
          className="rounded-2xl border border-emerald-200 bg-emerald-50 p-4 text-sm text-emerald-900"
        >
          <p className="font-semibold">配置闭环已就绪</p>
          <p className="mt-1">
            可用下方 curl 模板调用{" "}
            <code className="rounded bg-white/80 px-1">/v1</code>
            。请将 Key 占位符换成 API 密钥页复制的完整明文；
            <code className="rounded bg-white/80 px-1">model</code> 填分组名。
          </p>
        </div>
      ) : null}

      <section className="space-y-3">
        <h3 className="text-lg font-semibold">配置检查清单</h3>
        <ol className="space-y-3">
          {steps.map((step) => (
            <li
              key={step.id}
              className={`rounded-2xl border p-4 shadow-sm ${statusStyles(step.status)}`}
            >
              <div className="flex flex-wrap items-start justify-between gap-3">
                <div className="min-w-0 flex-1">
                  <div className="flex flex-wrap items-center gap-2">
                    <span className="text-xs font-semibold tracking-wide opacity-80">
                      步骤 {step.id}
                    </span>
                    <span className="rounded-full bg-white/70 px-2 py-0.5 text-xs font-semibold">
                      {statusLabel(step.status)}
                    </span>
                  </div>
                  <p className="mt-1 font-semibold">{step.title}</p>
                  <p className="mt-1 text-sm leading-6 opacity-90">{step.detail}</p>
                </div>
                <button
                  type="button"
                  onClick={() => onNavigate(step.target)}
                  className="shrink-0 rounded-lg border border-slate-300/80 bg-white px-3 py-2 text-sm font-medium text-slate-800 hover:bg-slate-50"
                >
                  {step.actionLabel}
                </button>
              </div>
            </li>
          ))}
        </ol>
      </section>

      <section className="rounded-2xl border border-slate-200 bg-white p-6 shadow-sm">
        <div className="flex flex-wrap items-start justify-between gap-3">
          <div>
            <h3 className="text-lg font-semibold">客户端快速对接</h3>
            <p className="mt-1 text-sm text-slate-600">
              模板中的{" "}
              <code className="rounded bg-slate-100 px-1">sk-octopus-YOUR_KEY</code>{" "}
              仅为占位；请到{" "}
              <button
                type="button"
                className="font-medium text-cyan-700 underline"
                onClick={() => onNavigate("API 密钥")}
              >
                API 密钥
              </button>{" "}
              页复制完整 Key。
            </p>
          </div>
          <div className="flex flex-wrap gap-2">
            <button
              type="button"
              className="rounded-lg border border-slate-200 px-3 py-1.5 text-xs font-medium"
              onClick={() => void onCopy(root, "Base URL")}
            >
              复制 Base URL
            </button>
            <button
              type="button"
              className="rounded-lg border border-slate-200 px-3 py-1.5 text-xs font-medium"
              onClick={() => void onCopy(v1Root, "OpenAI 兼容根")}
            >
              复制 /v1
            </button>
          </div>
        </div>

        <dl className="mt-4 grid gap-2 text-sm md:grid-cols-2">
          <div className="rounded-lg bg-slate-50 px-3 py-2">
            <dt className="text-slate-500">Base URL</dt>
            <dd className="break-all font-mono text-xs text-slate-900">{root}</dd>
          </div>
          <div className="rounded-lg bg-slate-50 px-3 py-2">
            <dt className="text-slate-500">OpenAI 兼容根</dt>
            <dd className="break-all font-mono text-xs text-slate-900">{v1Root}</dd>
          </div>
        </dl>

        <div className="mt-4 space-y-4">
          <div>
            <div className="mb-2 flex items-center justify-between gap-2">
              <p className="text-sm font-medium text-slate-700">GET /v1/models</p>
              <button
                type="button"
                className="text-xs font-medium text-cyan-700 underline"
                onClick={() => void onCopy(modelsCurl, "models curl")}
              >
                复制
              </button>
            </div>
            <pre className="overflow-x-auto rounded-xl bg-slate-950 p-4 text-xs leading-5 text-slate-100">
              {modelsCurl}
            </pre>
          </div>
          <div>
            <div className="mb-2 flex items-center justify-between gap-2">
              <p className="text-sm font-medium text-slate-700">
                POST /v1/chat/completions
              </p>
              <button
                type="button"
                className="text-xs font-medium text-cyan-700 underline"
                onClick={() => void onCopy(chatCurl, "chat curl")}
              >
                复制
              </button>
            </div>
            <pre className="overflow-x-auto rounded-xl bg-slate-950 p-4 text-xs leading-5 text-slate-100">
              {chatCurl}
            </pre>
          </div>
        </div>

        {copyHint ? (
          <p className="mt-3 text-sm text-emerald-700" role="status">
            {copyHint}
          </p>
        ) : null}
      </section>
    </div>
  );
}
