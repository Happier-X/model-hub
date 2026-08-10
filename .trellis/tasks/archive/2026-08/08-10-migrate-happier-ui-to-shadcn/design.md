# 设计：happier-ui → shadcn-vue 迁移

## 依赖与基建

```
pnpm remove happier-ui
pnpm dlx shadcn-vue@latest init        # Tailwind 4 + vite + src/components/ui + lib/utils (cn)
pnpm dlx shadcn-vue@latest add button card dialog input select switch checkbox progress
                                  spinner empty textarea badge tooltip pagination table sidebar item
pnpm add vue3-calendar-heatmap
```

- `main.ts`：删两行 happier-ui css 导入；保留 `./index.css`（含 `@import "tailwindcss"`）。
- shadcn init 会生成 `src/components/ui/*.vue` + `src/lib/utils.ts`（cn）。
- 热力图：`vue3-calendar-heatmap`，数据 `{ date: string('YYYY-MM-DD'), count: number }`；HomePage 现有 HHeatmapData `{ timestamp, value }` → computed 转换（timestamp → date 字符串）。

## 逐文件映射

### main.ts
删除 happier-ui css 导入。

### AppDialog.vue（包装层保留）
```
HDialog(v-model, close-on-overlay→? , close-on-esc→?) 
→ Dialog + DialogContent(@close? esc 默认) + DialogHeader + DialogTitle(插槽)
```
- shadcn Dialog 默认 overlay/esc 关闭；`closeDisabled` 时 `:close-on-esc="false"` `:close-on-overlay="false"`（reka-ui 支持）。
- HButton 关闭按钮 → Button variant ghost size sm。

### AppShell.vue
```
HSidebar(items, model-value, header 插槽)
→ SidebarProvider + Sidebar + SidebarHeader(品牌区) + SidebarContent
  + SidebarGroup + SidebarMenu(v-for navItems) + SidebarMenuItem
  + SidebarMenuButton(as-child → RouterLink :to)
```
- activeNavKey 逻辑保留（分组子路径高亮 → RouterLink active 类）。
- 折叠按钮：show-collapse-toggle=false 语义 → Sidebar 默认可折叠，可用 SidebarTrigger 或按需。

### GroupFormPage.vue（最重）
| H 组件 | shadcn | 备注 |
|---|---|---|
| HSelect | Select | thinkingEffortOptions → SelectItem；v-model 适配 |
| HCard | Card | `:deep(.h-card)` 高度链 → Card 根类（`h-full flex flex-col` 直接写类） |
| HCell | Item + ItemMedia + ItemContent | 供应商行（prefix chevron → ItemMedia icon / 自放），suffix HSwitch + 数量 span → Item 尾部自定义 |
| HInput | Input | leftFilter |
| HLoading | Spinner | 展开加载 |
| HEmpty | Empty | 无模型/无供应商 |
| HButton | Button | 各操作 |
| HSwitch | Switch | 自动同步 + thinking_effort? |
| HTag | Badge（或自写 tag.vue） | 队列分数/未匹配标签 |

### LogsPage.vue
```
HTable(columns/data/cell 插槽/sticky/loading/empty) 
→ Table + TableHeader(thead) + TableBody(v-for) + TableRow + TableHead + TableCell
  columns 配置驱动：v-for column → 单元格用 column.key 取值或定制
HSelect(级别筛选) → Select
HPagination → Pagination（v-slot page/items）
HBadge → Badge（statusCodeVariant 映射变体）
HCard/HButton → Card/Button
```

### ProvidersPage.vue
```
HTable(列式: name/base_url/enabled/actions) → Table 结构
HSwitch(enabled) → Switch
HInput/HTextarea/HCheckbox(表单) → Input/Textarea/Checkbox
HButton → Button；HPagination → Pagination；HDialog 已走 AppDialog
```

### HomePage.vue
```
HCard → Card；HButton → Button；HBadge → Badge
HHeatmap → CalendarHeatmap（vue3-calendar-heatmap；data 转换 date/count）
```

### GroupsPage / SettingsPage / GroupCard / AppTitleBar
- HCard→Card、HButton→Button、HEmpty→Empty、HCheckbox→Checkbox、HInput→Input、HProgress→Progress、HTextarea→Textarea。

## 样式策略

- 保留现有 Tailwind 布局类（flex 高度链、双栏、滚动区）——这是本迁移最大收益：不再有 `.h-card__body` 深选择器 hack，直接在 Card 上写 `class="flex min-h-0 flex-1 flex-col"`。
- `app-empty-compact` / `app-dialog-host` 等自定义全局类保留。
- shadcn 主题变量（--background 等 CSS vars 在 index.css / @theme 中）用默认即可，配色继续用现有 slate/cyan 工具类。

## 风险与对策

| 风险 | 对策 |
|---|---|
| shadcn Select v-model 差异 | 用 `:model-value` + `@update:model-value`（reka-ui 标准），不依赖 Vue 原生 v-model 语法糖 |
| Table 列式迁移量大 | LogsPage 5 列 + ProvidersPage 4 列，手写 Table 结构 + columns 数组驱动，先 Logs 后 Providers |
| 高度链回归（历史教训） | 每个页面迁移后立即 typecheck + 手工验证滚动；spec 更新双栏约定为 Card 类写法 |
| 热力图库数据格式 | HomePage computed 转换层集中处理，不散落 |
| shadcn init 网络/交互 | CLI 非交互参数；失败则手工拷入组件文件 + cn |

## 测试设计

- 前端：现有 unit 26 个不涉及组件库（utils 层），应全绿；typecheck/lint/build 为迁移正确性主闸。
- 手工清单（AC3）：分组新建/编辑（表单、展开、拖拽、排序、自动保存）、日志筛选分页、供应商增删改+分页、设置保存、首页统计/热力图/启停、AppShell 导航+折叠、AppDialog 打开关闭。
