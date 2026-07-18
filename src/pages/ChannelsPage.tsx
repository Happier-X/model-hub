import { useCallback, useEffect, useState, type FormEvent } from "react";
import {
  channelTypeLabel,
  createOpenAiChatChannel,
  deleteChannel,
  listChannels,
  maskSecret,
  primaryChannelKey,
  setChannelEnabled,
  updateOpenAiChatChannel,
  type Channel,
} from "../api/channel";
import { GatewayGate } from "../components/GatewayGate";

interface ChannelsPageProps {
  running: boolean;
  authOk: boolean;
  authMessage: string;
}

interface EditDraft {
  name: string;
  baseUrl: string;
  model: string;
  apiKey: string;
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
  const [editingId, setEditingId] = useState<number | null>(null);
  const [editDraft, setEditDraft] = useState<EditDraft | null>(null);
  const [editSaving, setEditSaving] = useState(false);
  const [revealedKeyIds, setRevealedKeyIds] = useState<Set<number>>(() => new Set());
  const [copyHint, setCopyHint] = useState<string | null>(null);

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

  const startEdit = (channel: Channel) => {
    setEditingId(channel.id);
    setEditDraft({
      name: channel.name,
      baseUrl: channel.base_urls?.[0]?.url ?? "",
      model: channel.model,
      apiKey: "",
    });
    setCopyHint(null);
  };

  const cancelEdit = () => {
    setEditingId(null);
    setEditDraft(null);
  };

  const onSaveEdit = async (channel: Channel) => {
    if (!editDraft) {
      return;
    }
    setEditSaving(true);
    setError(null);
    try {
      const primary = primaryChannelKey(channel);
      await updateOpenAiChatChannel({
        id: channel.id,
        name: editDraft.name,
        baseUrl: editDraft.baseUrl,
        model: editDraft.model,
        apiKey: editDraft.apiKey,
        primaryKeyId: primary?.id,
      });
      cancelEdit();
      await refresh();
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setEditSaving(false);
    }
  };

  const toggleReveal = (id: number) => {
    setRevealedKeyIds((prev) => {
      const next = new Set(prev);
      if (next.has(id)) {
        next.delete(id);
      } else {
        next.add(id);
      }
      return next;
    });
  };

  const onCopyKey = async (value: string) => {
    try {
      await navigator.clipboard.writeText(value);
      setCopyHint("已复制上游 Key");
    } catch {
      setCopyHint("复制失败，请手动选中");
    }
  };

