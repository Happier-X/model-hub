import { useCallback, useEffect, useState, type FormEvent } from "react";
import {
  createApiKey,
  deleteApiKey,
  listApiKeys,
  maskSecret,
  updateApiKey,
  type ApiKey,
} from "../api/apikey";
import { GatewayGate } from "../components/GatewayGate";

interface ApiKeysPageProps {
  running: boolean;
  authOk: boolean;
  authMessage: string;
}

export function ApiKeysPage({ running, authOk, authMessage }: ApiKeysPageProps) {
  const [keys, setKeys] = useState<ApiKey[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const [name, setName] = useState("local-client");
  const [createdKey, setCreatedKey] = useState<string | null>(null);
  const [copyHint, setCopyHint] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    if (!running || !authOk) {
      return;
    }
    setLoading(true);
    setError(null);
    try {
      setKeys(await listApiKeys());
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
    setCopyHint(null);
    try {
      const created = await createApiKey({ name, enabled: true });
      const plain = created?.api_key ?? "";
      if (!plain) {
        throw new Error("创建成功但未返回 api_key，请刷新列表或检查侧车版本。");
      }
      setCreatedKey(plain);
      setName("local-client");
      await refresh();
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setSaving(false);
    }
  };

  const onCopy = async (value: string) => {
    try {
      await navigator.clipboard.writeText(value);
      setCopyHint("已复制到剪贴板");
    } catch {
      setCopyHint("复制失败，请手动选中复制");
    }
  };

  const onToggleEnabled = async (item: ApiKey) => {
    setError(null);
    try {
      await updateApiKey({ id: item.id, enabled: !item.enabled, name: item.name });
      await refresh();
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : String(err));
    }
  };

  return (
    <div className="space-y-6">
      <div>
        <h2 className="text-2xl font-bold">API 密钥</h2>
        <p className="mt-1 text-sm text-slate-600">
          管理客户端网关 API Key（前缀 <code className="rounded bg-slate-100 px-1">sk-modelhub-</code>
          ）。与设置页中的管理 JWT 不是同一套凭证。
        </p>
      </div>

      <GatewayGate running={running} authOk={authOk} authMessage={authMessage}>
        {createdKey ? (
          <section
            role="status"
            className="rounded-2xl border border-emerald-200 bg-emerald-50 p-6 shadow-sm"
          >
            <h3 className="text-lg font-semibold text-emerald-900">创建成功</h3>
            <p className="mt-2 text-sm text-emerald-800">
              完整 Key 仅在此展示一次，请立即复制并妥善保存。关闭后列表中仅显示脱敏值。
            </p>
            <div className="mt-4 flex flex-col gap-2 md:flex-row md:items-center">
              <code className="flex-1 break-all rounded-lg border border-emerald-200 bg-white px-3 py-2 font-mono text-sm text-slate-900">
                {createdKey}
              </code>
              <button
                type="button"
                className="rounded-lg bg-emerald-700 px-4 py-2 text-sm font-medium text-white hover:bg-emerald-600"
                onClick={() => void onCopy(createdKey)}
              >
                复制
              </button>
              <button
                type="button"
                className="rounded-lg border border-emerald-300 px-4 py-2 text-sm text-emerald-900"
                onClick={() => {
                  setCreatedKey(null);
                  setCopyHint(null);
                }}
              >
                关闭
              </button>
            </div>
            {copyHint ? (
              <p className="mt-2 text-sm text-emerald-800">{copyHint}</p>
            ) : null}
          </section>
        ) : null}

        <form
          onSubmit={(event) => void onCreate(event)}
          className="rounded-2xl border border-slate-200 bg-white p-6 shadow-sm"
        >
          <h3 className="text-lg font-semibold">新建 API 密钥</h3>
          <div className="mt-4 grid gap-4 md:grid-cols-2">
            <label className="block text-sm md:col-span-2">
              <span className="font-medium text-slate-700">名称</span>
              <input
                className="mt-1 w-full rounded-lg border border-slate-300 px-3 py-2"
                value={name}
                onChange={(e) => setName(e.target.value)}
                required
                placeholder="例如 local-client"
              />
            </label>
          </div>
          <button
            type="submit"
            disabled={saving}
            className="mt-4 rounded-lg bg-cyan-600 px-4 py-2 text-sm font-medium text-white hover:bg-cyan-500 disabled:opacity-50"
          >
            {saving ? "创建中…" : "创建密钥"}
          </button>
        </form>

        <section className="rounded-2xl border border-slate-200 bg-white p-6 shadow-sm">
          <div className="flex items-center justify-between gap-3">
            <h3 className="text-lg font-semibold">密钥列表</h3>
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
          ) : keys.length === 0 ? (
            <p className="mt-4 text-sm text-slate-500">暂无密钥，请先创建。</p>
          ) : (
            <ul className="mt-4 divide-y divide-slate-100 rounded-xl border border-slate-200">
              {keys.map((item) => (
                <li
                  key={item.id}
                  className="flex flex-col gap-3 px-4 py-3 md:flex-row md:items-center md:justify-between"
                >
                  <div>
                    <p className="font-medium">
                      {item.name}{" "}
                      <span className="text-xs text-slate-500">#{item.id}</span>
                    </p>
                    <p className="mt-1 font-mono text-xs text-slate-500">
                      {maskSecret(item.api_key)}
                    </p>
                    <p className="mt-1 text-xs text-slate-500">
                      {item.enabled ? "启用" : "禁用"}
                    </p>
                  </div>
                  <div className="flex flex-wrap gap-2">
                    <button
                      type="button"
                      className="rounded-lg border border-slate-200 px-3 py-1.5 text-sm"
                      onClick={() => void onToggleEnabled(item)}
                    >
                      {item.enabled ? "禁用" : "启用"}
                    </button>
                    <button
                      type="button"
                      className="rounded-lg border border-red-200 bg-red-50 px-3 py-1.5 text-sm text-red-700"
                      onClick={() =>
                        void deleteApiKey(item.id)
                          .then(refresh)
                          .catch((err: unknown) => {
                            setError(err instanceof Error ? err.message : String(err));
                          })
                      }
                    >
                      删除
                    </button>
                  </div>
                </li>
              ))}
            </ul>
          )}
        </section>
      </GatewayGate>
    </div>
  );
}
