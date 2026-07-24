# 主窗口无边框 + 自定义标题栏

> 主窗口（label `main`）去系统边框后，前端自定义标题栏的窗口控制契约。与 overlay（backend [desktop-overlay.md](../backend/desktop-overlay.md)）是两套独立窗口，不要混用。

---

## Scope

- 主窗口在 `tauri.conf.json` 里静态声明，`"decorations": false` 去掉系统标题栏与边框。
- 前端 `src/components/AppTitleBar.vue` 提供全宽标题栏，承担「拖动 + 窗口控制」职责，不重复页面标题文字。
- `AppShell.vue` 布局为「上标题栏 + 下(侧栏 + 主区)」纵向包裹。
- overlay 悬浮窗（动态建窗、独立 capability）不受影响。

---

## Window Control Contract

窗口控制走 `@tauri-apps/api/window` 的 `getCurrentWindow()`，**不新增 Rust 自定义命令**（与 overlay 的 `getCurrentWindow().onMoved` 同源方案）：

- `win.minimize()` 最小化。
- `win.toggleMaximize()` 最大化/还原切换。
- `win.close()` 触发 `CloseRequested`，命中 `lib.rs` 现有 `main` 分支拦截：非退出态下 `prevent_close` + `hide` 到托盘、代理继续。**不要**为关闭按钮另写「隐藏」逻辑，语义由后端统一拦截。
- `win.isMaximized()` 查询最大化态，用于切换按钮图标；`onMounted` 读一次 + `win.onResized(cb)` 订阅同步，`onUnmounted` 调用 unlisten 清理。
- 所有窗口调用 try/catch 静默失败，不阻塞 UI。

---

## Capability Contract

主窗口 capability `src-tauri/capabilities/default.json` 追加最小权限集（仅挂 `main`）：

```
core:window:allow-start-dragging
core:window:allow-minimize
core:window:allow-toggle-maximize
core:window:allow-unmaximize
core:window:allow-is-maximized
core:window:allow-close
```

- 只授予标题栏实际用到的权限；`allow-close` 仅触发关闭请求，实际隐藏由后端拦截控制，不放大退出风险。
- overlay capability（`overlay.json`）不动。

---

## 布局与视觉

- 标题栏深色底（`bg-slate-900`）与侧栏呼应，高度约 36px（`h-9`）。
- 左侧拖动区用 DOM 属性 `data-tauri-drag-region`（需 `allow-start-dragging`），与 overlay 一致；不用 JS 调 `startDragging`。
- 右侧三按钮：最小化 / 最大化-还原 / 关闭；**按钮本身不得加 `data-tauri-drag-region`**，否则点击会被拖动区吞掉。
- 图标用 `@lucide/vue`：最小化 `Minus`、最大化 `Square`、还原态 `Copy`、关闭 `X`；关闭按钮 hover 用红色系区分危险操作。
- 按钮含中文 `title` + `aria-label` 无障碍属性。

---

## Anti-Patterns

- 给主窗口重新加回系统 `decorations`（去边框后靠自定义标题栏，别回退）。
- 为窗口控制新写 Rust 自定义命令（`@tauri-apps/api/window` 已够用）。
- 关闭按钮里自行 `hide()` 或改退出语义（必须复用后端 `CloseRequested` 拦截）。
- 把拖动属性挂到控制按钮上导致点击失效。
- 把窗口权限授予 overlay 或把主窗口权限扩大到非标题栏所需集合。