  const onDelete = async (channel: Channel) => {
    const ok = window.confirm(
      `确定删除渠道「${channel.name}」(#${channel.id})？此操作不可撤销。`,
    );
    if (!ok) {
      return;
    }
    setError(null);
    try {
      await deleteChannel(channel.id);
      if (editingId === channel.id) {
        cancelEdit();
      }
      await refresh();
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : String(err));
    }
  };

  const onToggleEnabled = async (channel: Channel) => {
    setError(null);
    try {
      await setChannelEnabled(channel.id, !channel.enabled);
      await refresh();
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : String(err));
    }
  };

  return (
    <div className="space-y-6">
      <div>
        <h2 className="text-2xl font-bold">渠道</h2>
        <p className="mt-1 text-sm text-slate-600">
          配置 OpenAI Chat 兼容上游。侧车 v0.9.28 渠道类型为数字枚举，创建时固定使用{" "}
          <code className="rounded bg-slate-100 px-1">type=0</code>（OpenAI Chat）；
          不要传字符串 type。
        </p>
      </div>

      <GatewayGate running={running} authOk={authOk} authMessage={authMessage}>
        <form
          onSubmit={(event) => void onCreate(event)}
          className="rounded-2xl border border-slate-200 bg-white p-6 shadow-sm"
        >
          <h3 className="text-lg font-semibold">新建渠道</h3>
          <p className="mt-1 text-xs text-slate-500">
            MVP 仅创建 OpenAI Chat（type=0）。单 Base URL + 单上游 Key。
          </p>
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
          {copyHint ? (
            <p className="mt-2 text-sm text-emerald-700" role="status">
              {copyHint}
            </p>
          ) : null}
          {loading ? (
            <p className="mt-4 text-sm text-slate-500">加载中…</p>
          ) : channels.length === 0 ? (
            <p className="mt-4 text-sm text-slate-500">暂无渠道，请先创建。</p>
          ) : (
            <ul className="mt-4 divide-y divide-slate-100 rounded-xl border border-slate-200">
              {channels.map((channel) => {
                const primary = primaryChannelKey(channel);
                const firstKey = primary?.channel_key;
                const firstUrl = channel.base_urls?.[0]?.url ?? "—";
                const revealed = revealedKeyIds.has(channel.id);
                const isEditing = editingId === channel.id && editDraft;

                return (
                  <li key={channel.id} className="px-4 py-3">
                    <div className="flex flex-col gap-3 md:flex-row md:items-start md:justify-between">
                      <div className="min-w-0">
                        <p className="font-medium">
                          {channel.name}{" "}
                          <span className="text-xs text-slate-500">#{channel.id}</span>
                        </p>
                        <p className="mt-1 break-all font-mono text-xs text-slate-500">
                          {firstUrl}
                        </p>
                        <p className="mt-1 text-xs text-slate-500">
                          {channelTypeLabel(channel.type)} · 模型 {channel.model} · Key{" "}
                          {revealed && firstKey ? firstKey : maskSecret(firstKey)} ·{" "}
                          {channel.enabled ? "启用" : "禁用"}
                        </p>
                      </div>
                      <div className="flex flex-wrap gap-2">
                        <button
                          type="button"
                          className="rounded-lg border border-slate-200 px-3 py-1.5 text-sm"
                          onClick={() =>
                            isEditing ? cancelEdit() : startEdit(channel)
                          }
                        >
                          {isEditing ? "收起" : "编辑"}
                        </button>
                        {firstKey ? (
                          <>
                            <button
                              type="button"
                              className="rounded-lg border border-slate-200 px-3 py-1.5 text-sm"
                              onClick={() => toggleReveal(channel.id)}
                            >
                              {revealed ? "隐藏 Key" : "显示 Key"}
                            </button>
                            {revealed ? (
                              <button
                                type="button"
                                className="rounded-lg border border-slate-200 px-3 py-1.5 text-sm"
                                onClick={() => void onCopyKey(firstKey)}
                              >
                                复制 Key
                              </button>
                            ) : null}
                          </>
                        ) : null}
                        <button
                          type="button"
                          className="rounded-lg border border-slate-200 px-3 py-1.5 text-sm"
                          onClick={() => void onToggleEnabled(channel)}
                        >
                          {channel.enabled ? "禁用" : "启用"}
                        </button>
                        <button
                          type="button"
                          className="rounded-lg border border-red-200 bg-red-50 px-3 py-1.5 text-sm text-red-700"
                          onClick={() => void onDelete(channel)}
                        >
                          删除
                        </button>
                      </div>
                    </div>

                    {isEditing && editDraft ? (
                      <div className="mt-4 rounded-xl border border-cyan-100 bg-cyan-50/40 p-4">
                        <p className="text-sm font-semibold text-slate-800">编辑渠道</p>
                        <div className="mt-3 grid gap-3 md:grid-cols-2">
                          <label className="block text-sm">
                            <span className="font-medium text-slate-700">名称</span>
                            <input
                              className="mt-1 w-full rounded-lg border border-slate-300 bg-white px-3 py-2"
                              value={editDraft.name}
                              onChange={(e) =>
                                setEditDraft({ ...editDraft, name: e.target.value })
                              }
                              required
                            />
                          </label>
                          <label className="block text-sm">
                            <span className="font-medium text-slate-700">上游模型名</span>
                            <input
                              className="mt-1 w-full rounded-lg border border-slate-300 bg-white px-3 py-2"
                              value={editDraft.model}
                              onChange={(e) =>
                                setEditDraft({ ...editDraft, model: e.target.value })
                              }
                              required
                            />
                          </label>
                          <label className="block text-sm md:col-span-2">
                            <span className="font-medium text-slate-700">Base URL</span>
                            <input
                              className="mt-1 w-full rounded-lg border border-slate-300 bg-white px-3 py-2 font-mono text-sm"
                              value={editDraft.baseUrl}
                              onChange={(e) =>
                                setEditDraft({ ...editDraft, baseUrl: e.target.value })
                              }
                              required
                            />
                          </label>
                          <label className="block text-sm md:col-span-2">
                            <span className="font-medium text-slate-700">
                              上游 API Key（留空则不修改）
                            </span>
                            <input
                              type="password"
                              className="mt-1 w-full rounded-lg border border-slate-300 bg-white px-3 py-2 font-mono text-sm"
                              value={editDraft.apiKey}
                              onChange={(e) =>
                                setEditDraft({ ...editDraft, apiKey: e.target.value })
                              }
                              placeholder="填写新 Key 以轮换"
                              autoComplete="off"
                            />
                          </label>
                        </div>
                        <div className="mt-3 flex flex-wrap gap-2">
                          <button
                            type="button"
                            disabled={editSaving}
                            className="rounded-lg bg-cyan-600 px-3 py-1.5 text-sm font-medium text-white hover:bg-cyan-500 disabled:opacity-50"
                            onClick={() => void onSaveEdit(channel)}
                          >
                            {editSaving ? "保存中…" : "保存"}
                          </button>
                          <button
                            type="button"
                            className="rounded-lg border border-slate-200 bg-white px-3 py-1.5 text-sm"
                            onClick={cancelEdit}
                            disabled={editSaving}
                          >
                            取消
                          </button>
                        </div>
                      </div>
                    ) : null}
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
