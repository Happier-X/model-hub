# 发布 v0.0.9

## Goal

完成 v0.0.9 版本发布流程：更新 changelog 与各版本文件（package.json、Cargo.toml、Cargo.lock、tauri.conf.json、tauri.release.conf.json），打 tag v0.0.9 推送到远端，触发 CI release-windows 构建并创建 GitHub Release。

## 变更范围（v0.0.8 → v0.0.9 之间的功能改动）

| 提交 | 类型 | 内容 |
|------|------|------|
| `fde3e74` | fix(backend) | 追加 Moved 事件覆盖同 DPI 切屏场景 (#3) |
| `442e07f` | feat(frontend) | 分组页改为响应式卡片网格布局 |
| `2b40d24` | feat(frontend) | 用 happier-ui 组件替换手写 UI 实现 |

## Requirements

1. 新建 `changelog/v0.0.9.md`，按历史格式记录本版本变更与安装/更新说明。
2. 将以下文件的版本号 `0.0.8` → `0.0.9`：
   - `package.json`
   - `src-tauri/Cargo.toml`
   - `src-tauri/Cargo.lock`
   - `src-tauri/tauri.conf.json`
   - `src-tauri/tauri.release.conf.json`
   - （`src-tauri/Cargo.lock` 中 `model-hub` package 的 version 字段）
3. 提交 `chore(release): v0.0.9`。
4. 创建本地 tag `v0.0.9` 并推送到 origin，触发 `.github/workflows/release-windows.yml`。

## Out of Scope

- 不改动功能代码、后端、数据库结构。
- 不做版本回退或删除既有 tag（除非发布失败需回滚）。
- 不处理 rediselig `.agents`/`.pi`/`.trellis` 的无关本地改动（本次发布不纳入）。

## Acceptance Criteria

- [ ] `changelog/v0.0.9.md` 已创建。
- [ ] 上述 5 个版本文件版本号已同步为 0.0.9，`Cargo.lock` 同步。
- [ ] 版本号变更后 `pnpm build` 通过。
- [ ] 已本地打 tag v0.0.9 并推送远端，CI release-windows 触发。
- [ ] 已提交 `chore(release): v0.0.9`。