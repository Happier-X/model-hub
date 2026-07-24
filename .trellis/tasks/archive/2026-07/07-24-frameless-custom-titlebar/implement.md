# 执行计划：主窗口无边框 + 自定义标题栏

## 有序清单

1. **Tauri 配置去边框**
   - `src-tauri/tauri.conf.json`：主窗口 window 对象加 `"decorations": false`。

2. **补窗口权限**
   - `src-tauri/capabilities/default.json` 的 `permissions` 追加：
     - `core:window:allow-start-dragging`
     - `core:window:allow-minimize`
     - `core:window:allow-toggle-maximize`
     - `core:window:allow-unmaximize`
     - `core:window:allow-is-maximized`
     - `core:window:allow-close`

3. **新增标题栏组件 `src/components/AppTitleBar.vue`**
   - `<script setup lang="ts">`：`getCurrentWindow()` 拿窗口句柄。
   - `isMaximized` 用 `ref<boolean>`；`onMounted` 读一次 `isMaximized()`，订阅 `onResized` 更新；`onUnmounted` 调用 unlisten 清理。
   - 方法：`minimize()` / `toggleMaximize()` / `close()`，各自 try/catch 静默失败（不阻塞 UI）。
   - 模板：全宽 flex 容器，左侧 `data-tauri-drag-region` 拖动区，右侧三按钮；按钮不带 drag 属性。
   - 图标 `@lucide/vue`：`Minus` / 最大化态 `Copy` 否则 `Square` / `X`；按钮含中文 `title` + `aria-label`。

4. **改布局 `src/components/AppShell.vue`**
   - 最外层由 `flex min-h-screen` 改为 `flex min-h-screen flex-col`。
   - 顶部插入 `<AppTitleBar />`。
   - 原「侧栏 + 主区」包一层 `<div class="flex min-h-0 flex-1">`。
   - 确认更新提示条、header、RouterView 滚动区仍在主区内且滚动正常。

5. **验证**
   - `pnpm lint`
   - `pnpm typecheck`
   - `cargo build`（在 `src-tauri/`）确认 capability schema 通过。
   - 若环境允许 `pnpm tauri dev` 手动核对；否则记录为需人工验收项。

## 验证命令

```powershell
pnpm lint
pnpm typecheck
cargo build --manifest-path src-tauri/Cargo.toml
```

## 审查门 / 回滚点

- 门 1：配置 + 权限（步骤 1-2）改完先 `cargo build`，确认权限标识符无误再写前端。
- 门 2：组件 + 布局（步骤 3-4）改完 `pnpm typecheck` + `pnpm lint`。
- 回滚：改动集中在 4 个文件 + 1 个新文件，`git checkout` 对应文件即可整体回退；无数据/迁移风险。

## 手动验收清单

- [ ] 启动主窗口无系统边框/标题栏。
- [ ] 拖动标题栏可移动窗口。
- [ ] 最小化按钮生效。
- [ ] 最大化/还原切换生效，图标同步。
- [ ] 关闭按钮隐藏到托盘，代理不停（托盘可再唤出）。
- [ ] overlay 悬浮窗行为不变。
