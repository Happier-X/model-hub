import { useEffect, useState } from "react";
import { getPaths, type AppPaths } from "./api/tauri";
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

function PathsPanel({ paths, pathError }: { paths: AppPaths | null; pathError: string | null }) {
  return (
    <section className="mt-8 rounded-2xl border border-slate-200 bg-white p-6 shadow-sm">
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
            <div key={key} className="grid gap-1 px-4 py-3 md:grid-cols-[10rem_1fr] md:gap-4">
              <dt className="text-sm font-medium text-slate-600">{label}</dt>
              <dd className="break-all font-mono text-sm text-slate-900">{paths[key]}</dd>
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

export function App() {
  const [activeItem, setActiveItem] = useState<NavigationItem>("仪表盘");
  const [paths, setPaths] = useState<AppPaths | null>(null);
  const [pathError, setPathError] = useState<string | null>(null);

  useEffect(() => {
    getPaths()
      .then(setPaths)
      .catch((error: unknown) => {
        const detail = error instanceof Error ? error.message : String(error);
        setPathError(`无法读取数据目录：${detail}。请检查目录权限后重试。`);
      });
  }, []);

  return (
    <div className="flex min-h-screen bg-slate-100 text-slate-900">
      <Sidebar activeItem={activeItem} onNavigate={setActiveItem} />
      <div className="flex min-w-0 flex-1 flex-col">
        <StatusBar />
        <main className="flex-1 overflow-auto p-8">
          <div className="mx-auto max-w-5xl">
            <p className="text-sm font-medium text-cyan-700">{activeItem}</p>
            <h2 className="mt-1 text-3xl font-bold tracking-tight">
              欢迎使用 Model Hub
            </h2>
            <p className="mt-3 max-w-2xl text-sm leading-6 text-slate-600">
              桌面应用基础框架已就绪。渠道、分组、日志与网关管理能力将在后续版本中接入。
            </p>

            {activeItem === "设置" ? (
              <PathsPanel paths={paths} pathError={pathError} />
            ) : (
              <>
                <PathsPanel paths={paths} pathError={pathError} />
                <section className="mt-6 grid gap-4 md:grid-cols-3">
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
