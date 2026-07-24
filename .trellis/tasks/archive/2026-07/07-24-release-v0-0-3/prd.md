# 发布 v0.0.3

## Goal

将当前 `master` 发布为 `v0.0.3`，包含 `v0.0.2` 之后的故障转移简化、应用内更新修复，以及 happier-ui / TanStack Form 相关前端改进。

## Requirements

- R1：版本号统一从 `0.0.2` 升为 `0.0.3`（`package.json`、`src-tauri/Cargo.toml`、`src-tauri/Cargo.lock` 中 model-hub 包版本、`src-tauri/tauri.conf.json`、`src-tauri/tauri.release.conf.json`）。
- R2：新增 `changelog/v0.0.3.md`，只描述 `v0.0.2` 之后的用户可见变化。
- R3：运行前端与相关 Rust 质量检查。
- R4：提交版本变更，推送 `master`，创建并推送 `v0.0.3` tag。
- R5：确认 `release-windows` 成功并生成 Release 及签名资产。

## Out of Scope

- 不修改业务功能代码（仅版本号、changelog 与发布相关文档一致性）。
- 不覆盖已发布的 `v0.0.1` / `v0.0.2` 资产。
- 不本地强制完整 NSIS 构建（以 tag 触发的 CI 为准；本地构建可选）。

## Acceptance Criteria

- [ ] 上述五处应用版本号均为 `0.0.3`。
- [ ] `changelog/v0.0.3.md` 存在且内容与 `v0.0.2..HEAD` 用户可见变更一致。
- [ ] `pnpm test:unit`、`pnpm typecheck`、`pnpm lint` 以及相关 Cargo 测试通过。
- [ ] `origin/master` 包含发版提交，远端存在 tag `v0.0.3`。
- [ ] GitHub Actions `release-windows` 成功，Release 含安装包、签名、`latest.json`、`SHA256SUMS.txt`，正文来自 changelog。
