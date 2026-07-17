const navigationItems = ["仪表盘", "渠道", "分组", "日志", "设置"] as const;

export type NavigationItem = (typeof navigationItems)[number];

interface SidebarProps {
  activeItem: NavigationItem;
  onNavigate: (item: NavigationItem) => void;
}

export function Sidebar({ activeItem, onNavigate }: SidebarProps) {
  return (
    <aside className="flex w-64 shrink-0 flex-col border-r border-slate-800 bg-slate-950 px-4 py-6 text-slate-100">
      <div className="mb-8 px-3">
        <p className="text-xs font-semibold tracking-[0.22em] text-cyan-400">
          本地模型网关
        </p>
        <h1 className="mt-2 text-2xl font-bold">Model Hub</h1>
      </div>

      <nav aria-label="主导航" className="space-y-1">
        {navigationItems.map((item) => (
          <button
            key={item}
            type="button"
            onClick={() => onNavigate(item)}
            aria-current={activeItem === item ? "page" : undefined}
            className={`w-full rounded-lg px-3 py-2.5 text-left text-sm font-medium transition ${
              activeItem === item
                ? "bg-cyan-500/15 text-cyan-300"
                : "text-slate-400 hover:bg-slate-900 hover:text-slate-100"
            }`}
          >
            {item}
          </button>
        ))}
      </nav>

      <div className="mt-auto rounded-xl border border-slate-800 bg-slate-900/70 p-3">
        <div className="flex items-center gap-2 text-sm font-medium">
          <span className="h-2.5 w-2.5 rounded-full bg-slate-500" />
          网关未集成
        </div>
        <p className="mt-1 text-xs text-slate-500">当前状态：空闲</p>
      </div>
    </aside>
  );
}
