# Model Hub

本机 **Vue 3 + Tauri 2** 管理台与 **进程内 Rust 代理**：统一 Base URL，按分组故障转移队列转发 OpenAI 兼容 Chat。本机 `/v1/*` **不校验**客户端 API Key。

## 能力（MVP）

- 多供应商（Provider）配置（上游 API Key 保留在供应商配置中）
- 分组 = 客户端 `model`，组内有序故障转移队列
- 上游错误即按队列顺序故障转移（无熔断 / 无 `auto_failover` 开关）
- 本机 `/v1/*` 无客户端鉴权（有无 `Authorization` 均放行；代理忽略客户端鉴权头）
- `POST /v1/chat/completions`（非流式 + SSE）、`GET /v1/models`
- 首页聚焦代理状态、请求统计与接入指引；设置页管理端口、数据目录和应用更新

## 开发

```powershell
pnpm install
pnpm tauri dev
```

仅前端：

```powershell
pnpm dev
```

校验：

```powershell
pnpm lint
pnpm typecheck
cd src-tauri
cargo test
cargo check
```

## 客户端用法

1. 配置供应商与分组队列（分组名 = 客户端 `model`）
2. 客户端 Base URL 使用首页展示的地址，例如 `http://127.0.0.1:8888`（OpenAI SDK 用 `.../v1`）
3. 对接 Pi：在「分组」页对目标分组点「配置到 Pi」，按分组名 upsert 写入 `~/.pi/agent/models.json` 的 `model-hub`（固定占位 Key，无需用户管理客户端 Key）

```bash
curl http://127.0.0.1:8888/v1/models

curl http://127.0.0.1:8888/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{"model":"你的分组名","messages":[{"role":"user","content":"hi"}]}'
```

OpenAI SDK 若要求非空 `api_key`，可填任意占位（如 `model-hub`）；代理不会校验该字段。

## 架构

- 管理面：Tauri commands（IPC）
- 客户端面：本机 HTTP `/v1/*`（无客户端 Key 鉴权）
- 数据：应用数据目录下 SQLite（新 schema 不含 `api_keys` 表，不兼容旧版客户端 Key 结构）
- **无**外部 `model-hub-gateway` / octopus 侧车进程

详细组件边界、路由与安全设计见 [当前架构](docs/current-architecture.md)。

## 文档

| 文档 | 说明 |
|------|------|
| [当前架构](docs/current-architecture.md) | 组件与安全边界 |
| [客户端对接](docs/client-integration.md) | Base URL、SDK |
| [Chat 上手](docs/chat-onboarding.md) | 联调与排错 |
| [本机验收](docs/local-acceptance.md) | 可勾选联调清单 |
| [MVP 验收](docs/mvp-acceptance.md) | 自动化 + 手工 AC |
| [v0.0.4 更新日志](changelog/v0.0.4.md) | 当前版本发布说明 |
| [应用更新](docs/in-app-updater.md) | 签名、Secrets、tag 发布 |

版本更新日志维护在仓库根目录 `changelog/vX.Y.Z.md`，并作为 GitHub Release 正文来源。

## 发布（Windows）

版本号请同步：`package.json`、`src-tauri/Cargo.toml`、`src-tauri/tauri.conf.json`、`src-tauri/tauri.release.conf.json`（当前 **0.0.4**）。

1. 撰写/更新 `changelog/vX.Y.Z.md`
2. 推送代码后打 tag：

```bash
git tag v0.0.4
git push origin v0.0.4
```

3. GitHub Actions `release-windows` 构建 NSIS、Updater 签名资产与 `latest.json`，Release 正文读取对应 changelog
4. 仓库需配置 Secrets：`TAURI_SIGNING_PRIVATE_KEY`、`TAURI_SIGNING_PRIVATE_KEY_PASSWORD`

本地仅构建安装包（不上传 Release）：

```powershell
pnpm release:windows
```

完整步骤见 [应用更新与发布说明](docs/in-app-updater.md)。
