# 悬浮状态条（Overlay）几何恢复规范

## 背景
桌面悬浮状态条在双屏/单屏切换、DPI 变化时会因窗口尺寸/位置异常而缩小或拖动不可用。根因是原实现未处理 Tauri 窗口事件。

### 事件覆盖
- **`ScaleFactorChanged`**：跨不同 DPI 显示器时触发。
- **`Resized`**：窗口物理尺寸改变时触发。
- **`Moved`**：窗口位置改变时触发（**同 DPI 显示器切屏时仅有此事件**，原修复遗漏该事件导致同 DPI 双屏→单屏场景仍异常）。

三种事件统一调用 `restore_overlay_geometry` 恢复尺寸与位置。`restore_overlay_geometry` 内部有防递归保护（物理尺寸偏差 >1px 才 `set_size`，位置 clamp 后变化才 `set_position`）。

## 修复设计
- 新增 `restore_overlay_geometry(window: &tauri::Window)` 函数。
- 逻辑尺寸固定为 `420×68`。
- 计算期望物理尺寸：`(420 * scale).round() as u32`。
- 仅在实际尺寸偏差 > 1px 时调用 `set_size`。
- 读取当前位置并 clamp 到主显示器工作区，仅发生变化时调用 `set_position`。
- 在 `lib.rs` 的 overlay 事件分支监听 `ScaleFactorChanged`、`Resized`、**`Moved`** 三种事件，统一调用此函数。

## 验收标准
- 双屏切换后悬浮条恢复正确尺寸与位置。
- 正常尺寸/位置的 `Resized` 事件不重复调用。
- `cargo test` 通过。
- `npm run build` / `npm run lint` 通过。

## 验证结果
- Rust 单元测试：9/9 通过。
- `cargo check` / `cargo fmt` 通过。
- `npm run build` / `npm run lint` / `npm run test:unit`（16/16）通过。
- 首次提交：`fix(backend): 悬浮条切屏后恢复尺寸与位置 (#3)`
- 补充提交：追加 `Moved` 事件监听，覆盖同 DPI 切屏场景

## 相关文件
- `src-tauri/src/lib.rs` — `on_window_event` 中的 overlay 事件分支
- `src-tauri/src/overlay.rs` — `restore_overlay_geometry` 函数定义

## 代码位置

### `src-tauri/src/lib.rs` overlay 事件分支
```rust
// 监听变换/缩放/移动三事件：
// - ScaleFactorChanged: DPI 改变时（跨不同 DPI 显示器）
// - Resized: 窗口物理尺寸改变时
// - Moved: 窗口位置改变时（同 DPI 显示器切屏时仅有此事件）
WindowEvent::ScaleFactorChanged { .. }
| WindowEvent::Resized(_)
| WindowEvent::Moved(_) => {
    if let Err(err) = overlay::restore_overlay_geometry(window) {
        tracing::warn!(error = %err, "恢复悬浮状态条几何信息失败");
    }
}
```

### `src-tauri/src/overlay.rs` geometry restoration guard
```rust
// 防递归保护
if actual_size.width.abs_diff(expected_width) > 1
    || actual_size.height.abs_diff(expected_height) > 1
{
    window.set_size(LogicalSize::new(OVERLAY_WIDTH, OVERLAY_HEIGHT))?;
}

let clamped = clamp_to_primary_work_area(window.app_handle(), current)?;
if clamped != current {
    window.set_position(Position::Physical(clamped))?;
}
```
