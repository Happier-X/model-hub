# 发布 v0.0.2

## Goal

将当前 `master` 发布为 `v0.0.2`，包含 v0.0.1 之后的配置到 Pi 参数修复与默认端口 8888 调整。

## Requirements

- R1：版本号统一从 `0.0.1` 升为 `0.0.2`（package、Cargo、Tauri 配置及 Cargo.lock）。
- R2：新增 `changelog/v0.0.2.md`，只描述 v0.0.1 之后的用户可见变化。
- R3：运行前端与 Rust 质量检查。
- R4：提交版本变更，推送 `master`，创建并推送 `v0.0.2` tag。
- R5：确认 `release-windows` 成功并生成 Release 及签名资产。

## Acceptance Criteria

- [ ] 所有应用版本号为 0.0.2。
- [ ] `changelog/v0.0.2.md` 存在且内容准确。
- [ ] `pnpm test:unit`、`pnpm typecheck`、`pnpm lint`、相关 Cargo 测试通过。
- [ ] origin/master 包含发版提交，远端存在 `v0.0.2`。
- [ ] GitHub Actions 发布成功，Release 含安装包、签名、latest.json、SHA256SUMS。
