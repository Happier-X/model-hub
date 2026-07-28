# 供应商页表格占满高度与底部分页

## Goal

供应商页面改为**无整页滚动**：表格恰好占满页面可用高度，只有表格 body（表头以下行）内部纵向滚动，
表格底部显示分页器（`HPagination`）。分页为**前端假分页**（对全量 `listProviders()` 返回的
`items` 做前端切片），不改后端。

## Background

- 当前 `src/pages/ProvidersPage.vue` 布局：
  ```
  div.space-y-6
  └─ HCard(variant="outlined" padding="md")
     ├─ #header: 标题"供应商" + 新建按钮
     ├─ p error 提示
     ├─ HEmpty(v-if items.length===0)
     └─ HTable(v-else, class="text-sm")
        └─ #cell 各列自定义渲染
  ```
- 无分页器；`refresh()`→`listProviders()` 全量加载到 `items`。后端 `list_providers` 无分页参数。
- AppShell 右主区结构：`<div class="min-h-0 flex-1 overflow-auto p-6"><RouterView /></div>`
  外层容器 `overflow-auto` + `p-6` 内边距。供应商页需在此容器内用全高度 flex 列布局，
  让表格区域吃掉剩余高度后内部滚动。

## Decisions

1. **前端假分页**：保持 `listProviders()` 全量加载，前端对 `items` 切片 + `HPagination`。
   不动后端 API。
2. **仅改供应商页布局**（方案 A）：不动 AppShell／其他页面，只调整 `ProvidersPage.vue`。
3. **保留 `p-6` 内边距**（A1）：表格与页面四周留 24px 间距，视觉一致。
4. **固定每页 10 条**，不加页大小选择器（数据量小、简单为上）。使用 `HPagination`
   默认 `page-size`（默认 10，无需显式传 prop 即可生效，但为可读性建议显式传）。
5. `sticky-header` prop 让 HTable 表头粘在滚动容器顶部（配合外层固定高度+`overflow-y:auto`）。

## Requirements

1. **无整页滚动**：供应商页根节点改为全高度 flex 列布局（`h-full flex flex-col`），
   外层 AppShell 的 `overflow-auto` 因子树高度精确填满而不产生页面级滚动条。
   为稳妥，根节点加 `overflow-hidden` 防范高度溢出导致的意外滚动。
2. **HCard 撑满高度**：`HCard` 从根节点吃掉所有剩余高度（`min-h-0 flex-1 flex flex-col`），
   内部按「header 固定 + 内容区（表格+分页器）flex-1 滚动」布局。
3. **表格 body 滚动**：`HTable` 的 `.h-table-wrapper` 或包裹容器设固定高度（flex-1 +
   min-h-0 + overflow-y:auto），配合 `sticky-header` prop 让表头固定、行区域滚动。
   `.h-table-wrapper` 本身是 `overflow-x: auto`，纵向滚动需在外层包裹容器实现。
4. **分页器在表格底部**：`HPagination` 放在表格滚动区域**之后**（不一起滚），
   紧贴表格下方。props：`:current` / `:total` / `:page-size` / `:disabled` +
   `@change` 翻页。`current` 使用页面级 `page` ref，`total` 为 `items.length`（全量数）。
5. **前端切片**：引入 `computed` 按 `page` 和 `pageSize=10` 从 `items` 切片出当前页数据
   `pagedItems`，传给 `HTable :data` 而非直接传 `items`。
6. **HEmpty 空态**：当 `items.length === 0` 时仍显示 `HEmpty`（不显示表格和分页器）。
   当分页切换后当前页无数据时（理论上全量数据 > 0 不会出现，但边界保护），显示空表格 +
   分页器。
7. 新建/编辑对话框、粘贴快速添加、Switch 行内启用开关等功能**完全不变**。
8. `refresh()` 保持全量 `listProviders()`，成功后重置 `page = 1`。
9. 顶部统计文本（可选）：HCard header 区域或 header 下方可加一行 "共 N 个供应商" 统计。
   不做强制要求，但若添加则样式一致（`text-sm text-slate-600`）。

## Out of Scope

- 后端 API 分页改造（`list_providers`）
- AppShell 结构改动（标题栏 / 侧栏 / 右主区容器）
- 页大小选择器（`HSelect` "每页"）
- 其他页面（首页 / 日志 / 设置）的布局
- 供应商新建/编辑/删除/开关功能逻辑

## Acceptance Criteria

- [ ] 供应商页不产生整页滚动（页面视口内只有表格 body 内部滚动条）
- [ ] 表格表头固定（滚动行区域时表头不移动）
- [ ] `HPagination` 出现在表格下方（不随表格 body 一起滚动）
- [ ] 换页功能正常：切片数据正确，最后一页含剩余行
- [ ] `listProviders()` 全量刷新后 `page` 重置为 1，当前页数据更新
- [ ] `HEmpty` 空态不受影响（items 为空时不显示表格和分页器）
- [ ] 新建/编辑供应商后列表刷新，分页重置
- [ ] 行内启用开关乐观更新+后端同步不受分页影响
- [ ] `npm run build` 通过
- [ ] `npm run lint` 通过

## Notes

- PRD-only 轻量级任务，无需 `design.md` / `implement.md`。前端假分页逻辑简单，直接实现即可。
- 校验命令：`npm run build`、`npm run lint`。
- 关键 CSS 模式：`h-full overflow-hidden`（根节点关滚动）+ `min-h-0 flex-1 flex flex-col`
  （卡片撑满）+ `min-h-0 flex-1 overflow-y-auto`（表格 wrapper 纵向滚动）+ `sticky-header`
  （表头固定）。
- 参考：日志页 `07-28-logs-pagination-bottom` 的分页器移至底部模式，但完全不同的滚动方案
  （日志页有服务端分页 + 主区整体滚动；供应商页是前端切片 + 表格内部滚动）。