# Model Hub v0.1.0 发布说明

本版本为 **Vue 3 管理台 + Tauri 2 进程内 Rust 代理** 的正式基线，不再依赖外部 `model-hub-gateway` / octopus 侧车进程。

## 能力摘要

- 多供应商（OpenAI 兼容 Base URL + 上游 Key）
- 分组 = 客户端请求中的 `model`，组内有序故障转移队列（支持拖拽排序、批量添加上游模型）
- 默认熔断（连续失败阈值 / 恢复等待 / 半开单探测）
- `/v1/*` 客户端 API Key **可选**（本机默认可不带；携带错误/停用 Key 仍 401）
- Pi 一键导出：API Key 页写入 `~/.pi/agent/models.json` 的 `model-hub` 供应商；Key 可留空（占位 `model-hub` 与无 Key 同等放行）
- `GET /v1/models`、`POST /v1/chat/completions`（非流式 + SSE）
- 管理台：概览启停与端口/今日请求统计、供应商/分组/Key/日志筛选分页、健康状态与连续失败次数、可选启动检查更新
- 旧版 SQLite 兼容：对 `groups` / `group_items` / `api_keys` / `request_logs` 缺列做幂等 `ALTER` + 必要双写，不重建丢数据
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
2. 安装 0.1.0 后优先在应用内**重新**配置供应商、分组与（可选）客户端 Key。
3. 若本机已有**当前架构**下的 SQLite，启动时会尽量补齐缺列（如 `auto_failover`、`status_code`、`masked` 等）；**不**保证完整导入更早侧车库的全部业务数据。
4. 客户端继续使用本机 Base URL（默认 `http://127.0.0.1:8080`），`model` 填**分组名**；本机可不带 Key，若带 Key 须为有效启用的客户端 Key。

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

## 本机对接 Pi Agent（可选）

1. 在「API 密钥」页点击 **一键配置到 Pi**（Key 可留空）。
2. 配置写入 `%USERPROFILE%\.pi\agent\models.json` 的 `providers.model-hub`（合并，不覆盖其它供应商）。
3. 在 Pi 中 `/model` 选择 `model-hub/<分组名>`。
4. 修改后须**完全重启** Model Hub 进程再验证（避免旧二进制仍强制 Key）。

## 已知限制

- 启动时检查更新为**可选**偏好（默认关闭）；不会静默安装。
- 不提供从 0.0.x 侧车数据库的完整自动导入（仅缺列兼容）。
- 默认仅监听 `127.0.0.1`；改非本机地址存在暴露风险（本机开放无 Key 时更须注意）。
- 客户端 Key 明文仅创建时展示一次；列表仅脱敏。
