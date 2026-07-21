# PRD：发布 v0.0.5

## 背景

主线已去掉 octopus 运行时兼容与命名残留，需发布新的 Windows 安装包与应用内更新基线。

## 目标

- 版本号统一为 **0.0.5**
- 撰写发布说明
- 推送 annotated tag `v0.0.5` 触发 `release-windows` Actions

## 范围

### 必须

-  bump：`package.json`、`src-tauri/tauri.conf.json`、`src-tauri/tauri.release.conf.json`、`src-tauri/Cargo.toml`（及 lock 同步）
- 新增 `docs/release-notes-v0.0.5.md`，更新 README 当前版本提示
- 提交后打 annotated tag 并 push（含 tag）

### 不做

- 不改产品功能（仅发布元数据）
- 不修改历史 release notes 正文

## 验收标准

- [ ] 各配置版本为 0.0.5
- [ ] 发布说明覆盖：移除 octopus 兼容、Key 前缀 `sk-modelhub-`、破坏性迁移提示
- [ ] tag `v0.0.5` 已推送，Actions 已触发
