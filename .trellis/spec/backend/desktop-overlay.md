# Desktop Overlay Status Bar

> Windows 桌面悬浮状态条（label `overlay`）契约：窗口构造、IPC 命令、配置字段与生命周期。

---

## Scope

MVP 让用户无需打开主窗口即可看到「最近成功模型」状态。悬浮条：

- 独立 Tauri `WebviewWindow`（label `overlay`），不嵌入 Windows 任务栏。
- 默认关闭，由设置页开关控制，重启保持。
- 只展示「最近成功模型」，不引入进行中的实时模型状态。
- 主显示器工作区右下角为默认位置；可拖动并跨重启恢复。

---

## Signatures

### 后端命令

- `set_overlay_enabled(app, enabled: bool) -> ShellPrefs`（**必须 `async` 命令**：首次开启会走 `WebviewWindowBuilder::build()`，Windows 上在同步命令里建窗会死锁，async 命令由 Tauri 放到独立线程执行才能解锁）
- `save_overlay_position(app, x: i32, y: i32) -> ()`
- `show_main_window(app) -> ()`（复用 `tray::show_main_window`）
- `get_shell_prefs(app) -> ShellPrefs`（已扩展）
- `set_check_update_on_startup(app, enabled) -> ShellPrefs`（已扩展返回 `overlay_enabled`）

### 前端封装（`src/api/tauri.ts`）

- `setOverlayEnabled(enabled): Promise<ShellPrefs>`
- `saveOverlayPosition(x, y): Promise<void>`
- `showMainWindow(): Promise<void>`
- `ShellPrefs { gateway_port, check_update_on_startup, overlay_enabled }`

Tauri 2 参数用 camelCase：`{ enabled }` / `{ x, y }`，序列化字段仍 snake_case。

### 配置字段（`shell.json`）

- `overlay_enabled: bool`（`#[serde(default)]`，默认 false）
- `overlay_x: Option<i32>`（`#[serde(default)]`）
- `overlay_y: Option<i32>`（`#[serde(default)]`）

旧 `shell.json` 缺字段时必须回退到默认值，不得报错。

---

## Window Contract

overlay 窗口只在 Rust 端用 `WebviewWindowBuilder` 动态创建，不放进 `tauri.conf.json`：

- `.decorations(false)` `.shadow(false)` `.resizable(false)`
- `.always_on_top(true)` `.skip_taskbar(true)`
- `.focused(false)` **且** `.focusable(false)`（Windows 下缺一不可，才能保证不抢焦点）
- `.visible(false)` 先建后 `show()`，避免闪现
- URL：`WebviewUrl::App("index.html?overlay=1".into())`，前端按 `?overlay=1` 分流挂载 `OverlayApp`
- capability：新建 `src-tauri/capabilities/overlay.json`，`windows: ["overlay"]`，仅授予 `core:default`、`core:window:allow-outer-position`、`core:window:allow-start-dragging`，禁止授予 updater/process

### 位置计算

- `default_position`：主显示器 `monitor.work_area()` 已扣除任务栏；宽高须乘 `monitor.scale_factor()`，因为 `work_area` 是物理像素而 builder 尺寸是逻辑像素。
- `configured_position`：读取 `overlay_x/overlay_y`；缺失回退默认。
- `clamp_to_primary_work_area`：所有入口（拖动保存、显示前、启动恢复）都必须夹回主显示器可见工作区，避免历史坐标或 DPI 变化后落到屏幕外。

---

## Lifecycle Contract

- 启动：`overlay::restore_overlay_on_start` 在 `setup` 尾部执行；`overlay_enabled=true` 才创建并 `show()`。失败仅告警，不阻塞主流程。
- `set_overlay_enabled` **必须**：先写 `shell.json`；随后调 `set_overlay_visible`；如显隐失败，回滚 `overlay_enabled=previous` 并再次保存，失败仅告警。禁止先显隐后写配置（会出现「窗口起来但配置未开」）。
- `set_overlay_enabled` **必须是 `async fn`**（或在独立线程执行）：首次开启走 `ensure_overlay` → `create_overlay` → `WebviewWindowBuilder::build()`，Windows 上在**同步命令或窗口事件处理器**中建窗会死锁（Webview2，wry#583）——同步命令跑在主线程，而建窗需要主线程事件循环处理 WebView2 创建消息，主线程被命令自身阻塞导致死锁。async 命令被 Tauri 放到独立线程执行，主线程事件循环得以继续。函数体保持无跨 `await` 持有非 Send 值（当前纯同步体，只用 `AppHandle`，不持有 `State`）。
- `restore_overlay_on_start` 在 `setup` 钩子建窗**安全**：此时事件循环尚未接管，是官方推荐的建窗时机之一，无需 async。
- `set_overlay_visible(true)` 显示前必须重新 `clamp_to_primary_work_area(outer_position())` 校正位置。
- overlay 的 `CloseRequested`：`api.prevent_close()` + `window.hide()`；只有 `AppExitState::is_exiting()==true` 时才放行关闭。真正退出仍由托盘「退出」触发。
- 主窗口关闭仍隐藏到托盘、代理继续运行；托盘退出停止代理并关闭两个窗口。overlay 不影响单实例唤起路径。

---

## Data & Refresh Contract

overlay 前端不新增后端能力，全部复用现有 IPC：

- 每 2500ms 并行调 `proxyStatus()` + `getLastSuccessRequest()`。
- 派生状态：

