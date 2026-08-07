# 分组表单页占满页高、整页不滚动

## Goal

编辑/新建分组页不整体滚动，双栏（可选模型 / 故障转移队列）撑满剩余页高，各自内部滚动。

## Background

- 当前 GroupFormPage 根容器 `flex flex-col gap-4` 无高度约束，内容多高就多高；AppShell 路由出口 `min-h-0 flex-1 overflow-auto p-6` 整页可滚动。
- 双栏 HCard 用 `max-h-[32rem]`：窗口高时只占 32rem 不撑满，窗口矮时页面整体滚动。
- 上一任务已修复 `.h-card__body` flex 链，双栏内部 `overflow-y-auto` 已可滚动。

## Requirements

1. 页面根容器改为 `h-full overflow-hidden`（自身不滚动），表单列布局 `flex-1 min-h-0` 传递高度
2. 双栏 grid 与左右 HCard 从 `max-h-[32rem]` 改为 `flex-1`，撑满剩余页高
3. 双栏内部滚动保留（overflow-y-auto 已有）；顶部说明、表单字段、底部按钮固定不滚动
4. 加载态 / 错误态不受影响

## Acceptance Criteria

- [ ] AC1：页面整体不产生滚动条，双栏恰巧占满可视高度
- [ ] AC2：窗口变高时双栏随之变高；内容超出时仅双栏内部滚动
- [ ] AC3：`pnpm typecheck` / `pnpm lint` / `pnpm build` 全绿
