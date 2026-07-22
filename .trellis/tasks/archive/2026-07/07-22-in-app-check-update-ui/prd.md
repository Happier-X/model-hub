# 应用内检查更新 UI

## Goal

在 Vue 管理台提供**手动「检查更新」**入口：调用已配置的 Tauri Updater，展示结果，经用户确认后再下载、安装并重启；避免静默覆盖。

## 已确认现状

| 层 | 状态 |
|----|------|
| Rust | `tauri_plugin_updater`、`tauri_plugin_process` 已注册 |
| 能力 | `updater:default`、`process:allow-restart` |
| 配置 | `tauri.conf.json` / release 含公钥与 `latest.json` 端点 |
| 前端 | **无**检查更新 UI；文档写明后续接入 |

## Requirements

- R1：概览页增加「检查更新」按钮（简体中文）。
- R2：调用 `@tauri-apps/plugin-updater` 的 `check()`。
- R3：无更新：明确提示当前已是最新（可展示当前版本）。
- R4：有更新：展示新版本号与可选 body；**须用户确认**后才 `downloadAndInstall`。
- R5：安装成功后调用 `@tauri-apps/plugin-process` 的 `relaunch()`；失败时中文错误、可重试。
- R6：检查/下载期间禁用重复点击；可选展示下载进度（Started/Progress/Finished）。
- R7：开发态（浏览器 `pnpm dev` 无 Tauri）失败时友好提示「请在桌面应用内检查更新」。
- R8：更新 `docs/in-app-updater.md`：管理台已支持手动检查。

## Acceptance Criteria

- [x] 概览页可触发检查更新。
- [x] 无更新 / 有更新 / 错误 三种文案可读。
- [x] 下载安装前有确认步骤。
- [x] 权限与插件无需新增 capability（沿用 default）。
- [x] `pnpm typecheck`、`pnpm lint`、`pnpm build` 通过。
- [x] 文档与实现一致。

## Out of Scope

- 启动时自动检查更新。
- 自定义更新频道 / 灰度。
- 非 Windows 安装包渠道验证。
- 修改签名密钥或 Release 工作流。

## Notes

- 轻量任务：PRD 足够。
- 不改代理业务逻辑。
