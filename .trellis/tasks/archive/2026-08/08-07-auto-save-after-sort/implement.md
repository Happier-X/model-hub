# 执行计划

## 阶段 1：payload 复用
- [ ] 新增 `buildGroupPayload(value)`，onSubmit 改用之（行为不变）

## 阶段 2：autoSaveAfterSort
- [ ] 新增 `autoSaveAfterSort()`：saving 防重入 → updateGroup → formMessage「已保存…」/ error
- [ ] `sortQueueByCapability` 尾部：applySortedItems 后按 isEditing 分支（自动保存 vs 保存后生效提示）
- [ ] 核对 applySortedItems 内部 formMessage 赋值，避免与最终提示竞争（最终赋值覆盖）

## 阶段 3：质量检查
- [ ] `pnpm typecheck` / `pnpm lint` / `pnpm test:unit` / `pnpm build` 全绿
- [ ] 手工核对 AC1-AC4

## 阶段 4：收尾
- [ ] journal + archive + commit
