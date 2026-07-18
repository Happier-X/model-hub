import { useCallback, useEffect, useMemo, useState } from "react";
import {
  clearLogs,
  formatLogTime,
  listLogs,
  logMatchesFilter,
  LOG_PAGE_SIZE_OPTIONS,
  type RelayLog,
} from "../api/log";
import { GatewayGate } from "../components/GatewayGate";

interface LogsPageProps {
  running: boolean;
  authOk: boolean;
  authMessage: string;
}

export function LogsPage({ running, authOk, authMessage }: LogsPageProps) {
  const [logs, setLogs] = useState<RelayLog[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [page, setPage] = useState(1);
  const [pageSize, setPageSize] = useState<number>(20);
  const [keyword, setKeyword] = useState("");
  const [onlyErrors, setOnlyErrors] = useState(false);
  const [expandedId, setExpandedId] = useState<number | null>(null);
  const [autoRefresh, setAutoRefresh] = useState(true);
  const [copyHint, setCopyHint] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    if (!running || !authOk) {
      return;
    }
    setLoading(true);
    setError(null);
    try {
      setLogs(await listLogs(page, pageSize));
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  }, [running, authOk, page, pageSize]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  useEffect(() => {
    if (!running || !authOk || !autoRefresh) {
      return;
    }
    const timer = window.setInterval(() => {
      void refresh();
    }, 5000);
    return () => window.clearInterval(timer);
  }, [refresh, running, authOk, autoRefresh]);

  const visibleLogs = useMemo(
    () => logs.filter((log) => logMatchesFilter(log, keyword, onlyErrors)),
    [logs, keyword, onlyErrors],
  );

  const hasPrev = page > 1;
  const hasNext = logs.length >= pageSize;

  const onClear = async () => {
    const ok = window.confirm("确定清空全部请求日志？此操作不可撤销。");
    if (!ok) {
      return;
    }
    setError(null);
    try {
      await clearLogs();
      setExpandedId(null);
      setPage(1);
      await refresh();
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : String(err));
    }
  };

  const onCopyError = async (text: string) => {
    try {
      await navigator.clipboard.writeText(text);
      setCopyHint("已复制错误信息");
    } catch {
      setCopyHint("复制失败，请手动选中");
    }
  };

  return (
    <div className="space-y-6">
      <div className="flex flex-wrap items-end justify-between gap-3">
        <div>
          <h2 className="text-2xl font-bold">日志</h2>
          <p className="mt-1 text-sm text-slate-600">
            侧车请求日志列表（轮询，非 SSE）。过滤仅作用于<strong>当前页</strong>
            已加载数据。
          </p>
        </div>
        <div className="flex flex-wrap gap-2">
          <label className="flex items-center gap-2 rounded-lg border border-slate-200 bg-white px-3 py-1.5 text-sm">
            <input
              type="checkbox"
              checked={autoRefresh}
              onChange={(e) => setAutoRefresh(e.target.checked)}
            />
            自动刷新
          </label>
          <button
            type="button"
            className="rounded-lg border border-slate-200 px-3 py-1.5 text-sm"
            onClick={() => void refresh()}
          >
            刷新
          </button>
          <button
            type="button"
            className="rounded-lg border border-red-200 bg-red-50 px-3 py-1.5 text-sm text-red-700"
            onClick={() => void onClear()}
          >
            清空
          </button>
        </div>
      </div>

      <GatewayGate running={running} authOk={authOk} authMessage={authMessage}>
        <section className="rounded-2xl border border-slate-200 bg-white p-6 shadow-sm">
          <div className="flex flex-wrap items-end gap-3">
            <label className="block min-w-[12rem] flex-1 text-sm">
              <span className="font-medium text-slate-700">关键词（当前页）</span>
              <input
                className="mt-1 w-full rounded-lg border border-slate-300 px-3 py-2 text-sm"
                value={keyword}
                onChange={(e) => setKeyword(e.target.value)}
                placeholder="模型 / 渠道 / 错误"
              />
            </label>
            <label className="flex items-center gap-2 pb-2 text-sm">
              <input
                type="checkbox"
                checked={onlyErrors}
                onChange={(e) => setOnlyErrors(e.target.checked)}
              />
              仅显示有错误
            </label>
            <label className="block text-sm">
              <span className="font-medium text-slate-700">每页</span>
              <select
                className="mt-1 block rounded-lg border border-slate-300 px-3 py-2 text-sm"
                value={pageSize}
                onChange={(e) => {
                  setPageSize(Number(e.target.value));
                  setPage(1);
                  setExpandedId(null);
                }}
              >
                {LOG_PAGE_SIZE_OPTIONS.map((size) => (
                  <option key={size} value={size}>
                    {size}
                  </option>
                ))}
              </select>
            </label>
          </div>
          <p className="mt-2 text-xs text-slate-500">
            侧车 list 无服务端关键字过滤；换页后过滤条件仍只作用于新页数据。
          </p>

          {error ? (
            <p role="alert" className="mt-4 text-sm text-red-600">
              {error}
            </p>
          ) : null}
          {copyHint ? (
            <p className="mt-2 text-sm text-emerald-700" role="status">
              {copyHint}
            </p>
          ) : null}

          {loading && logs.length === 0 ? (
            <p className="mt-4 text-sm text-slate-500">加载中…</p>
          ) : logs.length === 0 ? (
            <p className="mt-4 text-sm text-slate-500">暂无日志。</p>
          ) : visibleLogs.length === 0 ? (
            <p className="mt-4 text-sm text-slate-500">
              当前页无匹配项（共加载 {logs.length} 条）。可放宽过滤或换页。
            </p>
          ) : (
            <div className="mt-4 overflow-x-auto">
              <table className="min-w-full text-left text-sm">
                <thead className="border-b border-slate-200 text-slate-500">
                  <tr>
                    <th className="px-2 py-2 font-medium">时间</th>
                    <th className="px-2 py-2 font-medium">请求模型</th>
                    <th className="px-2 py-2 font-medium">渠道</th>
                    <th className="px-2 py-2 font-medium">Token</th>
                    <th className="px-2 py-2 font-medium">错误</th>
                    <th className="px-2 py-2 font-medium">操作</th>
                  </tr>
                </thead>
                <tbody>
                  {visibleLogs.map((log) => {
                    const expanded = expandedId === log.id;
                    const errPreview = log.error?.trim()
                      ? log.error.length > 48
                        ? `${log.error.slice(0, 48)}…`
                        : log.error
                      : "—";
                    return (
                      <tr key={log.id} className="border-b border-slate-100 align-top">
                        <td className="px-2 py-2 whitespace-nowrap text-xs">
                          {formatLogTime(log.time)}
                        </td>
                        <td className="px-2 py-2 font-mono text-xs">
                          {log.request_model_name || "—"}
                        </td>
                        <td className="px-2 py-2 text-xs">{log.channel_name || "—"}</td>
                        <td className="px-2 py-2 text-xs">
                          {log.input_tokens}/{log.output_tokens}
                        </td>
                        <td
                          className={`max-w-xs px-2 py-2 text-xs ${
                            log.error ? "text-red-600" : "text-slate-500"
                          }`}
                        >
                          {errPreview}
                        </td>
                        <td className="px-2 py-2">
                          <button
                            type="button"
                            className="rounded border border-slate-200 px-2 py-1 text-xs"
                            onClick={() =>
                              setExpandedId(expanded ? null : log.id)
                            }
                          >
                            {expanded ? "收起" : "详情"}
                          </button>
                        </td>
                      </tr>
                    );
                  })}
                </tbody>
              </table>
            </div>
          )}

          {expandedId != null
            ? (() => {
                const log = logs.find((item) => item.id === expandedId);
                if (!log) {
                  return null;
                }
                return (
                  <div className="mt-4 rounded-xl border border-cyan-100 bg-cyan-50/50 p-4 text-sm">
                    <div className="flex flex-wrap items-start justify-between gap-2">
                      <p className="font-semibold text-slate-800">
                        日志详情 #{log.id}
                      </p>
                      <button
                        type="button"
                        className="text-xs text-slate-600 underline"
                        onClick={() => setExpandedId(null)}
                      >
                        关闭
                      </button>
                    </div>
                    <dl className="mt-3 grid gap-2 md:grid-cols-2">
                      <div>
                        <dt className="text-xs text-slate-500">时间</dt>
                        <dd>{formatLogTime(log.time)}</dd>
                      </div>
                      <div>
                        <dt className="text-xs text-slate-500">渠道</dt>
                        <dd>{log.channel_name || "—"}</dd>
                      </div>
                      <div>
                        <dt className="text-xs text-slate-500">请求模型（分组名）</dt>
                        <dd className="font-mono text-xs">
                          {log.request_model_name || "—"}
                        </dd>
                      </div>
                      <div>
                        <dt className="text-xs text-slate-500">实际上游模型</dt>
                        <dd className="font-mono text-xs">
                          {log.actual_model_name || "—"}
                        </dd>
                      </div>
                      <div>
                        <dt className="text-xs text-slate-500">Token 入/出</dt>
                        <dd>
                          {log.input_tokens} / {log.output_tokens}
                        </dd>
                      </div>
                      <div>
                        <dt className="text-xs text-slate-500">耗时 / 费用</dt>
                        <dd>
                          {log.use_time} · {log.cost}
                        </dd>
                      </div>
                      <div className="md:col-span-2">
                        <dt className="text-xs text-slate-500">错误</dt>
                        <dd className="mt-1 whitespace-pre-wrap break-all rounded-lg bg-white p-3 font-mono text-xs text-red-700">
                          {log.error?.trim() || "—"}
                        </dd>
                        {log.error?.trim() ? (
                          <button
                            type="button"
                            className="mt-2 text-xs font-medium text-cyan-700 underline"
                            onClick={() => void onCopyError(log.error)}
                          >
                            复制错误
                          </button>
                        ) : null}
                      </div>
                    </dl>
                  </div>
                );
              })()
            : null}

          <div className="mt-4 flex flex-wrap items-center justify-between gap-3 border-t border-slate-100 pt-4 text-sm">
            <p className="text-slate-600">
              第 {page} 页 · 本页加载 {logs.length} 条
              {keyword || onlyErrors
                ? ` · 过滤后显示 ${visibleLogs.length} 条`
                : ""}
              {loading ? " · 刷新中…" : ""}
            </p>
            <div className="flex gap-2">
              <button
                type="button"
                disabled={!hasPrev || loading}
                className="rounded-lg border border-slate-200 px-3 py-1.5 disabled:opacity-50"
                onClick={() => {
                  setExpandedId(null);
                  setPage((p) => Math.max(1, p - 1));
                }}
              >
                上一页
              </button>
              <button
                type="button"
                disabled={!hasNext || loading}
                className="rounded-lg border border-slate-200 px-3 py-1.5 disabled:opacity-50"
                onClick={() => {
                  setExpandedId(null);
                  setPage((p) => p + 1);
                }}
              >
                下一页
              </button>
            </div>
          </div>
        </section>
      </GatewayGate>
    </div>
  );
}
