# 主窗口改无边框并做自定义标题栏

## Goal

将主窗口（`main`）改为无系统边框（`decorations: false`），并在前端补一个自定义标题栏，恢复拖动窗口与最小化/关闭等窗口控制能力，同时保持现有「关闭 = 隐藏到托盘、代理继续」的语义。

## Requirements

- 主窗口去掉系统原生标题栏与边框（overlay 已是无边框，保持不变）。
- 前端提供自定义标题栏，形态为**全宽顶栏**（横跨侧栏与主区顶部），包含：
  - 可拖动区域（复用 `data-tauri-drag-region`）。
  - 窗口控制按钮：**最小化、最大化/还原、关闭**（关闭沿用「隐藏到托盘」语义）。
  - 最大化/还原按钮图标随窗口最大化状态同步切换。
- 标题栏不重复页面标题文字，仅承担「拖动 + 窗口控制」职责（品牌在侧栏、页面标题在主区 header）。
- 主窗口 capability（`capabilities/default.json`）补齐自定义标题栏所需权限（拖动 + 最小化 + 最大化/还原 + 查询最大化状态 + 关闭）。
- 标题栏文案与图标使用简体中文/无障碍属性，视觉与现有 happier-ui + Tailwind 风格一致。
- 不影响 overlay 悬浮窗现有行为。

## Decisions

- 标题栏形态：全宽顶栏（覆盖侧栏顶部品牌区那一行的高度）。
- 按钮集：最小化 + 最大化/还原 + 关闭三按钮。

## Acceptance Criteria

- [ ] 主窗口启动后无系统边框/标题栏。
- [ ] 自定义标题栏可拖动移动窗口。
- [ ] 最小化按钮可最小化窗口；关闭按钮触发关闭请求并隐藏到托盘（代理不停）。
- [ ] overlay 悬浮窗行为不变。
- [ ] `pnpm lint`、`pnpm typecheck` 通过；`cargo` 构建通过。

## Notes

- overlay 已用 `WebviewWindowBuilder ... .decorations(false)` 与 `data-tauri-drag-region`，主窗口自定义标题栏复用同款方案。
- 关窗语义见 `lib.rs` on_window_event：`main` 的 `CloseRequested` 在非退出态下 `prevent_close` + `hide`。
