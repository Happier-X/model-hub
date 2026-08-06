# 发布 v0.1.1

## Goal

将当前 master 变更（依赖大升级 + 页面调整）发布为 **v0.1.1**，走既有 release-windows CI 流程构建 NSIS + updater 签名资产并创建 GitHub Release。

## Background

- 当前版本 0.1.0（tag `v0.1.0` 已发布），master 在 v0.1.0 之后新增：
  - `2dcf716` feat: 分组新建/编辑改为独立页面
  - `6bc4f82` feat: 日志页移除筛选控件，保留刷新/清理操作
  - `3229d59` chore(deps): 前端 npm 依赖全部 latest（vue-router 5 / vite 8 / eslint 10 / TS 6 / happier-ui 0.1.1 等）
  - `db2553a` chore(deps): 后端 Rust crate 全部 latest（reqwest 0.13 / rusqlite 0.40 / tower-http 0.7 等）
- 版本号经用户确认定为 **0.1.1**（patch）。
- 发布机制：push `v*` tag 触发 `.github/workflows/release-windows.yml`，tauri-action 构建 NSIS + updater 签名资产 + 创建 GitHub Release；workflow 从 `changelog/v{version}.md` 读 release body，并校验 latest.json 版本与 tag 一致。

## Requirements

1. **R1 版本号同步 0.1.0 → 0.1.1**
   - `package.json` / `src-tauri/Cargo.toml` / `src-tauri/Cargo.lock` / `src-tauri/tauri.conf.json` / `src-tauri/tauri.release.conf.json`
2. **R2 changelog**
   - 新增 `changelog/v0.1.1.md`，汇总 v0.1.0 以来用户可见变更（分组独立页、日志页删筛选、依赖升级）
3. **R3 发布**
   - commit `chore(release): v0.1.1`
   - 打 tag `v0.1.1` 并 push → CI 构建发布
4. **R4 验证**
   - 本地先跑 `pnpm build` + `cargo check` 冒烟（发布前置检查）
   - CI 完成后核验 Release 资产（latest.json、.sig、NSIS exe、SHA256SUMS.txt）

## Out of Scope

- 不引入新功能/重构
- 不修改 workflow（如 CI 失败按需修复，另议）

## Acceptance Criteria

- [ ] AC1：5 个版本号文件均为 0.1.1，无遗漏
- [ ] AC2：`changelog/v0.1.1.md` 存在且内容准确
- [ ] AC3：tag `v0.1.1` 已推送，CI workflow 触发成功
- [ ] AC4：GitHub Release v0.1.1 资产齐全（NSIS exe / latest.json / .sig / SHA256SUMS.txt），latest.json 版本与 tag 一致

## Notes

- 轻量任务：PRD-only。
- tag 必须打在包含版本号 bump 的 commit 上（先 commit 后 tag）。
