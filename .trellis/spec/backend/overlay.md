# 悬浮状态条（Overlay）几何恢复规范

## 背景
桌面悬浮状态条在双屏/单屏切换、DPI 变化时会因窗口尺寸/位置异常而缩小或拖动不可用。根因是原实现未处理 Tauri `WindowEvent::ScaleFactorChanged` 和 `WindowEvent::Resized` 事件。

## 修复设计
- 新增 `restore_overlay_geometry(window: &tauri::Window)` 函数。
- 逻辑尺寸固定为 `420×68`。
- 计算期望物理尺寸：`(420 * scale).round() as u32`。
- 仅在实际尺寸偏差 > 1px 时调用 `set_size`。
- 读取当前位置并 clamp 到主显示器工作区，仅发生变化时调用 `set_position`。
- 在 `lib.rs` 的 overlay 事件分支调用此函数。

## 验收标准
- 双屏切换后悬浮条恢复正确尺寸与位置。
- 正常尺寸/位置的 `Resized` 事件不重复调用。
- `cargo test` 通过。
- `npm run build` / `npm run lint` 通过。

## 验证结果
- Rust 单元测试：109/109 通过。
- `cargo check` / `cargo fmt` 通过。
- `npm run build` / `npm run lint` / `npm run test:unit` 通过。
- 提交：`fix(backend): 悬浮条切屏后恢复尺寸与位置 (#3)`。

## 相关文件
- `src-tauri/src/lib.rs`
- `src-tauri/src/overlay.rs`
