# 日志页分页器移至表格底部

## Goal

把日志页日志卡片中位于日志表格**上方**的 `HPagination` 组件，移到 `HTable`
**下方**（表格底部），让翻页操作紧跟数据末尾，符合表格交互惯例。
顶部统计文字「筛选 N 条 · 库内 M 条 · 第 x/y 页」保留在表格上方，与分页器**拆开**。

## Background

- 当前 `src/pages/LogsPage.vue` 日志 HCard（L236-289）结构：
  ```
  HCard
  ├─ 顶部条（mb-3 flex justify-between gap-2）：左侧统计 span，右侧 HPagination   ← 分页器在上
  └─ HTable
  ```
- 统计 span 在 L240-241，`HPagination` 在 L242-247；`HTable` 在 L251-287。
- `HPagination` props：`current=page` / `total=total` / `page-size=pageSize` /
  `disabled=loading`，`@change="({ current }) => goPage(current)"`。
- `goPage` 在 L115-121：clamp 到 `[1, totalPages]`，相同页不重复请求（空列表例外），设 `page` 后触发 `refresh`。
- 页大小选择（`HSelect` "每页"，L198-207）已在上方筛选条，且规范明确每页条数选择
  保留独立 `HSelect` 不并入 `show-size-changer`（`spec/frontend/component-guidelines.md` 3.1），
  本任务不动它。
- 顶部统计 span 里的「第 {{ page }} / {{ totalPages }} 页」与底部分页器会有信息重叠，但
  属预期：顶部给概览，底部给操作；保留两者（拆开的语义）。

## Requirements

1. 把 `HPagination` 块从 `HTable` 上方的顶部条里移出，作为 `HTable` 之后的独立底部行，
   单独一行放在 HCard 的 HTable 之后。
2. 顶部的统计 span（「筛选 N 条 · 库内 M 条 · 第 x/y 页」）保留在 HTable 上方，
   作为独立的 `mb-3` 行（原顶部条左右两栏拆分后，统计 span 单独成行）。
3. `HPagination` 的所有 props / 事件 / 行为完全不变（`current`/`total`/`page-size`/
   `disabled`/`@change`）。
4. 底部分页行的对齐：默认右对齐（操作靠右，符合翻页惯例），用 Tailwind flex 布局实现
   （如 `<div class="mt-3 flex justify-end">`）；不引入新组件。
5. 顶部统计行保持原 `text-sm text-slate-600` 样式。
6. 不改筛选条（分组名 / 状态 / 每页 / 筛选 / 刷新 / 自动刷新 / 清理 / 清空）、
   不改 `goPage` / `onPageSizeChange` / `refresh` 等任何逻辑。
7. 保留原有注释（ happier-ui#9 双重断言注释等）。

## Out of Scope

- 筛选条布局与控件。
- 页大小 `HSelect` 的位置（仍在筛选条，不并入 `show-size-changer`）。
- `goPage` / 分页逻辑、 `onPageSizeChange`、`applyFilters`、自动刷新等逻辑。
- 统计文字内容（保留「筛选 N 条 · 库内 M 条 · 第 x/y 页」原文）。
- 其他页面（供应商 / 分组 / 首页）的分页或布局调整。

## Acceptance Criteria

- [ ] `HPagination` 出现在 `HTable` 下方（表格底部），而非上方。
- [ ] 顶部统计 span 保留在 `HTable` 上方，文案与样式不变。
- [ ] 分页器 props/事件不变，翻页仍走 `goPage`，功能正常。
- [ ] 底部分页行右对齐，与表格之间有合理间距（如 `mt-3`）。
- [ ] 筛选条控件、页大小选择、自动刷新、清理/清空等功能不受影响。
- [ ] `npm run build` 通过，无类型错误。
- [ ] `npm run lint` 通过。

## Notes

- 轻量级前端布局调整，仅 `src/pages/LogsPage.vue` 模板部分。PRD-only，
  无需 `design.md` / `implement.md`。
- 校验命令：`npm run build`、`npm run lint`。
- 决策记录：用户明确要求「拆开」——统计留在上、分页器移到下，二者分离。
- 顶部「第 x/y 页」与底部分页器的信息冗余属预期（概览 vs 操作）。
