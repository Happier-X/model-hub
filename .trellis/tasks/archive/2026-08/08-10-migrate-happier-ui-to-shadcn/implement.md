# 执行计划

## 阶段 0：备份与基线
- [ ] git 状态 clean 确认（无未提交）；`pnpm test:unit` + `pnpm build` 基线绿

## 阶段 1：基建
- [ ] `pnpm remove happier-ui`；`pnpm dlx shadcn-vue@latest init`（Tailwind 4 / vue / src/components/ui）
- [ ] `shadcn-vue add`：button card dialog input select switch checkbox progress spinner empty textarea badge tooltip pagination table sidebar item
- [ ] `pnpm add vue3-calendar-heatmap`
- [ ] `main.ts` 移除 happier-ui css 导入；确认 Tailwind 层顺序
- [ ] `src/components/ui/tag.vue`（如 shadcn 无 tag 时自写）

## 阶段 2：逐文件迁移（按依赖序）
- [ ] AppDialog.vue（Dialog 包装）
- [ ] AppTitleBar.vue（Button）
- [ ] AppShell.vue（Sidebar 全家桶 + Button）
- [ ] GroupCard.vue（Button + Card 若用到）
- [ ] GroupsPage.vue（Card/Button/Empty）
- [ ] SettingsPage.vue（Card/Input/Checkbox/Button/Progress）
- [ ] HomePage.vue（Card/Button/Badge + CalendarHeatmap 数据转换）
- [ ] LogsPage.vue（Table 结构 + Select + Pagination + Badge）
- [ ] ProvidersPage.vue（Table 结构 + Switch + 表单 + Pagination）
- [ ] GroupFormPage.vue（最重：Select/Card/Item/Input/Spinner/Empty/Button/Switch/Badge + 双栏高度链）

## 阶段 3：清理
- [ ] 全库 grep：`happier-ui` / `<H[A-Z]` / `h-` 组件类残留 → 清零
- [ ] 删除 `:deep(.h-card)` 等深选择器；确认自定义全局类保留

## 阶段 4：质量检查
- [ ] `pnpm typecheck` / `lint` / `test:unit` / `build` 全绿
- [ ] 手工验收 AC3 清单（表单/展开/拖拽/排序自动保存/筛选分页/热力图/对话框/侧边栏）
- [ ] 双栏高度链回归验证（分组表单页不滚动/占满）

## 阶段 5：spec 同步
- [ ] frontend/component-guidelines.md：happier-ui 约定 → shadcn-vue（Item/Table/Sidebar 用法、卡片高度链写法）
- [ ] frontend/directory-structure.md：新增 src/components/ui/ + lib/utils
- [ ] frontend/index.md 如有相关表述

## 阶段 6：收尾
- [ ] journal + archive + commit（可拆分：基建 / 迁移 / spec）
