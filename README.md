# Model Hub

本机 **Vue 3 + Tauri 2** 管理台与 **进程内 Rust 代理**：统一 Base URL + 客户端 API Key，按分组故障转移队列转发 OpenAI 兼容 Chat。

## 能力（MVP）

- 多供应商（Provider）配置
- 分组 = 客户端 `model`，组内有序故障转移队列
- 默认熔断（连续失败阈值 / 恢复等待 / 半开）
- 客户端 API Key 可选（本机默认可不带；携带时须有效）
- `POST /v1/chat/completions`（非流式 + SSE）、`GET /v1/models`

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
2. （可选）在「API 密钥」页创建客户端 Key；本机默认可不带 Key
3. 客户端 Base URL 使用概览页展示的地址，例如 `http://127.0.0.1:8080`（OpenAI SDK 用 `.../v1`）
4. 对接 Pi：同一页「一键配置到 Pi」，写入 `~/.pi/agent/models.json` 的 `model-hub`

无 Key：

```bash
curl http://127.0.0.1:8080/v1/models

curl http://127.0.0.1:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{"model":"你的分组名","messages":[{"role":"user","content":"hi"}]}'
```

带 Key（可选）：

```bash
curl http://127.0.0.1:8080/v1/chat/completions \
  -H "Authorization: Bearer sk-modelhub-..." \
  -H "Content-Type: application/json" \
  -d '{"model":"你的分组名","messages":[{"role":"user","content":"hi"}]}'
```

## 架构

- 管理面：Tauri commands（IPC）
- 客户端面：本机 HTTP `/v1/*`
- 数据：应用数据目录下 SQLite（新 schema，不兼容旧版）
- **无**外部 `model-hub-gateway` / octopus 侧车进程

详细组件边界、路由与安全设计见 [当前架构](docs/current-architecture.md)。

## 文档

| 文档 | 说明 |
|------|------|
| [当前架构](docs/current-architecture.md) | 组件与安全边界 |
| [客户端对接](docs/client-integration.md) | Base URL、Key、SDK |
| [Chat 上手](docs/chat-onboarding.md) | 联调与排错 |
| [本机验收](docs/local-acceptance.md) | 可勾选联调清单 |
| [MVP 验收](docs/mvp-acceptance.md) | 自动化 + 手工 AC |
| [v0.1.0 发布说明](docs/release-notes-v0.1.0.md) | 能力与升级注意 |
| [应用更新](docs/in-app-updater.md) | 签名、Secrets、tag 发布 |

历史 0.0.x 说明仍保留在 `docs/release-notes-v0.0.*.md`，**勿**再按其中的侧车部署步骤操作当前版本。

## 发布（Windows）

版本号请同步：`package.json`、`src-tauri/Cargo.toml`、`src-tauri/tauri.conf.json`、`src-tauri/tauri.release.conf.json`（当前 **0.1.0**）。

1. 撰写/更新 `docs/release-notes-vX.Y.Z.md`
2. 推送代码后打 tag：

```bash
git tag v0.1.0
git push origin v0.1.0
```

3. GitHub Actions `release-windows` 构建 NSIS、Updater 签名资产与 `latest.json`
4. 仓库需配置 Secrets：`TAURI_SIGNING_PRIVATE_KEY`、`TAURI_SIGNING_PRIVATE_KEY_PASSWORD`

本地仅构建安装包（不上传 Release）：

```powershell
pnpm release:windows
```

完整步骤见 [应用更新与发布说明](docs/in-app-updater.md)。
