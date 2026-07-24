# 技术设计：主窗口无边框 + 自定义标题栏

## 1. 边界与目标

- 只改主窗口 `main`：去系统边框，前端补自定义标题栏。
- overlay 悬浮窗完全不动（它已 `decorations(false)`，走独立入口与 capability）。
- 保持关窗语义不变：关闭 = `prevent_close` + `hide` 到托盘，代理继续；仅托盘「退出」真正停。

## 2. 涉及改动面

| 层 | 文件 | 改动 |
|----|------|------|
| Tauri 配置 | `src-tauri/tauri.conf.json` | 主窗口 window 增加 `"decorations": false` |
| 权限 | `src-tauri/capabilities/default.json` | 补窗口拖动/最小化/最大化切换/查询最大化/关闭权限 |
| 前端 API | `src/api/tauri.ts` | 不新增 invoke 命令；窗口控制走 `@tauri-apps/api/window` 的 `getCurrentWindow()` |
| 前端组件 | 新增 `src/components/AppTitleBar.vue` | 全宽标题栏：拖动区 + 三按钮 |
| 前端布局 | `src/components/AppShell.vue` | 顶部挂 `AppTitleBar`，整体改为「上标题栏 + 下(侧栏+主区)」结构 |

## 3. 窗口控制契约

前端直接用 `@tauri-apps/api/window`，不经 Rust 自定义命令（与 overlay 的 `getCurrentWindow().onMoved` 同源方案）：

```ts
import { getCurrentWindow } from "@tauri-apps/api/window";
const win = getCurrentWindow();
win.minimize();           // 最小化
win.toggleMaximize();     // 最大化/还原切换
win.close();              // 触发 CloseRequested → 后端拦截隐藏到托盘
win.isMaximized();        // 查询当前是否最大化（用于切换按钮图标）
win.onResized(cb);        // 监听尺寸变化，同步 isMaximized 状态
```

- 关闭按钮调用 `win.close()`，命中 `lib.rs` 现有 `main` 分支的 `CloseRequested` 拦截，语义天然复用，无需改 Rust 事件逻辑。
- 拖动用 DOM 属性 `data-tauri-drag-region`（需 `allow-start-dragging` 权限），与 overlay 一致；不用 JS 调 `startDragging`。
- 最大化状态：`onMounted` 读一次 `isMaximized()`，并 `onResized` 订阅更新；`onUnmounted` 清理 unlisten（遵循组合式函数副作用清理规范）。

## 4. capability 权限清单（default.json 追加）

```
core:window:allow-start-dragging
core:window:allow-minimize
core:window:allow-toggle-maximize
core:window:allow-unmaximize
core:window:allow-is-maximized
core:window:allow-close
```

- 只授予主窗口所需最小集；`allow-close` 仅触发关闭请求，实际隐藏仍由后端拦截控制，不放大退出风险。
- overlay capability 不动。

## 5. 布局与视觉

现有结构是 `flex`：左侧栏 + 右主区。改为纵向包裹：

```
<div class="flex min-h-screen flex-col">
  <AppTitleBar />                     <!-- 全宽，固定高度约 h-9/36px -->
  <div class="flex min-h-0 flex-1">   <!-- 原侧栏 + 主区 -->
    <aside>...</aside>
    <main>...</main>
  </div>
</div>
```

标题栏视觉：
- 深色底（与侧栏 `bg-slate-900` 呼应），高度约 36px。
- 左侧：可拖动区（`data-tauri-drag-region`），放一个轻量标识文字或留空；不重复页面标题。
- 右侧：三个窗口控制按钮（最小化 / 最大化-还原 / 关闭），按钮**不**加 `data-tauri-drag-region`（否则点击会被拖动吞掉）。
- 关闭按钮 hover 用红色系（`hover:bg-red-500`）区分危险操作；其余 hover 用中性灰。
- 图标用 `@lucide/vue`：最小化 `Minus`，最大化 `Square`，还原 `Copy`（或 `Minimize2`），关闭 `X`。
- 按钮含 `title` + `aria-label` 中文无障碍属性。

## 6. 风险与兼容

- 无边框后无法用系统边缘拉伸？Windows 下 `decorations:false` 仍保留 `resizable` 的系统命中区域（Tauri 默认行为），若实测不能拉伸再评估 `start-resize-dragging`，本期先不加。
- 多显示器/最大化图标同步：靠 `onResized` 覆盖，最大化/还原/系统快捷键都会触发 resize。
- overlay 无回归：改动不触及 overlay.rs / OverlayApp.vue / overlay capability。

## 7. 验证

- `pnpm lint`、`pnpm typecheck`。
- `cargo build`（capability JSON 由 schema 校验，错误标识符会编译期报错）。
- 手动：启动后无系统边框；拖动标题栏移动窗口；最小化；最大化/还原图标切换；关闭 → 隐藏到托盘且代理不停；overlay 正常。
