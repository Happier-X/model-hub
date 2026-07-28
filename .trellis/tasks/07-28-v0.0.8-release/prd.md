# 发布 v0.0.8

## Goal

完成 v0.0.8 版本发布流程：更新 changelog、package.json、Cargo.toml、tauri.conf.json 等版本文件，生成 release PR，触发 CI 构建与发布（不创建 Git tag，也不修改已提交的 v0.0.8 代码）。

## 根因

当前版本为 0.0.7，v0.0.8 版本已准备就绪，但用户取消了 release 流程。需重新执行 release 步骤而不触发新版本构建。

## 修复设计

- 更新 changelog/v0.0.8.md（若不存在则创建）
- 同步更新 `package.json`、`src-tauri/Cargo.toml`、`src-tauri/tauri.conf.json`、`src-tauri/tauri.release.conf.json` 中的版本号
- 生成 release PR（不推 tag）
- 运行 `npm run build` 验证构建

## Out of Scope

- 不创建 Git tag v0.0.8
- 不修改代码（仅版本文件）
- 不触发实际发布

## Acceptance Criteria

- [ ] changelog/v0.0.8.md 已更新
- [ ] 版本号同步更新
- [ ] release PR 已准备
- [ ] `npm run build` 通过
- [ ] 已提交 `chore(release): v0.0.8`