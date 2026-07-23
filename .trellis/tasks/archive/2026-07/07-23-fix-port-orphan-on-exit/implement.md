# 实施计划

1. `runtime.rs`：`stop` 超时 abort；`Drop` for `ProxyHandle` best-effort stop；必要时提取常量。
2. 接入 `tauri-plugin-single-instance`：Cargo.toml + `lib.rs` 初始化 + 第二实例 show 窗口。
3. 文案：`tray` tooltip、Overview 提示。
4. 单测：stop/Drop 相关可测部分；`cargo test`。
5. 更新 `.trellis/spec/backend` 中代理生命周期/错误处理相关约定。
6. `pnpm typecheck` / `pnpm lint`（若改前端文案）。

## 验证

```powershell
cd src-tauri; cargo test
pnpm typecheck
pnpm lint
```

手工：启动 → 关窗 → 端口仍监听 → 托盘退出 → 端口释放；再开第二实例应聚焦第一实例。
