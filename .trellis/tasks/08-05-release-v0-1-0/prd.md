# 发布 v0.1.0

## Goal

完成 v0.1.0 版本发布流程：新建 `changelog/v0.1.0.md`，同步 5 个版本文件（package.json、Cargo.toml、Cargo.lock、tauri.conf.json、tauri.release.conf.json）`0.0.9 → 0.1.0`，验证构建，提交 `chore(release): v0.1.0`，打 tag `v0.1.0` 推送到远端触发 CI release-windows，并确认 GitHub Release 生成。

## 变更范围（v0.0.9 → v0.1.0）

| 提交 | 类型 | 内容 |
|------|------|------|
| `cc49501` | fix(frontend) | 修复首页热力图 TDZ 遮蔽导致不展示 |
| `9927c68` | feat(frontend) | 分组编辑对齐 octopus 交互（卡片即时编辑 + 双栏选模） |
| `b30b22b` | feat | 模型能力排序改用 llm_benchmark 榜单 |

> 另含 `d695054` docs（规范禁用 reactive）——文档性提交，不进 changelog 功能列表。

## Requirements

1. 新建 `changelog/v0.1.0.md`，按历史格式记录本版本变更与安装/更新说明。
2. 将以下文件版本号 `0.0.9` → `0.1.0`：
   - `package.json`
   - `src-tauri/Cargo.toml`
   - `src-tauri/Cargo.lock`（`model-hub` 包 version 字段）
   - `src-tauri/tauri.conf.json`
   - `src-tauri/tauri.release.conf.json`
3. `pnpm build` 验证通过。
4. 提交 `chore(release): v0.1.0`。
5. 本地打 tag `v0.1.0` 并推送 origin，触发 `.github/workflows/release-windows.yml`。
6. 确认 GitHub Actions 触发、Release 资产（NSIS + latest.json + SHA256SUMS）生成。

## Out of Scope

- 不改功能代码、后端、数据库结构。
- 不做版本回退或删除既有 tag（除非发布失败需回滚）。
- 不做本地 NSIS 打包（CI 负责构建产物；`pnpm release:windows` 仅为可选手动兜底）。

## Acceptance Criteria

- [ ] `changelog/v0.1.0.md` 已创建且格式与历史一致。
- [ ] 5 个版本文件版本号同步为 0.1.0。
- [ ] `pnpm build` 通过。
- [ ] 已提交 `chore(release): v0.1.0`。
- [ ] 已打 tag `v0.1.0` 并推送远端，CI release-windows 触发。
- [ ] GitHub Release v0.1.0 资产可见（NSIS exe + latest.json + SHA256SUMS.txt）。

## Notes

- 发布任务：无技术设计需求，PRD-only + implement.md 执行清单。
- 参考：`.trellis/tasks/archive/2026-08/08-03-release-v0-0-9/`（同流程）。