| ProxyStatus.state | last_success | 展示 |
|-------------------|--------------|------|
| `running` / `starting` | 有 | 上游模型 + 分组/供应商/时间 |
| `running` / `starting` | 无 | 「暂无模型」 |
| `idle` / `stopping` | 任意 | 「代理已停止」 |
| `error` | 任意 | 「代理异常」+ `last_error` 摘要 |

- 请求失败保留上一次有效数据，只做非阻断提示（避免闪烁）。
- 不展示 API Key 或消息正文；`last_success` 只使用 `group_name` / `provider_name` / `upstream_model` / `time`。

---

## Validation & Error Matrix

| 条件 | 行为 |
|------|------|
| `shell.json` 缺 overlay 字段 | 视为 `overlay_enabled=false`、`overlay_x/y=None`；不报错 |
| `overlay_enabled=true` 但窗口创建失败 | 命令返回错误；`shell.json` 回滚到 `previous` |
| 拖动上报坐标越界或跨屏 | Rust 端 `clamp_to_primary_work_area` 夹回主屏工作区后再落库 |
| 主显示器信息不可用 | `AppError::Business("无法获取主显示器信息")`；启动路径仅告警不阻塞 |
| overlay 请求主窗口打开 | 走 `show_main_window`，不改变 overlay 显隐；不得让 overlay 变前台 |
| overlay CloseRequested 未退出 | `prevent_close` + `hide`；退出中才放行 |

---

## Good / Base / Bad Cases

- **Good**：开关开启后主显示器右下角出现悬浮条，成功请求后 3 秒内切换到最终成功上游模型，故障转移后展示最终成功候选项；重启后恢复到上次坐标。
- **Base**：无成功历史时显示「暂无模型」；代理停止时显示「代理已停止」；短暂网络失败时保留上次数据，右侧只出现细微提示。
- **Bad**（禁止）：先显隐后写配置；用 `position()+size()` 计算默认位置忽略 DPI；直接信任前端坐标落库；把 updater/process 权限授予 overlay；把 overlay 也纳入托盘退出以外的关闭路径。

---

## Tests Required

- `settings::tests::missing_overlay_fields_default_off`：旧 `shell.json` 加载后 `overlay_enabled=false`、坐标 `None`。
- `settings::tests::overlay_fields_roundtrip`：save→load 后开关与坐标一致。
- `settings::tests::save_port_preserves_check_update_flag` 保持通过（保证配置字段互不干扰）。
- 现有 `proxy::runtime` 单元测试保持通过（`ShellConfig` 初始化用 `..Default::default()`，避免新增字段破坏）。
- Windows 11 手工验收（自动化不可覆盖）：不抢焦点、100/125/150% DPI 无裁切、拖动后重启恢复位置、代理停止 / 异常 / 无历史三态可区分。

---

## Wrong vs Correct

### Wrong

```rust
// 先显隐后写配置：显隐失败会留下「窗口在但配置关」
crate::overlay::set_overlay_visible(&app, enabled)?;
cfg.overlay_enabled = enabled;
crate::settings::save_shell_config(config_dir, &cfg)?;
```

```rust
// 用 position()+size() 且忽略 DPI，会在 125/150% 缩放下偏移
let width = OVERLAY_WIDTH.round() as i32;
```

```rust
// 同步命令里建窗：Windows 首次开启死锁（wry#583）
#[tauri::command]
pub fn set_overlay_enabled(app: AppHandle, enabled: bool) -> Result<ShellPrefs, InvokeError> {
    // ... set_overlay_visible -> ensure_overlay -> WebviewWindowBuilder::build() 在主线程阻塞
}
```

### Correct

```rust
// 先写配置，显隐失败回滚配置
cfg.overlay_enabled = enabled;
crate::settings::save_shell_config(config_dir, &cfg)?;
if let Err(error) = crate::overlay::set_overlay_visible(&app, enabled) {
    cfg.overlay_enabled = previous;
    let _ = crate::settings::save_shell_config(config_dir, &cfg);
    return Err(error.into());
}
```

```rust
// work_area 是物理像素，须按 scale_factor 换算窗口尺寸
let scale = monitor.scale_factor();
let width = (OVERLAY_WIDTH * scale).round() as i32;
let height = (OVERLAY_HEIGHT * scale).round() as i32;
```

```rust
// async 命令里建窗：Tauri 放到独立线程执行，主线程事件循环继续处理 WebView2 创建消息
#[tauri::command]
pub async fn set_overlay_enabled(app: AppHandle, enabled: bool) -> Result<ShellPrefs, InvokeError> {
    // 先写配置，再 set_overlay_visible -> build()；失败回滚配置
}
```

---

## Anti-Patterns

- 把 overlay 静态声明进 `tauri.conf.json`：Windows 下 `focused:false` 不生效，且不便按开关动态控制显隐。
- 引入 `tauri-plugin-window-state` 全局管理所有窗口：会连带改变 `main` 既有位置与尺寸行为。
- 把打开主窗口/更新/进程等能力挂到 `overlay` capability：违反最小权限。
- overlay 中做 API Key、消息正文等敏感展示。
- 在 overlay 使用 `always_on_top` 之外的方式抢占任务栏空间（DeskBand 等已弱化）。
- 在同步 `#[tauri::command]` 或窗口事件处理器中创建 `WebviewWindow`（`WebviewWindowBuilder::build()`）：Windows 上会死锁（wry#583）。建窗只允许在 `setup` 钩子或 async 命令/独立线程中进行。
