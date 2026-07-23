# 技术设计

## 1. stop 可靠释放端口

当前：

```text
shutdown_tx.send → timeout(3s, join) → 超时则丢弃 handle（不 abort）
```

改为：

```text
shutdown_tx.send
timeout(GRACEFUL_STOP, join)
  Ok → 正常
  Err(Elapsed) → join.abort()；可选再短等或直接标 Idle
```

- 常量如 `PROXY_STOP_GRACE: Duration = 3s`（可微调）。
- `ProxyHandle::drop`：若 `live.is_some()` 则 `let _ = self.stop()`。
- `request_quit` / `RunEvent::Exit` 仍调用 stop（幂等）。

## 2. 单实例

使用官方 `tauri-plugin-single-instance`（桌面 target，与 Tauri 2.11 对齐）：

- 依赖：`tauri-plugin-single-instance`（cfg desktop）。
- `Builder` 上 `.plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| { show_main_window(app); }))`。
- **顺序**：单实例插件应在会 `proxy.start()` 的 setup **之前**注册；第二实例在插件层退出，不应再跑完整 setup 启代理（以插件行为为准；若第二实例仍进 setup，则禁止第二次 start）。
- 回调：显示主窗口 + unminimize + focus（复用 `tray::show_main_window`）。

## 3. 文案

| 位置 | 内容方向 |
|------|----------|
| 托盘 tooltip | 关闭窗口将隐藏到托盘，代理继续；托盘「退出」才停止代理并释放端口 |
| Overview | 简短说明关窗/退出差异（若已有端口说明可并列） |
| 自动改口 note | 可提示「若意外多开旧实例，请托盘退出旧进程」 |

## 4. 测试

- `stop`：单元/集成尽量覆盖「shutdown 后状态 Idle」；abort 路径可用 mock 或短超时难测则文档+手工验收。
- `AppExitState` 既有单测保留。
- 单实例：手工验收为主（CI 难开双 GUI）。

## 5. 兼容

- 关窗隐藏逻辑不变。
- 自动改口扫描保留；单实例减少「自己占自己」场景。
