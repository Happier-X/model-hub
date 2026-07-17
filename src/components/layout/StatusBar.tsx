export function StatusBar() {
  return (
    <header className="flex h-16 items-center justify-between border-b border-slate-200 bg-white px-8">
      <div>
        <p className="text-sm font-semibold text-slate-900">本地管理控制台</p>
        <p className="text-xs text-slate-500">无需登录，配置仅保存在本机</p>
      </div>
      <div className="flex items-center gap-2 rounded-full border border-slate-200 bg-slate-50 px-3 py-1.5 text-xs font-medium text-slate-600">
        <span className="h-2 w-2 rounded-full bg-slate-400" />
        网关状态：未集成
      </div>
    </header>
  );
}
