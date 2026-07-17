import { useCallback, useEffect, useState, type FormEvent } from "react";
import { listChannels, type Channel } from "../api/channel";
import {
  createRoundRobinGroup,
  deleteGroup,
  listGroups,
  type Group,
} from "../api/group";
import { GatewayGate } from "../components/GatewayGate";

interface GroupsPageProps {
  running: boolean;
  authOk: boolean;
  authMessage: string;
}

export function GroupsPage({ running, authOk, authMessage }: GroupsPageProps) {
  const [groups, setGroups] = useState<Group[]>([]);
  const [channels, setChannels] = useState<Channel[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const [name, setName] = useState("my-model");
  const [channelId, setChannelId] = useState<number | "">("");
  const [modelName, setModelName] = useState("");

  const refresh = useCallback(async () => {
    if (!running || !authOk) {
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
  }, [running, authOk]);

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

  return (
    <div className="space-y-6">
      <div>
        <h2 className="text-2xl font-bold">分组</h2>
        <p className="mt-1 text-sm text-slate-600">
          分组名即客户端请求中的 <code className="rounded bg-slate-100 px-1">model</code>{" "}
          参数。默认负载模式：轮询（Round Robin）。
        </p>
      </div>

      <GatewayGate running={running} authOk={authOk} authMessage={authMessage}>
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
            <p className="mt-2 text-sm text-amber-700">请先在「渠道」页创建至少一个渠道。</p>
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
              {groups.map((group) => (
                <li
                  key={group.id ?? group.name}
                  className="flex flex-col gap-2 px-4 py-3 md:flex-row md:items-center md:justify-between"
                >
                  <div>
                    <p className="font-medium">{group.name}</p>
                    <p className="text-xs text-slate-500">
                      模式 {group.mode === 1 ? "轮询" : group.mode} · 成员{" "}
                      {group.items?.length ?? 0}
                    </p>
                  </div>
                  {group.id != null ? (
                    <button
                      type="button"
                      className="rounded-lg border border-red-200 bg-red-50 px-3 py-1.5 text-sm text-red-700"
                      onClick={() =>
                        void deleteGroup(group.id as number)
                          .then(refresh)
                          .catch((err: unknown) => {
                            setError(err instanceof Error ? err.message : String(err));
                          })
                      }
                    >
                      删除
                    </button>
                  ) : null}
                </li>
              ))}
            </ul>
          )}
        </section>
      </GatewayGate>
    </div>
  );
}
