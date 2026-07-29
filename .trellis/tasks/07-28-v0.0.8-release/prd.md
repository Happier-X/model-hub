# 发布 v0.0.8

## Goal

完成 v0.0.8 版本发布流程：更新 changelog、package.json、Cargo.toml、tauri.conf.json 等版本文件，删除旧 tag 并重建 v0.0.8 tag 推送到远端，触发 CI release windows 构建。

## 变更内容

- 创建 changelog/v0.0.8.md
- 同步更新 `package.json`、`src-tauri/Cargo.toml`、`src-tauri/tauri.conf.json`、`src-tauri/tauri.release.conf.json` 版本号 0.0.7→0.0.8
- 删除并重建远程 Git tag v0.0.8 以触发 release CI

## Acceptance Criteria

- [ ] changelog/v0.0.8.md 已更新
- [ ] 版本号同步更新
- [ ] release PR 已准备
- [ ] `npm run build` 通过
- [ ] 已提交 `chore(release): v0.0.8`