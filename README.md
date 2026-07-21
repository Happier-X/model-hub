# Model Hub

本机 **Vue 3 + Tauri 2** 管理台与 **进程内 Rust 代理**：统一 Base URL + 客户端 API Key，按分组故障转移队列转发 OpenAI 兼容 Chat。

## 能力（MVP）

- 多供应商（Provider）配置
- 分组 = 客户端 `model`，组内有序故障转移队列
- 默认熔断（连续失败阈值 / 恢复等待 / 半开）
- 强制客户端 API Key（`Authorization: Bearer sk-modelhub-...`）
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

1. 在应用内创建客户端 API Key（明文仅展示一次）
2. 配置供应商与分组队列
3. 客户端 Base URL 使用概览页展示的地址，例如 `http://127.0.0.1:8080`
4. `model` 填分组名

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

旧 `gateway-rust` 侧车与 React UI 已废弃，不再作为运行时依赖。
