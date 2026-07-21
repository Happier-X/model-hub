import { useCallback, useEffect, useMemo, useState, type FormEvent } from "react";
import { listChannels, type Channel } from "../api/channel";
import {
  createRoundRobinGroup,
  deleteGroup,
  groupModeLabel,
  listGroups,
  updateRoundRobinGroup,
  type Group,
  type GroupItem,
} from "../api/group";
import { GatewayGate } from "../components/GatewayGate";

interface GroupsPageProps {
  running: boolean;
}

interface EditDraft {
  name: string;
  channelId: number | "";
  modelName: string;
}

function primaryItem(group: Group): GroupItem | undefined {
  return group.items?.[0];
}

export function GroupsPage({ running }: GroupsPageProps) {
  const [groups, setGroups] = useState<Group[]>([]);
  const [channels, setChannels] = useState<Channel[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const [name, setName] = useState("my-model");
  const [channelId, setChannelId] = useState<number | "">("");
  const [modelName, setModelName] = useState("");
  const [editingId, setEditingId] = useState<number | null>(null);
  const [editDraft, setEditDraft] = useState<EditDraft | null>(null);
  const [editSaving, setEditSaving] = useState(false);

  const channelMap = useMemo(() => {
    const map = new Map<number, Channel>();
    for (const channel of channels) {
      map.set(channel.id, channel);
    }
    return map;
  }, [channels]);

  const channelLabel = (id: number): string => {
    const channel = channelMap.get(id);
    return channel ? `#${id} ${channel.name}` : `#${id}`;
  };

  const formatBindings = (group: Group): string => {
    const items = group.items ?? [];
    if (items.length === 0) {
      return "无绑定成员";
    }
    return items
      .map((item) => `${channelLabel(item.channel_id)} → ${item.model_name}`)
      .join("；");
  };

  const refresh = useCallback(async () => {
    if (!running) {
      return;
    }
    setLoading(true);
    setError(null);
    try {
      const [groupList, channelList] = await Promise.all([
        listGroups(),
        listChannels(),
      ]);
      setGroups(groupList);
      setChannels(channelList);
      setChannelId((current) => {
        if (current !== "" || !channelList[0]) {
          return current;
        }
        setModelName(channelList[0].model || "");
        return channelList[0].id;
      });
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  }, [running]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const onCreate = async (event: FormEvent) => {
    event.preventDefault();
    if (channelId === "") {
      setError("请选择渠道");
      return;
    }
    setSaving(true);
    setError(null);
    try {
      await createRoundRobinGroup({
        name,
        channelId,
        modelName,
      });
      await refresh();
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setSaving(false);
    }
  };

  const startEdit = (group: Group) => {
    if (group.id == null) {
      return;
    }
    const item = primaryItem(group);
    setEditingId(group.id);
    setEditDraft({
      name: group.name,
      channelId: item?.channel_id ?? (channels[0]?.id ?? ""),
      modelName: item?.model_name ?? channels[0]?.model ?? "",
    });
  };

  const cancelEdit = () => {
    setEditingId(null);
    setEditDraft(null);
  };

  const onSaveEdit = async (group: Group) => {
    if (!editDraft || group.id == null) {
      return;
    }
    if (editDraft.channelId === "") {
      setError("请选择渠道");
      return;
    }
    setEditSaving(true);
    setError(null);
    try {
      await updateRoundRobinGroup({
        id: group.id,
        name: editDraft.name,
        channelId: editDraft.channelId,
        modelName: editDraft.modelName,
        primaryItem: primaryItem(group) ?? null,
      });
      cancelEdit();
      await refresh();
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setEditSaving(false);
    }
  };

  const onDelete = async (group: Group) => {
    if (group.id == null) {
      return;
    }
    const ok = window.confirm(
      `确定删除分组「${group.name}」(#${group.id})？客户端将无法再用此 model 名。`,
    );
    if (!ok) {
      return;
    }
    setError(null);
    try {
      await deleteGroup(group.id);
      if (editingId === group.id) {
        cancelEdit();
      }
      await refresh();
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : String(err));
    }
  };

  return (
    <div className="space-y-6">
      <div>
        <h2 className="text-2xl font-bold">分组</h2>
        <p className="mt-1 text-sm text-slate-600">
          客户端请求中的{" "}
          <code className="rounded bg-slate-100 px-1">model</code> 填{" "}
          <strong>分组名</strong>；成员里的{" "}
          <code className="rounded bg-slate-100 px-1">model_name</code>{" "}
          是上游真实模型。默认负载：轮询（Round Robin）。
        </p>
      </div>

      <GatewayGate running={running}>
        <form
          onSubmit={(event) => void onCreate(event)}
          className="rounded-2xl border border-slate-200 bg-white p-6 shadow-sm"
        >
          <h3 className="text-lg font-semibold">新建分组</h3>
          <div className="mt-4 grid gap-4 md:grid-cols-2">
            <label className="block text-sm">
              <span className="font-medium text-slate-700">分组名（model）</span>
              <input
                className="mt-1 w-full rounded-lg border border-slate-300 px-3 py-2"
                value={name}
                onChange={(e) => setName(e.target.value)}
                required
              />
            </label>
            <label className="block text-sm">
              <span className="font-medium text-slate-700">渠道</span>
              <select
                className="mt-1 w-full rounded-lg border border-slate-300 px-3 py-2"
                value={channelId === "" ? "" : String(channelId)}
                onChange={(e) => {
                  const id = Number(e.target.value);
                  setChannelId(id);
                  const channel = channels.find((item) => item.id === id);
                  if (channel?.model) {
                    setModelName(channel.model);
                  }
                }}
                required
              >
                <option value="" disabled>
                  请选择
                </option>
                {channels.map((channel) => (
                  <option key={channel.id} value={channel.id}>
                    #{channel.id} {channel.name}
                  </option>
                ))}
              </select>
            </label>
            <label className="block text-sm md:col-span-2">
              <span className="font-medium text-slate-700">上游 model_name</span>
              <input
                className="mt-1 w-full rounded-lg border border-slate-300 px-3 py-2"
                value={modelName}
                onChange={(e) => setModelName(e.target.value)}
                required
              />
            </label>
          </div>
          <button
            type="submit"
            disabled={saving || channels.length === 0}
            className="mt-4 rounded-lg bg-cyan-600 px-4 py-2 text-sm font-medium text-white hover:bg-cyan-500 disabled:opacity-50"
          >
            {saving ? "创建中…" : "创建分组"}
          </button>
          {channels.length === 0 ? (
            <p className="mt-2 text-sm text-amber-700">
              请先在「渠道」页创建至少一个渠道。
            </p>
          ) : null}
        </form>

        <section className="rounded-2xl border border-slate-200 bg-white p-6 shadow-sm">
          <div className="flex items-center justify-between">
            <h3 className="text-lg font-semibold">分组列表</h3>
            <button
              type="button"
              className="rounded-lg border border-slate-200 px-3 py-1.5 text-sm"
              onClick={() => void refresh()}
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
          ) : groups.length === 0 ? (
            <p className="mt-4 text-sm text-slate-500">暂无分组。</p>
          ) : (
            <ul className="mt-4 divide-y divide-slate-100 rounded-xl border border-slate-200">
              {groups.map((group) => {
                const isEditing =
                  group.id != null && editingId === group.id && editDraft;
                return (
                  <li key={group.id ?? group.name} className="px-4 py-3">
                    <div className="flex flex-col gap-2 md:flex-row md:items-start md:justify-between">
                      <div className="min-w-0">
                        <p className="font-medium">
                          {group.name}{" "}
                          {group.id != null ? (
                            <span className="text-xs text-slate-500">#{group.id}</span>
                          ) : null}
                        </p>
                        <p className="mt-1 text-xs text-slate-500">
                          {groupModeLabel(group.mode)} · 成员{" "}
                          {group.items?.length ?? 0}
                        </p>
                        <p className="mt-1 break-all text-xs text-slate-600">
                          绑定：{formatBindings(group)}
                        </p>
                      </div>
                      <div className="flex flex-wrap gap-2">
                        {group.id != null ? (
                          <button
                            type="button"
                            className="rounded-lg border border-slate-200 px-3 py-1.5 text-sm"
                            onClick={() =>
                              isEditing ? cancelEdit() : startEdit(group)
                            }
                          >
                            {isEditing ? "收起" : "编辑"}
                          </button>
                        ) : null}
                        {group.id != null ? (
                          <button
                            type="button"
                            className="rounded-lg border border-red-200 bg-red-50 px-3 py-1.5 text-sm text-red-700"
                            onClick={() => void onDelete(group)}
                          >
                            删除
                          </button>
                        ) : null}
                      </div>
                    </div>

                    {isEditing && editDraft ? (
                      <div className="mt-4 rounded-xl border border-cyan-100 bg-cyan-50/40 p-4">
                        <p className="text-sm font-semibold text-slate-800">编辑分组</p>
                        <div className="mt-3 grid gap-3 md:grid-cols-2">
                          <label className="block text-sm">
                            <span className="font-medium text-slate-700">
                              分组名（客户端 model）
                            </span>
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
                            <span className="font-medium text-slate-700">绑定渠道</span>
                            <select
                              className="mt-1 w-full rounded-lg border border-slate-300 bg-white px-3 py-2"
                              value={
                                editDraft.channelId === ""
                                  ? ""
                                  : String(editDraft.channelId)
                              }
                              onChange={(e) => {
                                const id = Number(e.target.value);
                                const channel = channels.find((c) => c.id === id);
                                setEditDraft({
                                  ...editDraft,
                                  channelId: id,
                                  modelName: channel?.model || editDraft.modelName,
                                });
                              }}
                              required
                            >
                              <option value="" disabled>
                                请选择
                              </option>
                              {channels.map((channel) => (
                                <option key={channel.id} value={channel.id}>
                                  #{channel.id} {channel.name}
                                </option>
                              ))}
                            </select>
                          </label>
                          <label className="block text-sm md:col-span-2">
                            <span className="font-medium text-slate-700">
                              上游 model_name
                            </span>
                            <input
                              className="mt-1 w-full rounded-lg border border-slate-300 bg-white px-3 py-2"
                              value={editDraft.modelName}
                              onChange={(e) =>
                                setEditDraft({
                                  ...editDraft,
                                  modelName: e.target.value,
                                })
                              }
                              required
                            />
                          </label>
                        </div>
                        <div className="mt-3 flex flex-wrap gap-2">
                          <button
                            type="button"
                            disabled={editSaving || channels.length === 0}
                            className="rounded-lg bg-cyan-600 px-3 py-1.5 text-sm font-medium text-white hover:bg-cyan-500 disabled:opacity-50"
                            onClick={() => void onSaveEdit(group)}
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
