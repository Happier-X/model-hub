# 技术设计：桌面悬浮状态条显示当前模型

## 概览

新增一个独立的无边框 Tauri WebView 窗口（label `overlay`）作为桌面悬浮状态条，类似桌面悬浮歌词：置顶、不抢焦点、不进任务栏，常驻主显示器右下角、任务栏上方。窗口内渲染最近成功请求的上游模型（复用现有 `get_last_success_request`），并根据代理状态（复用 `proxy_status`）区分“运行/停止/异常”。开关状态存入现有 `shell.json`，窗口位置用 Tauri 官方 `tauri-plugin-window-state` 持久化。

不改动请求路由、模型选择和故障转移逻辑；不新增“请求进行中”的实时状态。

## 边界与职责

| 组件 | 变更 | 说明 |
|------|------|------|
| 壳（Rust `lib.rs` / 新 `overlay.rs`） | 新增 | 创建/显示/隐藏 overlay 窗口；注册 window-state 插件；根据启动时开关决定是否显示 |
| 壳（`settings.rs`） | 扩展 | `ShellConfig` 增加 `overlay_enabled: bool`（`#[serde(default)]`，默认 false） |
| 壳（`commands.rs`） | 扩展 | 新增 `set_overlay_enabled`；`ShellPrefs` 增加 `overlay_enabled` 字段 |
| 壳（`tray.rs`） | 可选扩展 | 托盘菜单可加“显示/隐藏悬浮条”项（MVP 可不做，设置页开关已足够） |
| 前端（新 overlay 页面） | 新增 | 悬浮条 UI；轮询 `proxy_status` + `get_last_success_request`；处理拖动 |
| 前端（路由/入口） | 扩展 | 按窗口 label 或 URL hash 分流，overlay 窗口只渲染悬浮条，不加载主 SPA 布局 |
| 前端（`SettingsPage.vue`） | 扩展 | 增加“显示桌面悬浮条”开关，复用现有 HCheckbox + invoke 持久化模式 |
| 前端（`api/tauri.ts`） | 扩展 | 增加 `setOverlayEnabled`；`ShellPrefs` 类型加 `overlay_enabled` |

## 窗口方案

用 Rust 端 `WebviewWindowBuilder` 创建 overlay 窗口（不在 `tauri.conf.json` 静态声明，避免启动即显示、且便于按开关动态控制）：

- `.decorations(false)` 无边框
- `.always_on_top(true)` 置顶
- `.skip_taskbar(true)` 不进任务栏
- `.focused(false)` 创建时不抢焦点（tauri.conf.json 的 `focus:false` 在 Windows 下无效，必须走 builder，已查证 issue #11566）
- `.resizable(false)`、`.shadow(false)`、初始 `.inner_size(...)` 固定窄条尺寸
- `.visible(false)` 先建后按需 `show()`，避免闪现

窗口 URL 用同一前端产物，通过 URL（如 `index.html#/overlay` 或独立查询参数）区分。前端入口在 `main.ts` 判断当前窗口 label / hash，overlay 窗口挂载独立的 `OverlayApp`，不加载 `AppShell` 侧栏布局。

### 不抢焦点

- 创建用 `focused(false)`。
- 悬浮条内交互（拖动、点击打开主窗口）不应把 overlay 变成前台活动窗口而打断用户在其它应用的输入。拖动用无边框窗口拖拽（`data-tauri-drag-region` 或 `startDragging`），点击“打开主窗口”调用显示 main 窗口逻辑（复用 `tray::show_main_window` 的等价 IPC）。

## 位置持久化

引入 `tauri-plugin-window-state`（Windows 支持，官方维护）：

- 注册插件后 overlay 窗口在关闭/移动时自动保存位置与尺寸，重启后恢复。
- 首次无历史时，用代码计算主显示器工作区右下角、任务栏上方的默认坐标（`app.primary_monitor()` 取工作区尺寸减去窗口尺寸和内边距）。
- 主窗口 main 是否纳入插件状态需谨慎：main 已有 `tauri.conf.json` 尺寸约定，接入插件会改变现有行为。MVP 只对 overlay 应用位置恢复；如插件按全局 StateFlags 生效，需用 `with_denylist` 或按需 restore 仅 overlay，避免回归 main 窗口既有布局。

## 数据与刷新

overlay 前端定时（如每 2~3 秒）并在窗口显示时轮询：

- `proxy_status()` → 得到 `state`（idle/starting/running/stopping/error）与 `last_error`。
- `get_last_success_request()` → 得到 `group_name` / `provider_name` / `upstream_model` / `time`，可能为 null。

展示状态机（纯前端派生）：

| 代理状态 | 有最近成功 | 悬浮条展示 |
|----------|-----------|-----------|
| running | 有 | ● 上游模型（主）+ 分组/供应商/时间（次或悬停） |
| running | 无 | ○ 暂无模型（运行中，尚无成功请求） |
| stopping/idle | 任意 | ◌ 代理已停止（灰） |
| error | 任意 | ⚠ 代理异常（红）+ last_error 摘要 |

“最近成功模型”语义与首页“最近成功请求”一致，避免出现两处口径不一。不展示 API Key、请求内容等敏感字段。

## 生命周期与退出

- 应用启动 setup：读 `shell.json`，若 `overlay_enabled` 为 true 则创建并显示 overlay，否则不创建（或创建后隐藏）。
- 设置页开关：`set_overlay_enabled(true)` 时创建/显示 overlay；`false` 时隐藏（`hide()`）或关闭。为简化状态，推荐“存在即隐藏/显示”而非频繁 create/close。
- overlay 窗口的 CloseRequested 不应终止应用；与 main 一致，overlay 关闭只隐藏或视为关闭开关，真正退出仍只走托盘“退出”。
- 现有 main 窗口关闭隐藏到托盘、代理继续运行、托盘“退出”停代理的生命周期保持不变。

## 兼容性

- `ShellConfig` 新字段用 `#[serde(default)]`，旧 `shell.json` 无此字段时默认 false，不破坏现有配置读写和 `.bak` 回退逻辑。
- 不改数据库 schema，不改 `/v1/*` 契约，不改故障转移。
- 新增依赖 `tauri-plugin-window-state`（Rust + JS），按现有插件固定版本风格 pin 版本。

## 关键取舍

- **独立窗口 vs 系统级任务栏嵌入**：Windows 11 不允许普通应用向任务栏嵌入自定义文本，DeskBand 已弱化，故采用置顶无边框独立窗口模拟“状态条”。这是 PRD 已确认的方向。
- **Rust builder 创建 vs conf.json 声明**：选 builder，因为需要 `focused(false)` 在 Windows 生效，且要按开关动态控制显隐。
- **官方 window-state 插件 vs 自写持久化**：选官方插件，Windows 支持完善、维护活跃，减少自写坐标校验的越界风险；仅需注意不要连带改变 main 窗口既有行为。
- **轮询 vs 事件推送**：MVP 用轮询，贴合“只展示最近成功模型”，无需在转发层新增实时状态推送；后续要做“进行中态”再引入事件。

## 风险

- overlay 接入 window-state 可能连带影响 main 窗口位置/尺寸行为，需在实现时隔离（denylist 或仅对 overlay restore）。
- 全屏应用可能遮挡 overlay（MVP 不做全屏检测，PRD 已列范围外）。
- 多显示器/DPI 缩放下默认右下角坐标计算需用工作区而非全屏尺寸，避免被任务栏遮挡或跑出屏幕。
- overlay 窗口透明/圆角在 Windows 下可能需要额外配置（transparent + CSP），若做透明背景需一并验证。
