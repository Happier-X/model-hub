# 执行计划：Rust 网关 HTTP 服务骨架

## 清单

1. [x] 创建独立 `gateway-rust` crate，钉扎 axum/tokio/clap/serde/tracing/thiserror 依赖
2. [x] 配置模块：读取 `data/config.json`、默认值、IP/端口校验、可行动错误与单测
3. [x] HTTP 模块：`AppState`、`GET /health`、JSON 404 fallback 与 router 单测
4. [x] server 模块：预绑定 listener、graceful shutdown、公共 `run/serve` API
5. [x] CLI：`--config` 默认 `data/config.json`、tracing、Ctrl-C、非零错误退出
6. [x] 集成测试：随机本机端口、health、404、oneshot 优雅退出
7. [x] `gateway-rust/README.md`：实验状态、运行示例、与 octopus 并存边界
8. [x] 更新根 README 与 backend spec 的 Rust 网关 crate 契约
9. [x] 运行 fmt/check/test/clippy；确认现有 Tauri lint/build/test/check 不回归
10. [ ] 勾选 AC、父任务关联状态、spec 更新、提交归档

## 验证

```powershell
cargo fmt --manifest-path gateway-rust/Cargo.toml -- --check
cargo check --manifest-path gateway-rust/Cargo.toml
cargo test --manifest-path gateway-rust/Cargo.toml
cargo clippy --manifest-path gateway-rust/Cargo.toml --all-targets -- -D warnings

pnpm lint
pnpm build
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo test --manifest-path src-tauri/Cargo.toml
cargo check --manifest-path src-tauri/Cargo.toml
```

手工：

```powershell
cargo run --manifest-path gateway-rust/Cargo.toml -- --config gateway-rust/testdata/config.json
curl.exe http://127.0.0.1:<测试端口>/health
curl.exe -i http://127.0.0.1:<测试端口>/unknown
```

## 审查门

- 默认仅 `127.0.0.1`
- 测试只用随机端口，不碰 8080 用户实例
- 不按进程名结束 octopus
- HTTP 错误只返回稳定 JSON，不泄露内部错误
- 不修改当前 Tauri/octopus 启动与发布链路
- crate 模块可供后续鉴权/SQLite/Chat 子任务扩展

## 回滚

删除 `gateway-rust/` 及本子任务文档更新；现有桌面应用不受影响。
