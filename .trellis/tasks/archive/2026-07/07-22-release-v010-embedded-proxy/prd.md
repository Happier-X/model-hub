# v0.1.0 发布就绪：文档与发布流程对齐内嵌代理

## Goal

将仓库中的**当前用户入口文档与发布产物说明**对齐到 **0.1.0（Vue3 + Tauri 进程内 Rust 代理）**，避免读者按旧版侧车 `model-hub-gateway` / 本地无鉴权 / `prepare:gateway-rust` 路径操作失败；并补齐 v0.1.0 发布说明与本地发布命令入口。

## 背景（审计）

| 项 | 现状 |
|----|------|
| 版本号 | `package.json` / `tauri.conf.json` / `tauri.release.conf.json` 均为 **0.1.0** |
| 架构文档 | `README.md`、`docs/current-architecture.md`、`docs/mvp-acceptance.md`、`docs/local-acceptance.md` 已描述内嵌代理 |
| 历史发布说明 | `docs/release-notes-v0.0.x.md` 仍描述侧车网关、本地开放无 Key 等（**保留作历史**） |
| 缺口 | **缺少** `docs/release-notes-v0.1.0.md`；`package.json` **无** `release:windows`；README 未写清 tag 发布与签名 Secret |
| CI | `.github/workflows/release-windows.yml` 已按 NSIS + updater + 内嵌代理文案，可复用 |

## Requirements

- R1：新增 `docs/release-notes-v0.1.0.md`（能力摘要、相对 0.0.8 的破坏性变化、升级注意、安装与更新、验证命令）。
- R2：`package.json` 增加 Windows 发布脚本入口（例如 `release:windows` → `tauri build --bundles nsis -c src-tauri/tauri.release.conf.json`），与 workflow 一致。
- R3：`README.md` 增加简短「发布（Windows）」与「应用内更新」指向（`docs/in-app-updater.md`、发布说明、需 `TAURI_SIGNING_PRIVATE_KEY`）。
- R4：快速扫读 `docs/client-integration.md`、`docs/chat-onboarding.md`、`docs/in-app-updater.md`：若仍写侧车/无 Key/旧 Key 前缀，改为当前契约或加「历史版本见 release-notes-v0.0.x」提示；**不重写**全部 0.0.x 历史说明。
- R5：版本号三处保持 **0.1.0** 一致（已一致则仅校验）。

## Acceptance Criteria

- [x] 存在 `docs/release-notes-v0.1.0.md`，明确：强制客户端 Key、进程内代理、分组=model、故障转移/熔断、SQLite 新 schema 不兼容旧库。
- [x] `pnpm release:windows`（或同等脚本）可映射到与 CI 一致的 tauri 构建参数。
- [x] README 含发布与更新入口，无 `prepare:gateway-rust` 作为当前必做步骤。
- [x] 客户端/引导/更新文档不引导用户使用已删除的侧车部署步骤。
- [x] 不修改业务代理逻辑；`pnpm typecheck` 与 `pnpm lint` 不因文档任务失败（若仅文档可注明 N/A）。

## Out of Scope

- 实际打 tag、推 GitHub Release、配置 Secret。
- 代码签名证书、商店上架。
- 删除或改写全部 `release-notes-v0.0.x` 历史内容。
- 新功能开发（上游 models 探测、暗色主题等）。

## Notes

- 轻量文档任务：PRD + 实现即可。
- 发布说明应链接 `docs/local-acceptance.md` 与 `docs/current-architecture.md`。
