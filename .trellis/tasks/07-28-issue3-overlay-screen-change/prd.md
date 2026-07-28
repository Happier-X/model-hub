# Issue #3: 悬浮条切屏后变小无法拖动

## Goal

修复 GitHub Issue [#3](https://github.com/Happier-X/model-hub/issues/3)：桌面悬浮状态条从双屏
切换为单屏后变小且无法拖动。

## 根因（研究已确认）

- `src-tauri/src/overlay.rs` 在创建时以逻辑尺寸 `420 × 68` 固定窗口尺寸，但未监听显示器/DPI
  变化后的窗口事件。
- Tauri 2 / Windows 在移动到不同 DPI 显示器、改变显示器缩放/分辨率、双屏拔除时，会派发
  `WindowEvent::ScaleFactorChanged` 和/或 `WindowEvent::Resized`。即使窗口不可 resizable，
  该事件也仍会发生。
- 项目原先只在创建、显示、保存位置时按 `primary_monitor()` clamp；屏幕拓扑变化当下不会重新
  校正尺寸和位置，因此会保留异常物理尺寸或旧坐标，最终表现为悬浮条缩小、拖动区域不可用。

## 修复设计

- 在 `src-tauri/src/overlay.rs` 新增公开的 overlay 几何恢复函数：
  1. 按逻辑尺寸 `420 × 68` 和当前窗口所在显示器 DPI 计算期望物理尺寸；
  2. 只有实际 `inner_size` 与期望尺寸相差超过 1px 时调用 `set_size(LogicalSize(420, 68))`，
     防止 `Resized → set_size → Resized` 无限递归；
  3. 读取当前位置并重新 clamp 到主显示器工作区，只有位置发生变化时才 `set_position`。
- 在 `src-tauri/src/lib.rs` 的已有 overlay `on_window_event` 分支处理
  `WindowEvent::ScaleFactorChanged` 与 `WindowEvent::Resized`，调用该恢复函数；保留原有
  `CloseRequested` 仅隐藏语义。

## Out of Scope

- 不改变悬浮窗固定逻辑尺寸 `420 × 68`、无边框、不可调整尺寸等产品设计。
- 不改前端 `data-tauri-drag-region` 结构（正常屏幕切换前拖动正常，非根因）。
- 不改用户手动保存悬浮窗位置的配置字段。
- 不新增多屏位置持久化策略（当前设计仍以主屏工作区为最终回退范围）。

## Acceptance Criteria

- [ ] 显示器/DPI 切换事件后，overlay 恢复 `420 × 68` 的逻辑尺寸
- [ ] 切换后 overlay 位置被限制在主显示器可见工作区，且可正常拖动
- [ ] 对已是正常尺寸/位置的 `Resized` 事件不重复 `set_size` / `set_position`
- [ ] `cargo test --manifest-path src-tauri/Cargo.toml` 通过
- [ ] `npm run build` / `npm run lint` 通过
- [ ] 已提交 `fix(backend): 悬浮条切屏后恢复尺寸与位置 (#3)`

## Notes

- Tauri 官方文档说明：不可 resize 的窗口仍会因 DPI scaling/全屏等触发 `Resized`；
  `ScaleFactorChanged` 会在移动到不同缩放显示器、改显示器分辨率/缩放时触发。
- `WebviewWindow::current_monitor()` 用于取当前窗口所在显示器的缩放比例；获取不到时退回主显示器，以保证恢复逻辑不因短暂拓扑重排失败。
- Keep `prd.md` focused on requirements, constraints, and acceptance criteria.
- Lightweight tasks can remain PRD-only.
- For complex tasks, add `design.md` for technical design and `implement.md` for execution planning before `task.py start`.