import { useCallback, useEffect, useState } from "react";
import { clearLogs, formatLogTime, listLogs, type RelayLog } from "../api/log";
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

  const refresh = useCallback(async () => {
    if (!running || !authOk) {
      return;
    }
    setLoading(true);
    setError(null);
    try {
      setLogs(await listLogs(1, 40));
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  }, [running, authOk]);

  useEffect(() => {
    void refresh();
    if (!running || !authOk) {
      return;
    }
    const timer = window.setInterval(() => {
      void refresh();
    }, 5000);
    return () => window.clearInterval(timer);
  }, [refresh, running, authOk]);

  return (
    <div className="space-y-6">
      <div className="flex flex-wrap items-end justify-between gap-3">
        <div>
          <h2 className="text-2xl font-bold">日志</h2>
          <p className="mt-1 text-sm text-slate-600">轮询侧车请求日志列表（非 SSE）。</p>
        </div>
        <div className="flex gap-2">
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
            onClick={() =>
              void clearLogs()
                .then(refresh)
                .catch((err: unknown) => {
                  setError(err instanceof Error ? err.message : String(err));
                })
            }
          >
            清空
          </button>
        </div>
      </div>

      <GatewayGate running={running} authOk={authOk} authMessage={authMessage}>
        <section className="rounded-2xl border border-slate-200 bg-white p-6 shadow-sm">
          {error ? (
            <p role="alert" className="text-sm text-red-600">
              {error}
            </p>
          ) : null}
          {loading && logs.length === 0 ? (
            <p className="text-sm text-slate-500">加载中…</p>
          ) : logs.length === 0 ? (
            <p className="text-sm text-slate-500">暂无日志。</p>
          ) : (
            <div className="overflow-x-auto">
              <table className="min-w-full text-left text-sm">
                <thead className="border-b border-slate-200 text-slate-500">
                  <tr>
                    <th className="px-2 py-2 font-medium">时间</th>
                    <th className="px-2 py-2 font-medium">请求模型</th>
                    <th className="px-2 py-2 font-medium">渠道</th>
                    <th className="px-2 py-2 font-medium">Token</th>
                    <th className="px-2 py-2 font-medium">错误</th>
                  </tr>
                </thead>
                <tbody>
                  {logs.map((log) => (
                    <tr key={log.id} className="border-b border-slate-100 align-top">
                      <td className="px-2 py-2 whitespace-nowrap text-xs">
                        {formatLogTime(log.time)}
                      </td>
                      <td className="px-2 py-2 font-mono text-xs">
                        {log.request_model_name}
                      </td>
                      <td className="px-2 py-2 text-xs">{log.channel_name}</td>
                      <td className="px-2 py-2 text-xs">
                        {log.input_tokens}/{log.output_tokens}
                      </td>
                      <td className="max-w-xs px-2 py-2 text-xs text-red-600">
                        {log.error || "—"}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
        </section>
      </GatewayGate>
    </div>
  );
}
