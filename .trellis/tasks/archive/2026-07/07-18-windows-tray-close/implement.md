# 执行计划：Windows 托盘与关闭行为

## 清单

1. [x] 阅读 Tauri 2 tray/window API 与当前 crate
2. [x] 新增 `tray.rs`：显示/启动/停止/退出；托盘左键显示
3. [x] `lib.rs`：AppExitState；setup tray；CloseRequested hide；Exit stop gateway
4. [x] 图标与设置/网关文案
5. [x] Rust 单测 + `cargo fmt/test/check`
6. [x] `pnpm lint/build`
7. [x] 更新 docs/mvp-acceptance / gateway README 并勾选 AC

## 手工（Windows）

1. 启动应用/网关
2. 关主窗口 → 确认托盘仍在、端口仍监听
3. 托盘显示 → 恢复聚焦
4. 托盘退出 → 确认应用与托管网关结束
