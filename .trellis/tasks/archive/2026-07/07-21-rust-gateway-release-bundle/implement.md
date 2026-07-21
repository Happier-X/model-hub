# 执行计划：发布内嵌 Rust 网关

## 清单

1. [x] prepare-bundled-gateway-rust.ps1 + gitignore
2. [x] tauri.release.conf.json 增加 rust sidecar 映射
3. [x] binary.rs：rust 从 resource 部署
4. [x] release-windows.yml 构建 gateway-rust
5. [x] 单测 + 文档/spec
6. [ ] cargo/pnpm 门禁；提交归档

## 验证

```powershell
# 本地 prepare（可选）
powershell -ExecutionPolicy Bypass -File scripts/prepare-bundled-gateway-rust.ps1

cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo test --manifest-path src-tauri/Cargo.toml
cargo check --manifest-path src-tauri/Cargo.toml
pnpm lint
pnpm build
```

## 审查门

- 默认 octopus 不变
- 不提交 exe
- AGPL 材料保留
- 混用 db 警告仍在
