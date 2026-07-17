import { useCallback, useEffect, useState, type FormEvent } from "react";
import {
  createOpenAiChatChannel,
  deleteChannel,
  listChannels,
  maskSecret,
  setChannelEnabled,
  type Channel,
} from "../api/channel";
import { GatewayGate } from "../components/GatewayGate";

interface ChannelsPageProps {
  running: boolean;
  authOk: boolean;
  authMessage: string;
}

export function ChannelsPage({ running, authOk, authMessage }: ChannelsPageProps) {
  const [channels, setChannels] = useState<Channel[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const [name, setName] = useState("openai-main");
  const [baseUrl, setBaseUrl] = useState("https://api.openai.com/v1");
  const [apiKey, setApiKey] = useState("");
  const [model, setModel] = useState("gpt-4o-mini");
  const [showKey, setShowKey] = useState(false);

  const refresh = useCallback(async () => {
    if (!running || !authOk) {
      return;
    }
    setLoading(true);
    setError(null);
    try {
      setChannels(await listChannels());
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  }, [running, authOk]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const onCreate = async (event: FormEvent) => {
    event.preventDefault();
    setSaving(true);
    setError(null);
    try {
      await createOpenAiChatChannel({ name, baseUrl, apiKey, model });
      setApiKey("");
      await refresh();
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="space-y-6">
      <div>
        <h2 className="text-2xl font-bold">渠道</h2>
        <p className="mt-1 text-sm text-slate-600">
          MVP 支持 OpenAI Chat 兼容渠道（类型 openai/chat_completions）。
        </p>
      </div>

      <GatewayGate running={running} authOk={authOk} authMessage={authMessage}>
        <form
          onSubmit={(event) => void onCreate(event)}
          className="rounded-2xl border border-slate-200 bg-white p-6 shadow-sm"
        >
          <h3 className="text-lg font-semibold">新建渠道</h3>
          <div className="mt-4 grid gap-4 md:grid-cols-2">
            <label className="block text-sm">
              <span className="font-medium text-slate-700">名称</span>
              <input
                className="mt-1 w-full rounded-lg border border-slate-300 px-3 py-2"
                value={name}
                onChange={(e) => setName(e.target.value)}
                required
              />
            </label>
            <label className="block text-sm">
              <span className="font-medium text-slate-700">上游模型名</span>
              <input
                className="mt-1 w-full rounded-lg border border-slate-300 px-3 py-2"
                value={model}
                onChange={(e) => setModel(e.target.value)}
                required
              />
            </label>
            <label className="block text-sm md:col-span-2">
              <span className="font-medium text-slate-700">Base URL</span>
              <input
                className="mt-1 w-full rounded-lg border border-slate-300 px-3 py-2 font-mono text-sm"
                value={baseUrl}
                onChange={(e) => setBaseUrl(e.target.value)}
                required
              />
            </label>
            <label className="block text-sm md:col-span-2">
              <span className="font-medium text-slate-700">上游 API Key</span>
              <div className="mt-1 flex gap-2">
                <input
                  type={showKey ? "text" : "password"}
                  className="w-full rounded-lg border border-slate-300 px-3 py-2 font-mono text-sm"
                  value={apiKey}
                  onChange={(e) => setApiKey(e.target.value)}
                  required
                  autoComplete="off"
                />
                <button
                  type="button"
                  className="rounded-lg border border-slate-200 px-3 text-sm"
                  onClick={() => setShowKey((v) => !v)}
                >
                  {showKey ? "隐藏" : "显示"}
                </button>
              </div>
            </label>
          </div>
          <button
            type="submit"
            disabled={saving}
            className="mt-4 rounded-lg bg-cyan-600 px-4 py-2 text-sm font-medium text-white hover:bg-cyan-500 disabled:opacity-50"
          >
            {saving ? "创建中…" : "创建渠道"}
          </button>
        </form>

        <section className="rounded-2xl border border-slate-200 bg-white p-6 shadow-sm">
          <div className="flex items-center justify-between gap-3">
            <h3 className="text-lg font-semibold">渠道列表</h3>
            <button
              type="button"
              onClick={() => void refresh()}
              className="rounded-lg border border-slate-200 px-3 py-1.5 text-sm"
            >
              刷新
            </button>
          </div>

          {error ? (
            <p role="alert" className="mt-4 text-sm text-red-600">
              {error}
            </p>
          ) : null}
          {loading ? (
            <p className="mt-4 text-sm text-slate-500">加载中…</p>
          ) : channels.length === 0 ? (
            <p className="mt-4 text-sm text-slate-500">暂无渠道，请先创建。</p>
          ) : (
            <ul className="mt-4 divide-y divide-slate-100 rounded-xl border border-slate-200">
              {channels.map((channel) => {
                const firstKey = channel.keys?.[0]?.channel_key;
                const firstUrl = channel.base_urls?.[0]?.url ?? "—";
                return (
                  <li
                    key={channel.id}
                    className="flex flex-col gap-3 px-4 py-3 md:flex-row md:items-center md:justify-between"
                  >
                    <div>
                      <p className="font-medium">
                        {channel.name}{" "}
                        <span className="text-xs text-slate-500">#{channel.id}</span>
                      </p>
                      <p className="mt-1 font-mono text-xs text-slate-500">{firstUrl}</p>
                      <p className="mt-1 text-xs text-slate-500">
                        模型 {channel.model} · Key {maskSecret(firstKey)} ·{" "}
                        {channel.enabled ? "启用" : "禁用"}
                      </p>
                    </div>
                    <div className="flex flex-wrap gap-2">
                      <button
                        type="button"
                        className="rounded-lg border border-slate-200 px-3 py-1.5 text-sm"
                        onClick={() =>
                          void setChannelEnabled(channel.id, !channel.enabled).then(refresh)
                        }
                      >
                        {channel.enabled ? "禁用" : "启用"}
                      </button>
                      <button
                        type="button"
                        className="rounded-lg border border-red-200 bg-red-50 px-3 py-1.5 text-sm text-red-700"
                        onClick={() =>
                          void deleteChannel(channel.id).then(refresh).catch((err: unknown) => {
                            setError(err instanceof Error ? err.message : String(err));
                          })
                        }
                      >
                        删除
                      </button>
                    </div>
                  </li>
                );
              })}
            </ul>
          )}
        </section>
      </GatewayGate>
    </div>
  );
}
