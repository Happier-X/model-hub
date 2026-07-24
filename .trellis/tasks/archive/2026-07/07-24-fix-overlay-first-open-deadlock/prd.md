# 修复首次打开悬浮条卡死

## 目标

修复在 Windows 上首次从设置页开启桌面悬浮条时应用卡死（死锁）的问题，使悬浮条能正常创建并显示。

## 背景与根因

- 调用链：设置页开关 → `setOverlayEnabled(true)` → 后端同步命令 `set_overlay_enabled` → `overlay::set_overlay_visible(true)` → `ensure_overlay` → `create_overlay` → `WebviewWindowBuilder::build()`。
- Tauri 官方文档明确：在 Windows 上，`WebviewWindowBuilder::new`/`build` 在**同步命令**或事件处理器中调用会死锁（Webview2 issue，wry#583）。同步命令跑在主线程，而建窗需要主线程事件循环处理 WebView2 创建消息，主线程被命令自身阻塞导致死锁。
- 仅“首次”卡死：`ensure_overlay` 命中已存在窗口时直接返回，只有首次真正 `build()`。
- 启动路径 `restore_overlay_on_start` 不受影响：在 `setup` 钩子执行，事件循环尚未接管，是官方推荐的建窗时机之一。

## 需求

- `set_overlay_enabled` 命令改为 `async`，使其在独立线程执行，避免阻塞主线程事件循环，从而解除首次建窗死锁。
- 保持既有契约不变：先写 `shell.json`，再调 `set_overlay_visible`；显隐失败回滚 `overlay_enabled=previous` 并再次保存，失败仅告警。
- 保持 `ShellPrefs` 返回结构与字段不变；前端 `setOverlayEnabled` 调用方式不变。
- 保持启动恢复、拖动保存位置、关闭仅隐藏等既有行为不变。
- 在 overlay 规范中补充“建窗必须走 async 命令 / 独立线程，禁止在同步命令或事件处理器中建窗”的约束，防止回归。

## 验收标准

- [ ] Windows 上首次开启悬浮条不再卡死，悬浮条正常出现在主显示器右下角。（Windows 运行时手工验收）
- [ ] 关闭再开启、重复切换均正常，不卡死。（Windows 运行时手工验收）
- [x] `set_overlay_enabled` 为 `async fn`，内部保持“先写配置后显隐、失败回滚”的顺序。
- [x] 启动时 `overlay_enabled=true` 的恢复路径行为不变。
- [x] overlay 规范文档记录该 Windows 建窗死锁约束与正确做法。
- [x] `cargo fmt --check`、`cargo test`、`cargo check` 通过；前端 `build` 通过。
