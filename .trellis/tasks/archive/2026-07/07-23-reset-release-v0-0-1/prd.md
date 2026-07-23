# 重置发布为 v0.0.1

## Goal

清空历史版本号与发布标签痕迹，按 muses 风格用仓库内 `changelog/` 维护发布说明，并以 `v0.0.1` 作为新的起始正式版本重新发布。

## Background

- 当前代码版本为 `0.1.1`（`package.json`、`src-tauri/Cargo.toml`、`tauri.conf.json`、`tauri.release.conf.json`）。
- 本地存在 tag：`v0.0.1`–`v0.0.8`、`v0.1.1`；远端 origin 也有对应 tags；GitHub 存在对应 Releases。
- 现有说明分散在 `docs/release-notes-v*.md`，CI 从该路径复制附件。
- muses 采用 `changelog/vX.Y.Z.md`，Release 工作流读取该文件写入 GitHub Release body。
- 当前 `master` 相对 origin 超前若干提交，发版前需一并推送。

## Requirements

- R1：应用版本号统一改为 `0.0.1`（`package.json`、`src-tauri/Cargo.toml`、`src-tauri/tauri.conf.json`、`src-tauri/tauri.release.conf.json`）。
- R2：新增 `changelog/v0.0.1.md`，按**当前产品首发说明**撰写，不逐条搬运旧 release-notes 历史叙事。
- R3：完整重置历史发布痕迹：删除本地与远端全部历史 tag；删除对应 GitHub Releases；删除 `docs/release-notes-v*.md`。
- R4：同步 README 与 `docs/in-app-updater.md` 发版步骤，改为维护 `changelog/vX.Y.Z.md`。
- R5：Release CI 读取 `changelog/v{version}.md` 作为 GitHub Release body（对齐 muses），并保留 NSIS / 签名 / `latest.json` / `SHA256SUMS` 上传能力。
- R6：先推送 `master` 上未推送提交，再创建并推送新 `v0.0.1` tag 触发 Windows 发布工作流。

## Acceptance Criteria

- [ ] 四个版本文件均为 `0.0.1`。
- [ ] 存在 `changelog/v0.0.1.md`，以当前能力为中心的首发说明。
- [ ] Release CI 读取 `changelog/v{version}.md` 作为 Release body。
- [ ] 本地与远端历史 tag 已删除，旧 GitHub Releases 已删除。
- [ ] `docs/release-notes-v*.md` 已删除，文档改为引用 `changelog/`。
- [ ] `master` 与新 `v0.0.1` 已推送远端，可触发发布工作流。

## Out of Scope

- 修改产品功能本身（除版本与发版流程外）。
- 更换 Tauri 签名密钥。
- 回写或归档旧 release-notes 内容到 changelog 历史条目。

## Confirmed Decisions

- 完整重置：删除本地与远端全部历史 tag、对应 GitHub Releases、以及 `docs/release-notes-v*.md`。
- `changelog/v0.0.1.md` 按当前产品首发说明撰写。
- 发版前先推送 `master` 未推送提交，再推新 `v0.0.1` tag。
