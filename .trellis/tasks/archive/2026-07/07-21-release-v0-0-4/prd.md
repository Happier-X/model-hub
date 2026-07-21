# 准备 v0.0.4 Windows 发布

## Goal

将产品版本统一为 **0.0.4**，撰写中文发布说明，跑通本地门禁后提交，并推送 `v0.0.4` 标签触发 GitHub Actions `release-windows`。

## Requirements

### R1. 版本对齐

- `package.json`
- `src-tauri/tauri.conf.json`
- `src-tauri/tauri.release.conf.json`
- `src-tauri/Cargo.toml`（package version）

均为 `0.0.4`。

### R2. 发布说明

- 新增 `docs/release-notes-v0.0.4.md`，涵盖：
  - 默认 Rust 网关、安装包不再内嵌 octopus
  - 双实现回退 `MODEL_HUB_GATEWAY_IMPL=octopus`
  - 数据迁移 `migrate-octopus` 与勿混用 db
  - Chat/SSE/日志/设置等已有能力摘要（对用户有意义的变更）
  - 未代码签名 / SmartScreen 提示
  - 从 v0.0.3 应用内更新可用（若 updater 配置仍指向 latest）

### R3. 门禁

- `pnpm lint` / `pnpm build`
- `cargo fmt --check` / `test` / `check`（src-tauri）
- `cargo test` / `clippy`（gateway-rust，合理范围内）

### R4. 发布动作

- 提交版本与说明
- 推送 `master` 与 annotated tag `v0.0.4`
- 关注 Actions 是否开始（不阻塞于 runner 排队）

## Acceptance Criteria

- [x] AC1：四处版本均为 0.0.4
- [x] AC2：有 release-notes-v0.0.4.md
- [x] AC3：本地门禁通过
- [ ] AC4：tag `v0.0.4` 已推送（或明确推送失败原因）

## Out of Scope

- 本机完整 NSIS 发包（以 CI 为准）
- 代码签名采购
