# 分组页与日志页无整页滚动 + 内容区内部滚动

## Goal

将供应商页（`07-28-provider-table-fill-pagination`）确立的「页面整体不滚动 + 内容区内部滚动」
模式推广到 **分组页**（`GroupsPage.vue`）与 **日志页**（`LogsPage.vue`）。

- **日志页**：套用同供应商页模式 — 页根节点 `h-full overflow-hidden`，日志表格区 `overflow-y-auto`
  + `sticky-header` 表头固定，`HPagination` 保留在表格下方（已在底部，搬迁到滚动区外），
  **保留服务端真分页不变**。筛选 HCard 留在顶部不滚区。
- **分组页**：不引入 HTable/HPagination，仅改为「无整页滚动」——根节点 `h-full overflow-hidden`，
  分组卡片列表包裹在 `overflow-y-auto` 区域内滚动，HCard header、错误、统计信息在固定区。
  保留卡片式布局。

## Background

- 供应商页 `ProvidersPage.vue` 刚完成同模式改造，Pattern 已写入 `spec/frontend/component-guidelines.md`
  「页面内部表格滚动模式」章节。
- **日志页**：当前 `div.space-y-6` → 筛选 HCard(多控件) → message/error p → 日志 HCard(统计行 +
  HTable + 底部 HPagination)。`HPagination` 在 `07-28-logs-pagination-bottom` 已移到表格下方。
  日志用服务端真分页：`listLogs(query)` 回 `LogPage { items, total }`。`storedTotal` 来自
  `log/{query}` 接口。保留所有筛选/自动刷新/清理/清空逻辑。
- **分组页**：当前 `div.space-y-6` → HCard(header 标题+新建按钮 → 说明 p → message/error p →
  HEmpty → `div v-for g in groups` 渲染分组卡片(名称/思考强度/自动同步/配置Pi/编辑/删除 +
  队列 `<ol><li>`))。分组数量通常不多（< 50），无表格、无分页。

## Decisions

**与供应商页一致的通用模式（仅对 HTable 页做 sticky-header + 分页）：**
1. 日志页：供应商页同款布局模式 + 服务端分页保留。
2. 分组页：仅「无整页滚动」+「分组列表区内部滚动」，不引入表格/分页。

## Requirements

### 日志页 (LogsPage.vue)

1. **无整页滚动**：根节点 `div.space-y-6` → `div.h-full.flex.flex-col.overflow-hidden`。
2. **筛选 HCard 固定**：第一块筛选卡片保持在根 flex 列中（自然渲染在上方，不参与滚动区）。
3. **message/error p 固定**：保持在筛选卡之后、列表卡之前，不参与滚动。
4. **日志 HCard 撑满剩余高度**：`class="min-h-0 flex-1 flex flex-col"` + scoped `:deep(.h-card){display:flex;flex-direction:column}` +
   `:deep(.h-card__body){flex:1;min-height:0;display:flex;flex-direction:column}`（与供应商页相同）。
5. **表格内部滚动**：统计行 `p.mb-3` 保留在 `.h-card__body` 顶部（不随表格滚），包裹 `HTable` 的
   `div` 设 `min-h-0 flex-1 overflow-y-auto` + `:sticky-header="true"`。
6. **HPagination 在滚动区外**：分页 `div.mt-3.flex.justify-end.shrink-0` 在 `overflow-y-auto` 之外。
7. 所有筛选控件、`goPage`/`onPageSizeChange`/`applyFilters`/`refresh`/自动刷新/清理/清空逻辑**完全不变**。
8. `listLogs` 分页参数（`page`/`page-size`/`status`/`group`）不变；`HPagination` props 不变。
9. 保留原有注释（happier-ui#9 双重断言注释等）。

### 分组页 (GroupsPage.vue)

1. **无整页滚动**：根节点 `div.space-y-6` → `div.h-full.flex.flex-col.overflow-hidden`。
2. **HCard 撑满剩余高度**：`class="min-h-0 flex-1 flex flex-col"` + scoped `:deep(.h-card){display:flex;flex-direction:column}` +
   `:deep(.h-card__body){flex:1;min-height:0;display:flex;flex-direction:column}`。
3. **分组卡片列表内部滚动**：说明 `p`、`message` p、`error` p、`HEmpty` 保留在 `.h-card__body` 顶部
   （不滚），分组卡片列表（`div v-for g in groups` 块）包裹在 `<div class="min-h-0 flex-1 overflow-y-auto">`
   内部滚动。
4. **不引入 HTable/HPagination**：卡片式布局、操作按钮、队列表项 `<ol><li>` 全部保留。
5. 新建/编辑/删除/配置到 Pi/拖拽排序/拉取模型/自动同步/Leaderboard 排序**完全不变**。
6. 保留所有原有注释。

## Out of Scope

- 分组页引入表格或分页（明确不做）
- 日志页分页逻辑改造（保留服务端真分页不变）
- AppShell 改动
- 后端 API 改动
- 其他页面（首页 / 设置）

## Acceptance Criteria

### 日志页
- [ ] 日志页不发生整页滚动（页面视口内仅表格 body 内部滚动）
- [ ] `HTable` 表头固定（滚动行时表头不动）
- [ ] `HPagination` 在表格下方（不随表格一起滚动）
- [ ] 筛选 HCard（分组名/状态/每页/筛选/刷新/自动刷新/清理/清空）功能正常
- [ ] 服务端分页翻页正常
- [ ] 空数据时 `HTable` 显示 empty-text（"暂无日志"）

### 分组页
- [ ] 分组页不发生整页滚动（页面视口内仅分组卡片列表内部滚动）
- [ ] HCard header（标题 + 新建按钮）固定不随滚动
- [ ] 说明 p、message/error p 固定在滚动区上方
- [ ] 分组卡片列表 `v-for` 在 `overflow-y-auto` 区域内滚动
- [ ] HEmpty 空态不变（分组列表为空时显示"暂无分组"）
- [ ] 新建/编辑/删除/配置到 Pi/排序等操作不受影响

### 共同
- [ ] `npm run build` 通过（无类型错误）
- [ ] `npm run lint` 通过

## Notes

- PRD-only 轻量级任务。两页布局模式与供应商页完全同款（spec 已有详细实现合同），
  实现时直接参照 `ProvidersPage.vue` 最终状态即可。
- 校验命令：`npm run build`、`npm run lint`。
- 日志页额外注意：筛选 HCard 有大量控件（分组 Select / 状态 Select / 每页 Select /
  筛选按钮 / 刷新 / 自动刷新 Checkbox + 时间输入 / 清理过期 / 清空全部），
  占据较大纵向空间，但与之前无区别——页面滚动区在表格卡内。
- 分组页的 `div v-for g in groups` 不是单个子元素根，需要在 `overflow-y-auto` 区域
  内包一层容器（`<div>` wrapping all group cards），否则每张卡片独立参与滚动。