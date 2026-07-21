# 执行计划：壳双实现切换

## 清单

1. [x] `GatewayImpl` 与 env 解析
2. [x] binary 解析支持 rust 名 / RUST_BIN
3. [x] process 启动参数按 impl 分支；status.impl_name
4. [x] 前端 GatewayStatus 可选字段（若暴露）
5. [x] 单测：默认 octopus、rust 参数、缺失二进制错误
6. [x] 文档：gateway/README、gateway-rust/README、spec
7. [x] cargo test/fmt/check；pnpm lint/build
8. [ ] 提交归档

## 验证

```powershell
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo test --manifest-path src-tauri/Cargo.toml
cargo check --manifest-path src-tauri/Cargo.toml
pnpm lint
pnpm build
```

## 审查门

- 默认路径零变化
- rust 仅 opt-in
- 不改 release 内嵌策略
- 数据混用风险有文档警告
