# 修复 GitHub Issue #3: 悬浮条切屏变小无法拖动

## Goal

修复 GitHub Issue [#3](https://github.com/Happier-X/model-hub/issues/3)：
桌面悬浮状态条从双屏切换为单屏时，出现变小、无法拖动的问题。

## 根因

1. 之前只在 `lib.rs` 的 `on_window_event` 中监听了 `ScaleFactorChanged` 和 `Resized` 事件来触发 `restore_overlay_geometry` 恢复几何信息。
2. 当两台显示器 **DPI 相同** 时，切屏后 Tauri 只触发 `Moved` 事件，不会触发 `ScaleFactorChanged` 或 `Resized`。
3. 缺少对 `Moved` 的处理 → 窗口位置/尺寸得不到恢复 → 变小、无法拖动。

## 范围内

- 在 `src-tauri/src/lib.rs` 的 overlay 事件分支中追加 `WindowEvent::Moved(_)` 监听。
- 三种事件统一调用已有 `overlay::restore_overlay_geometry` 函数恢复尺寸和位置。
- `restore_overlay_geometry` 已有防递归保护（`abs_diff > 1px` 才 `set_size`，位置 clamp 后变化才 `set_position`），无需改动。

## Out of Scope

- 不改 `overlay.rs` 的 `restore_overlay_geometry` 函数逻辑。
- 不改悬浮窗固定逻辑尺寸 `420 × 68` 等产品设计。
- 不改前端。

## Acceptance Criteria

- [x] 代码修改：`src-tauri/src/lib.rs` 追加 `WindowEvent::Moved(_)` 处理
- [x] `cargo check` 通过
- [x] `npm run typecheck` / `npm run lint` / `npm run test:unit` 通过
- [x] `cargo test` 通过
- [ ] 已提交并推送
- [ ] GitHub Issue #3 已关闭

## Notes

- 本次为轻量级修复，PRD-only 即可，无需 `design.md` 和 `implement.md`。
- 修改已在用户批准前完成并验证通过。
