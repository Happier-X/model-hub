# 执行计划：去侧车发布

## 清单

1. [x] impl_kind 默认 rust + 单测
2. [x] binary/octopus 错误文案与相关测
3. [x] tauri.release.conf.json 仅 rust sidecar
4. [x] release-windows.yml / package.json
5. [x] NOTICE + README 全家桶
6. [x] backend spec 同步
7. [x] cargo/pnpm 门禁；提交归档（未 commit，待主会话）

## 验证

```powershell
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo test --manifest-path src-tauri/Cargo.toml
cargo check --manifest-path src-tauri/Cargo.toml
pnpm lint
pnpm build
```

## 审查门

- 默认无 AGPL 二进制
- 显式 octopus 回退仍可用
- 文档不谎称仍内嵌 octopus
