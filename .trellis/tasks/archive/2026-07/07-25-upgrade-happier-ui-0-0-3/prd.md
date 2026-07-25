# 升级 happier-ui 0.0.3 并替换手写控件

## Goal

将 `happier-ui` 从 0.0.2 升级到 0.0.3，用 0.0.3 新增的 5 个组件（HSelect / HTable / HBadge / HTextarea / HPagination，正是本项目上一轮提给库的 issue）替换项目里对应的手写控件，缺功能则给 happier-ui 提 issue，并更新前端 spec 组件面约定。

## Scope

0.0.3 新增导出（对照上一轮提的 issue，5 个全部落地）：
- `HSelect` + `HSelectOption` ← 手写 `<select>`（5 处）
- `HTable` + `HTableColumn` + `HTableSort` ← 手写 `<table>`（2 处）
- `HBadge` / `HTag` ← 手写状态色标（2 处）
- `HTextarea` ← 手写 `<textarea>`（1 处）
- `HPagination` ← 手写「上一页/下一页」（1 处）

## Requirements

### 必做（升级）

- `happier-ui` 依赖升级到 `0.0.3`（package.json + lockfile）。
- CSS 入口名未变（`styles.css` + `tokens.css`），`main.ts` 无需改（已确认 0.0.3 无破坏性 CSS 改名）。

### 组件替换（能换都换）

- **HSelect（5 处）**：LogsPage 状态筛选 + 每页条数；GroupsPage 排序方式 + 批量供应商 + 队列行内供应商。选项转 `HSelectOption[]`（`{value,label}`）。
- **HTextarea（1 处）**：ProvidersPage 粘贴快速添加框（`rows` / `spellcheck` / 等宽字体）。
- **HBadge（2 处）**：HomePage 代理状态；LogsPage 状态码色标。颜色映射到 `variant`（success/warning/danger/default/info）。
- **HPagination（1 处）**：LogsPage 分页；可顺带合并每页条数选择（`showSizeChanger`），若合并会改变现有布局则保守只换翻页。
- **HTable（2 处，最高风险）**：LogsPage 8 列日志表（状态列彩标、错误列、故障转移多行）；ProvidersPage 4 列供应商表（操作列按钮）。复杂渲染用 `cell` slot 承接。独立回滚点。

### 缺功能 → 提 issue

- 替换过程中若发现 0.0.3 组件缺少必要能力（如 HTextarea 等宽字体透传、HSelect 占位语义、HTable cell slot 承接复杂渲染不足），先记录，能降级实现则降级，不能则给 happier-ui 提 issue 并在该处保留手写实现，注明原因。

### 明确不换

- **HTag**：项目无「可关闭标签」场景（当前状态展示更贴 HBadge），本轮不启用，列入 spec 理由。
- 沿用上轮不启用清单（HToast / HProgress / HCell / HFloatingBubble / HRange / HIconButton）除非本轮出现新契合场景。

### spec 更新

- 更新 `.trellis/spec/frontend/component-guidelines.md` 组件面：纳入 HSelect / HTable / HBadge / HTextarea / HPagination 为可映射组件，写明 HTag 不启用理由，以及任何因缺功能保留手写的位置。

## Non-Goals

- 不改后端 Rust。
- 不改 overlay 窗口构造与生命周期。
- 不为替换改变任何现有交互语义或可见文案。
- 不改 GroupsPage 拖拽排序交互（领域特定，非通用控件）。

## Acceptance Criteria

- [ ] `happier-ui` 安装版本为 0.0.3。
- [ ] 5 处 select 已替换为 HSelect，选项/占位/双向绑定行为不变。
- [ ] ProvidersPage 粘贴框已替换为 HTextarea（或记录缺功能保留原因）。
- [ ] HomePage / LogsPage 状态色标已替换为 HBadge，颜色语义一致。
- [ ] LogsPage 分页已替换为 HPagination，翻页/边界禁用行为不变。
- [ ] 2 处 table 已替换为 HTable（或记录缺功能回退原因），列渲染与交互不变。
- [ ] `pnpm lint`、`pnpm typecheck`、`pnpm build`、`pnpm test:unit` 通过；`cargo build` 通过。
- [ ] 前端 spec 组件面已更新，与实际替换一致。
- [ ] 若有缺功能，已在 happier-ui 提 issue 并在代码/spec 注明。

## Notes

- 组件 API（0.0.3 实测）：
  - HSelect：`options: HSelectOption[]` + `v-model`（string|number）+ `placeholder` / `clearable` / `size` / `ariaLabel`；emit `update:modelValue` + `change`；`option` slot 自定义渲染。
  - HTable：`columns: HTableColumn[]`（`key/title/width/align/sortable/render`）+ `data` + `rowKey` + `bordered/striped/stickyHeader/loading/emptyText`；slot `cell`(column,row,index) / `loading` / `empty`；emit `sort`。
  - HBadge：`variant: default/success/warning/danger/info` + `size: sm/md` + `dot` + `ariaLabel`；默认 slot 文本。
  - HTag：`variant` + `size` + `closable` + `disabled`；emit `close`。
  - HTextarea：`v-model` + `rows/spellcheck/resize/maxLength/showCount/size/label/description/error`；emit `update:modelValue`/`focus`/`blur`。
  - HPagination：`current/total/pageSize/showSizeChanger/pageSizeOptions/showTotal/simple`；emit `change`({current,pageSize}) / `update:current` / `update:pageSize`。
