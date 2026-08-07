# 修复分组表单页整页滚动（grid 布局下 flex-1 失效）

## Goal

分组表单页整页不再滚动：双栏（可选模型 / 故障转移队列）撑满剩余页高，内部各自滚动。

## Background

- 上一轮把双栏 HCard 的 `max-h-[32rem]` 改为 `flex-1`，但整页仍有滚动条。
- 根因：双栏容器是 `grid grid-cols-1 lg:grid-cols-2`，**HCard 作为 grid item，`flex-1`（flex 属性）不生效**——grid item 高度由内容决定，卡片无法压缩，内容撑高 → 外层 `overflow-auto` 产生滚动条。
- 对照：ProvidersPage 用 `h-full` 正常，证明 `h-full` 链本身没问题，问题只在 grid 容器。
- `.h-card__body` flex 链（:deep 样式）已就位，改对容器后内部滚动即可生效。

## Requirements

1. 双栏容器从 `grid grid-cols-1 gap-4 lg:grid-cols-2` 改为 flex 布局：`flex flex-col gap-4 lg:flex-row`（小屏纵向堆叠、大屏双列，响应式语义等价）
2. 左右 HCard 保持 `flex min-h-0 flex-1 flex-col`（改 flex 容器后 flex-1 生效）
3. 页面根 `h-full overflow-hidden`、表单 `flex-1 min-h-0` 链保留

## Acceptance Criteria

- [ ] AC1：整页无滚动条，双栏占满可视高度；内容超出时仅双栏内部滚动
- [ ] AC2：窗口缩放时双栏自适应（小屏堆叠、大屏双列）
- [ ] AC3：`pnpm typecheck` / `pnpm lint` / `pnpm build` 全绿
