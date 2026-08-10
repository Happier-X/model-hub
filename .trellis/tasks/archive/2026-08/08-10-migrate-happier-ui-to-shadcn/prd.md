# 组件库迁移：happier-ui → shadcn-vue

## Goal

将全部 `happier-ui`（私有组件库 0.1.1）组件替换为 **shadcn-vue**（v1，底层 reka-ui）+ 少量现成库，移除 happier-ui 依赖与样式，UI 形态与现有 Tailwind 4 样式体系一致。

## Background（已核实）

- 引用文件 11 个：`main.ts`、`AppDialog`、`AppShell`、`AppTitleBar`、`GroupCard`、`GroupFormPage`、`GroupsPage`、`HomePage`、`LogsPage`、`ProvidersPage`、`SettingsPage`。
- 使用组件 17 个：HButton / HCard / HCell / HCheckbox / HDialog / HEmpty / HHeatmap / HInput / HLoading / HPagination / HProgress / HSelect / HSidebar / HSwitch / HTable / HTag / HTextarea。
- 项目已装 Tailwind 4.3.3（`@tailwindcss/vite`）；shadcn-vue v1 组件为**源码拷入**（`src/components/ui/*.vue`），与 Tailwind 同源无样式冲突。
- shadcn-vue 组件清单核实：Table / Data Table / Pagination / Sidebar（Provider+Header+Content+Group+Trigger）/ Item（+ItemGroup+ItemMedia+ItemContent）/ Card / Dialog / Button / Input / Select / Switch / Checkbox / Progress / Spinner / Empty / Textarea / Badge / Tag 全可用。
- 热力图：`vue3-calendar-heatmap`（SVG GitHub 贡献图风格，MIT，npm）与 HomePage HHeatmap（365 天格子）形态一致；备选 vue3-apexcharts。
- 表单 `@tanstack/vue-form`、图标 `@lucide/vue` 保留。

## Decisions

| # | 决策 | 结论 |
|---|------|------|
| 1 | 组件来源 | shadcn-vue CLI `pnpm dlx shadcn-vue@latest add <name>` 拷入 `src/components/ui/`，源码可改 |
| 2 | 热力图 | `vue3-calendar-heatmap`（npm 依赖）；数据格式 `{ date, count }` 适配 HomePage 现有 HHeatmapData（timestamp/value） |
| 3 | HTable → | shadcn `Table`（手写 th/td 结构，改用现有 columns 配置驱动渲染）或 Data Table（tanstack）；LogsPage/ProvidersPage 两处表格，选 **Table + 现结构**（不引 tanstack，保持轻） |
| 4 | HPagination → | shadcn `Pagination`（v-slot page/items 模式）；两处 |
| 5 | HSidebar → | shadcn `Sidebar` 全家桶 + `SidebarMenu`/`SidebarMenuItem`/`SidebarMenuButton` 驱动 navItems |
| 6 | HCell → | shadcn `Item`（title/description/actions + ItemMedia）；GroupFormPage 供应商行 + 队列行 |
| 7 | HDialog → | shadcn `Dialog`（DialogContent + DialogHeader/DialogTitle，Teleport 内）；AppDialog 包装层保留 |
| 8 | HSelect → | shadcn `Select`（SelectTrigger/SelectContent/SelectItem；v-model 由 value/change 事件适配）；LogsPage/GroupFormPage |
| 9 | HSwitch/HCheckbox/HInput/HTextarea/HTag/HEmpty/HLoading/HProgress/HBadge → | shadcn 对应组件；HLoading→Spinner、HTag→自写（shadcn 无 tag，用 Badge 或自写小组件） |
| 10 | 样式 | 保留 Tailwind 布局类（flex/grid/高度链）；删除 happier-ui 的 tokens.css/styles.css 导入与 `:deep(.h-card)` 等迁移到 shadcn 对应类 |
| 11 | 热力图 colorScale | vue3-calendar-heatmap 默认 GitHub 绿阶，与现有风格可接受（HomePage 卡片内） |

## Requirements

### R1 基建
- 卸载 `happier-ui`；`pnpm dlx shadcn-vue@latest init`（Tailwind 4 适配）+ add 全部所需组件
- `main.ts` 移除 `happier-ui/tokens.css` / `styles.css` 导入（确认 Tailwind 层顺序不再依赖）
- 安装 `vue3-calendar-heatmap`；`src/components/ui/` 下新增 tag.vue（如 shadcn 无）

### R2 逐文件迁移（11 文件）
- 每个调用点：组件名 + props/事件签名按 shadcn-vue 文档适配（v-model → model-value/@update:model-value、variant/size 取值映射、slot 结构变化）
- 保留：路由、组合式函数、utils、纯逻辑、Tailwind 布局类、scoped 样式（删除 `.h-*` 深选择器）
- `app-empty-compact` 等自定义类保留

### R3 行为保持
- 功能零回归：分页/筛选、表单校验、供应商展开、队列拖拽/排序/自动保存、更新检查、日志渲染（Markdown html:false 不变）
- 双栏 flex 高度链（根 h-full → flex-1 → 滚动区）保持

### R4 验证
- `pnpm typecheck` / `lint` / `test:unit` / `build` 全绿
- 手工：5 个页面 + 分组新建/编辑 + 对话框 + 表格分页 + 热力图渲染

## Out of Scope

- 不改后端 / IPC / DB / 任何 .rs 与 utils 逻辑
- 不引入 tanstack-table（保持轻量，两处表格用 shadcn Table 手写结构即可）
- 不改设计语言（沿用现有 Tailwind 配色 slate/cyan 体系）
- 不重写 GroupCard / AppTitleBar 布局结构（仅换组件）

## Acceptance Criteria

- [ ] AC1：`package.json` 无 happier-ui；`main.ts` 无 happier-ui 样式导入
- [ ] AC2：全库无 `happier-ui` / `<H[A-Z]` / `.h-` 残留引用
- [ ] AC3：功能零回归（分页/筛选/表单/展开/拖拽/排序自动保存/更新检查/日志渲染）
- [ ] AC4：typecheck/lint/unit(26)/build 全绿
- [ ] AC5：热力图（HomePage）渲染正常，365 天格子 + loading
- [ ] AC6：spec 同步（component-guidelines / directory-structure 等前端 spec 中 happier-ui 表述）
