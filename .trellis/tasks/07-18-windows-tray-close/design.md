# 设计：Windows 托盘与关闭行为

## 边界

| 层 | 职责 |
|----|------|
| Tauri 壳 | 托盘、菜单事件、窗口关闭拦截、真实退出 |
| gateway | 复用 `stop_managed`，不改进程策略 |
| 前端 | 仅设置/提示文案（可选） |

## Tauri 2 结构

新增 `src-tauri/src/tray.rs`（建议）：

```text
setup_tray(app)
  MenuItem: show, gateway_start/stop(可选), quit
  TrayIconBuilder::new()
    .icon(app.default_window_icon())
    .menu(...)
    .on_menu_event(...)
    .on_tray_icon_event(...)
```

主窗口关闭：

```text
Builder::on_window_event(|window, event|
  if main + CloseRequested:
    api.prevent_close()
    window.hide()
)
```

## 真退出状态

风险：托盘 `app.exit(0)` 可能先触发 CloseRequested；需要区分「用户真退出」。建议：

- `AppExitState(AtomicBool)` managed state。
- tray quit：`state.exiting=true; app.exit(0)`。
- `on_window_event`：仅 `!exiting` 时 prevent_close + hide。
- `RunEvent::Exit` 继续 `stop_managed`。

也可直接 tray quit 先 `stop_managed` 再 exit，但仍保留 RunEvent 幂等停止。

## 显示窗口

```text
app.get_webview_window("main")
  .show()
  .unminimize()
  .set_focus()
```

## 图标

优先 `app.default_window_icon().cloned()`；若 `None`，setup 返回可诊断错误。必要时在 `tauri.conf.json bundle.icon` 填 `icons/icon.ico`。

## 菜单

MVP：显示 / 启动网关 / 停止网关 / 退出。

- start/stop 通过 `GatewayHandle` + 现有路径调用；需要把内部函数重构为可从 tray 调用，或 MVP 只做显示/退出。PRD 要求「至少一个状态相关动作，或先显示/退出」；设计建议显示+启动+停止+退出。
- 启动失败不弹复杂对话框，可保留 runtime last_error，用户打开窗口可见。

## 回滚

- 移除 tray module、on_window_event、exit state；恢复直接关窗退出。
