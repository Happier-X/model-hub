# 发布 v0.0.8

## Goal

同步版本号到 `0.0.8`，新增 `changelog/v0.0.8.md`，跑质量检查，提交并推送 `v0.0.8` tag
触发 `release-windows` CI，发布 Windows 安装包到 GitHub Release。

本轮主题：**三列表页（供应商/分组/日志）布局重构** —— 无整页滚动、表格/列表内部滚动、
底部分页器；附供应商行内启用开关、标题栏加号按钮等 UI 细节调整。

## 范围内（本次发布包含的 feat，自 v0.0.7 tag 之后）

| Commit | 描述 | 类别 |
|--------|------|------|
| 3884be1 | 供应商页表格占满页高与底部分页 | 新增 / 变更（布局重构） |
| 61cc93c | 分组页无整页滚动+卡片列表内部滚动 | 新增 / 变更（布局重构） |
| d750559 | 日志页无整页滚动+表格 body 滚动+底部保留分页 | 新增 / 变更（布局重构） |
| bd4cb09 | 日志页分页器移至表格底部 | 新增 / 变更（布局重构前置） |
| 0a11605 | spec 记录表格内部滚动+底部分页布局模式 | 文档（不进 changelog 主体） |
| 8efbfa8 | 供应商列表启用列改为行内 HSwitch 开关 | 新增 / 变更 |
| 1ec5b3a | 供应商页标题右侧加号按钮并移除旧版管理条 | 新增 / 变更（UI 细节） |
| 9bf8e2a | 分组页标题右侧加号按钮并移除旧版管理条 | 新增 / 变更（UI 细节） |
| 8a0ed99 | 供应商操作栏编辑按钮 variant ghost → outline | 修复（UI 细节） |

## Requirements

### 版本号同步（4 处）
- `package.json` `version`: `0.0.7` → `0.0.8`
- `src-tauri/Cargo.toml` `version`: `0.0.7` → `0.0.8`
- `src-tauri/tauri.conf.json` `version`: `0.0.7` → `0.0.8`
- `src-tauri/tauri.release.conf.json` `version`: `0.0.7` → `0.0.8`

### Changelog
- 新增 `changelog/v0.0.8.md`，含三大块：新增/变更、修复、安装与更新（沿用 v0.0.7/v0.0.6
  模板）。安装与更新段引用 GitHub Release URL 与 `latest.json` 清单，原样复制上一版。

### 质量检查
- `npm run build` 通过（`vue-tsc --noEmit && vite build`，无类型错误）
- `npm run lint` 通过（`eslint .`）
- `npm run test:unit` 通过（`node --experimental-strip-types --test src/utils/*.test.ts`）

### 提交与发布
- 一次发布提交：`chore(release): v0.0.8`（含 4 文件版本号 + changelog）
- 推送 annotated tag `v0.0.8`：`git tag -a v0.0.8 -m "v0.0.8"` + `git push origin v0.0.8`
- Tag push 触发 `.github/workflows/release-windows.yml`，自动构建 NSIS 发布资产到 GitHub Release

## Out of Scope

- 后端逻辑 / Rust 代码改动（本轮所有 feat 都是前端 Vue 页面）
- Tauri 配置结构、updater 公钥、CI workflow 改动
- 应用内更新流程逻辑改动
- 本机 `pnpm release:windows` 预构建（耗时较长，CI 会跑；本机不强制跑）

## Acceptance Criteria

- [ ] `package.json` `version` = `0.0.8`
- [ ] `src-tauri/Cargo.toml` `version` = `0.0.8`
- [ ] `src-tauri/tauri.conf.json` `version` = `0.0.8`
- [ ] `src-tauri/tauri.release.conf.json` `version` = `0.0.8`
- [ ] `changelog/v0.0.8.md` 存在，分「新增/变更、修复、安装与更新」三大块，文案简体中文
- [ ] `npm run build` 通过
- [ ] `npm run lint` 通过
- [ ] `npm run test:unit` 通过
- [ ] 已提交 `chore(release): v0.0.8`
- [ ] 已推送 annotated tag `v0.0.8` 到 origin
- [ ] CI `release-windows` 收到 tag 后开始运行（GitHub Actions 显示该 workflow 在跑）

## Notes

- PRD-only 轻量发布任务。参考 `docs/in-app-updater.md`「发布步骤」章节及
  `changelog/v0.0.7.md` / `v0.0.6.md` 模板。
- **不可逆操作提醒**：`git push origin v0.0.8` 触发 CI 真实发布。按
  `docs/in-app-updater.md` 失败处理「已发布标签的资产不得原地覆盖；修复后发布更高版本」。
  push tag 前确认所有版本号 + changelog + 质量检查已通过。
- 本机无 `TAURI_SIGNING_PRIVATE_KEY` 环境变量时**不要**跑 `pnpm release:windows`（会因缺私钥失败），
  CI 自带 secrets 能跑通。
- 推 tag 后即时通知用户「CI 已触发，请到 GitHub Actions 与 Release 页面确认产物」，
  本会话不阻塞等待 CI 完成。