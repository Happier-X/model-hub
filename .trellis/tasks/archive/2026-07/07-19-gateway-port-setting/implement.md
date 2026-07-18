# 执行计划：网关端口设置

## 清单

1. [x] 新增 Rust 壳配置 `gateway/settings.rs`：默认、读取、安全替换/备份恢复、损坏回退与单测
2. [x] 扩展 `AppError`：配置读取/写入、非法端口、运行中不可修改
3. [x] `GatewayRuntime::set_port`：仅停止态可改，同步 port/base_url；单测
4. [x] `lib.rs` setup 从 shell 配置初始化端口
5. [x] 新增并注册 `gateway_set_port` invoke，确保持久化与内存状态一致
6. [x] `src/api/tauri.ts` 增加类型安全封装
7. [x] `src/App.tsx` 设置页增加端口表单、校验、禁用态和成功/错误反馈
8. [x] 修订端口占用提示和相关 README/spec 契约
9. [x] 运行 lint/build/fmt/test/check；自动化覆盖 18080 配置与环境变量传播
10. [ ] 更新 AC、spec、提交并归档

## 验证

```powershell
pnpm lint
pnpm build
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo test --manifest-path src-tauri/Cargo.toml
cargo check --manifest-path src-tauri/Cargo.toml
```

手工链路：

1. 停止网关。
2. 设置端口为一个空闲端口（如 18080）。
3. 启动网关。
4. 确认状态栏和 Base URL 为新端口，`gateway/data/config.json` 与环境覆盖一致。
5. 重启应用，确认端口仍保留。
6. 网关运行中尝试修改，确认被阻止且不会自动重启/杀进程。

## 审查门

- 默认仍为 `127.0.0.1:8080`
- 保存端口只允许停止态
- 无 localStorage 单真源
- 不自动选端口、不结束占用进程
- 前后端字段与错误码一致
