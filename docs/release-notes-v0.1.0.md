# Model Hub v0.1.0 发布说明

本版本为 **Vue 3 管理台 + Tauri 2 进程内 Rust 代理** 的正式基线，不再依赖外部 `model-hub-gateway` / octopus 侧车进程。

## 能力摘要

- 多供应商（OpenAI 兼容 Base URL + 上游 Key）
- 分组 = 客户端请求中的 `model`，组内有序故障转移队列
- 默认熔断（连续失败阈值 / 恢复等待 / 半开单探测）
- `/v1/*` 客户端 API Key **可选**（本机默认可不带；携带时须有效）
- `GET /v1/models`、`POST /v1/chat/completions`（非流式 + SSE）
- 管理台：概览启停与端口、供应商/分组/Key/日志、健康状态与连续失败次数
- Windows NSIS 安装包 + Tauri Updater 签名资产（见 [应用更新](./in-app-updater.md)）

## 相对 v0.0.8 的破坏性变化

| 项 | v0.0.8 及更早常见形态 | v0.1.0 |
|----|----------------------|--------|
| 代理形态 | 侧车 `model-hub-gateway.exe` / 历史 octopus | **Tauri 进程内**异步 HTTP 代理 |
| 客户端鉴权 | 部分版本强制 Key / 部分本地开放 | **本机默认可不带 Key**；携带错误 Key 仍 401 |
| 管理面 | 可能依赖网关管理 HTTP | **仅** Tauri IPC commands |
| 数据库 | 旧网关 schema | **新 schema**；**不**自动迁移旧库 |
| 构建 | 可能需 `prepare:gateway-rust` 等 | 仅根前端 + `src-tauri` |

从 0.0.x 升级时，请：

1. 完全退出旧版（含托盘）。
2. 安装 0.1.0 后在应用内**重新**配置供应商、分组与客户端 Key（勿假设旧 SQLite 可直接沿用）。
3. 客户端继续使用本机 Base URL（默认 `http://127.0.0.1:8080`），`model` 填**分组名**，并带上新创建的 Key。

## 安装与更新

- 安装包：GitHub Release 中的 NSIS `.exe`
- 校验：同 Release 的 `SHA256SUMS.txt`
- 应用内更新清单：`https://github.com/Happier-X/model-hub/releases/latest/download/latest.json`（需签名校验）
- 概览页提供手动「检查更新」；用户确认后下载安装并重启。也可从 GitHub Release 安装。详情见 [应用更新与发布说明](./in-app-updater.md)。

## 架构与验收

- 架构：[当前架构](./current-architecture.md)
- 自动化 / 手工 MVP：[mvp-acceptance.md](./mvp-acceptance.md)
- 本机联调（健康、故障转移等）：[local-acceptance.md](./local-acceptance.md)
- 客户端对接：[client-integration.md](./client-integration.md)

## 开发者验证

```powershell
pnpm install
pnpm lint
pnpm typecheck
pnpm build
cd src-tauri
cargo test
cargo check
```

本地打 Windows 安装包（需本机已配置 Tauri 环境；正式发布仍以 tag 触发 CI 为准）：

```powershell
pnpm release:windows
```

CI 推送 `v*` tag 时执行 `.github/workflows/release-windows.yml`，需 Secrets：

```text
TAURI_SIGNING_PRIVATE_KEY
TAURI_SIGNING_PRIVATE_KEY_PASSWORD
```

## 已知限制

- 管理台支持手动检查更新；暂无启动时自动检查。
- 不提供从 0.0.x 侧车数据库的自动导入。
- 默认仅监听 `127.0.0.1`；改非本机地址存在暴露风险。
