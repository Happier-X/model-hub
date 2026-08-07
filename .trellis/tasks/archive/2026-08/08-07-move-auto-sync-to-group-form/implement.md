# 执行计划

## 阶段 1：GroupFormPage 加开关
- [ ] 导入 `setProviderAutoSync`（如缺）+ `extractInvokeError`（如缺）
- [ ] 新增 `autoSyncTogglingIds` ref
- [ ] 新增 `toggleProviderAutoSync(p, next)`（乐观更新/IPC/回滚）
- [ ] HCell suffix 加 HSwitch + `@click.stop`，保留原数量/同步状态 span

## 阶段 2：ProvidersPage 移除
- [ ] 删除 `providerColumns` 的 auto_sync 列
- [ ] 删除列模板 auto_sync 分支 + `toggleProviderAutoSync` + `autoSyncTogglingIds`
- [ ] 删除 `setProviderAutoSync` 导入（grep 确认无残留引用）
- [ ] 保留 defaultFormValues.auto_sync: true 与提交透传

## 阶段 3：质量检查
- [ ] `pnpm typecheck` / `pnpm lint` / `pnpm test:unit` / `pnpm build` 全绿
- [ ] 手工核对 AC1-AC3（开关显示/供应商页无列/点击不展开）

## 阶段 4：spec + journal + 提交
- [ ] spec：frontend 组件/页面规范如有相关小节同步（grep auto_sync 相关 spec）
- [ ] journal 记录 + archive + commit
