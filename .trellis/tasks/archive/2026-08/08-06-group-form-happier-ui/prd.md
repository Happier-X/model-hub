# 分组表单页样式改造：改用 happier-ui 组件

## Goal

`src/pages/GroupFormPage.vue` 中手写的容器、条目、标签、空态、加载态改用 happier-ui 组件承载，减少自写 Tailwind 结构，统一视觉与交互语义（hover / 焦点 / 键盘可达）。

## Background（已确认事实）

- happier-ui 0.1.1 共导出 30 个组件；项目当前用了 HBadge / HButton / HCard / HCheckbox / HDialog / HEmpty / HHeatmap / HInput / HPagination / HProgress / HSelect / HSidebar / HSwitch / HTable / HTextarea。
- 未启用且本次相关：**HTag**（spec 记「项目无可关闭标签场景」）、**HCell / HCellGroup**（spec 记「设置页复合行非标准列表项」）、**HLoading**、**HTooltip**、**HScrollbar**。
- `GroupFormPage.vue` 现有手写结构：双栏容器 `div.rounded-lg.border`、供应商手风琴 `button` + ChevronDown、队列项分数 `span.rounded-full`、删除 `×` 原生 `button`、3 处空态 `p.text-slate-400`、2 处加载态纯文字、编辑失败错误块 `div.border-rose-200`。
- 关键 API 约束：
  - `HCard`：`variant` outlined/filled/flat、`padding` none/sm/md/lg、`radius`、slots header/default/footer。
  - `HCell`：`title`/`description`/`clickable`/`showChevron`/`ariaLabel`，slots prefix/suffix，emit `click`。**`showChevron` 是固定右向箭头，无法旋转**。
  - `HEmpty`：`title` 必填、`description` 可选、slot icon/default；项目已有 `app-empty-compact` 类收紧默认 60vh。
  - `HTag`：`variant` default/primary/success/warning/danger、`size` sm/md、`closable`、emit `close`。
  - `HLoading`：`mode` local/global、`size`、`label`。

## Decisions

| 决策 | 结论 |
|------|------|
| 范围 | 仅 `GroupFormPage.vue` 样式/结构改造，不动业务逻辑与数据流 |
| HTag | **放开使用**（用户确认 A），做队列项分数标签，不使用 `closable` |
| HCell | **放开使用**（用户确认 A），做左栏供应商手风琴条目 |
| 手风琴箭头 | HCell `:show-chevron="false"`，在 `#prefix` slot 放自旋转 ChevronDown，保留「展开转 90°」的现有视觉 |
| 保留手写 | 拖拽手柄（`⋮⋮` 需 draggable + cursor-grab，库无对应件）、左栏模型按钮列表（需 font-mono + 紧凑密度，HButton 撑太大） |

## Requirements

1. **R1 双栏容器改 HCard**
   - 左「可选模型」、右「故障转移队列」外层容器改 `HCard variant="outlined" padding="none"`，标题行进 `#header`
   - 保留 `max-h-[32rem]` + 内部滚动区的高度约束与 flex 布局行为

2. **R2 供应商手风琴条目改 HCell**
   - `clickable`、`:show-chevron="false"`、`#prefix` 放可旋转 ChevronDown、`#suffix` 放模型数
   - 绑定态（`isBound`）禁用交互的表现保留
   - 键盘可达性由 HCell 提供（emit `click` 同时响应 Enter/Space）

3. **R3 分数标签改 HTag**
   - 匹配到榜单 → `variant="success"`；未匹配 → `variant="default"`；`size="sm"`
   - 保留现有 `title` 悬浮说明文案（分数 + 匹配层级）

4. **R4 空态改 HEmpty**
   - 3 处：无供应商、上游未返回模型、队列为空（含绑定态文案分支）
   - 配 `app-empty-compact` 避免撑高；文案沿用现有语义

5. **R5 加载态改 HLoading**
   - 2 处：编辑页「正在加载分组…」、左栏「正在拉取模型…」
   - `mode="local"`，label 沿用现有文案

6. **R6 删除按钮与错误块**
   - 队列项删除 `×` 改 `HButton variant="ghost" size="sm"`（保留 rose 语义色与 title）
   - 编辑失败错误块改 `HCard variant="outlined"`，保留 rose 语义配色与「返回列表」入口

7. **R7 spec 同步**
   - `component-guidelines.md` 3.1：HTag / HCell 从「本轮不启用」移到已启用映射，写清适用形态与 `showChevron` 限制；登记 HLoading 用法

## Out of Scope

- 不改业务逻辑、数据流、表单提交与校验（TanStack Form 结构不动）
- 不改拖拽排序实现与左栏模型按钮列表
- 不改其他页面（本次仅 GroupFormPage）
- 不引入新依赖

## Acceptance Criteria

- [ ] AC1：`pnpm typecheck` / `pnpm lint` / `pnpm test:unit` / `pnpm build` 全绿
- [ ] AC2：R1-R6 六类替换全部落地，页面无残留等价手写结构
- [ ] AC3：交互行为不回归——手风琴展开/收起、箭头旋转、点击加模型、全部加入、拖拽排序、删除、清空、排序、立即同步、保存/取消/返回均与改造前一致
- [ ] AC4：绑定态（`isBound`）只读约束仍生效（手风琴禁用、队列不可改）
- [ ] AC5：编辑页加载中/加载失败/非法 id 三种状态展示正常
- [ ] AC6：spec 已同步 HTag / HCell / HLoading 的启用状态与限制

## Notes

- 复杂任务：需补 `design.md`（组件映射与 slot 结构）+ `implement.md`（逐项替换顺序与验证点）后再 `task.py start`。
- 视觉回归无自动化手段，AC3 依赖实现后人工核对 + 用户确认。
