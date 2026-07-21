# model-hub-gateway（默认网关）

> **状态：默认网关实现。** 本 crate 是 Rust 原生网关，当前实现 **HTTP 骨架 + SQLite 持久化 + 渠道/分组 CRUD + 请求日志 list/clear + 管理 JWT / 客户端 API Key 鉴权 + 非流式/SSE 流式 Chat 转发**。  
> Windows 安装包**默认启动**本二进制（`sidecar/model-hub-gateway.exe`）。

## 目标

- 独立进程，默认只绑定本机 `127.0.0.1`
- 配置文件：`data/config.json` 的 `server` / `database` / `auth`
- `GET /health` 稳定 JSON 200；未知路径 JSON 404
- **SQLite only**：启动时按 `database.path` 打开/创建库并自动迁移
- **两套凭证分离**：
  - 管理 API：`Authorization: Bearer <JWT>`
  - 客户端 `/v1/*`：`Bearer sk-modelhub-...` 或 `x-api-key`
- 客户端 API Key 仅 create 返回完整明文；存储只保留哈希 + 脱敏
- 渠道上游 Key 可明文存于本机 SQLite；日志禁止打印完整 Key
- **客户端 `model` = 分组名**；上游 `model` = group item 的 `model_name`
- `POST /v1/chat/completions`：非流式整包 JSON；`stream=true` 透明代理上游 `text/event-stream`
- 请求日志：chat 转发后写入 `request_logs`；管理端 list/clear；不落盘完整 messages / 密钥
- Ctrl-C 优雅退出；测试使用随机端口 + 临时库，不按进程名清理网关

## 运行

在仓库根目录：

```powershell
cargo run --manifest-path gateway-rust/Cargo.toml -- --config gateway-rust/testdata/config.json
```

默认配置路径为 `data/config.json`（相对当前工作目录）。**文件不存在会启动失败**，不会静默使用内存默认值。

## 壳接入

```text
model-hub-gateway.exe --config data/config.json
```

工作目录 = `gateway_dir`。无 subcommand 时即为 serve。

```powershell
# 安装态：自动从内嵌资源部署（无需手工放置）
# 开发态：
cargo build --manifest-path gateway-rust/Cargo.toml --release
$env:MODEL_HUB_GATEWAY_RUST_BIN = "$PWD\gateway-rust\target\release\model-hub-gateway.exe"
# 或：pnpm prepare:gateway-rust → tools\gateway-rust\model-hub-gateway.exe
# 或设置 MODEL_HUB_GATEWAY_BIN
pnpm tauri dev
```

二进制解析：`MODEL_HUB_GATEWAY_BIN` → `MODEL_HUB_GATEWAY_RUST_BIN` → 安装资源 `sidecar/model-hub-gateway.exe`（按哈希部署到 `bin_dir`）→ `bin_dir/model-hub-gateway.exe`。

## 配置

```json
{
  "server": {
    "host": "127.0.0.1",
    "port": 8080
  },
  "database": {
    "type": "sqlite",
    "path": "data/data.db"
  },
  "log": {
    "level": "info"
  },
  "auth": {
    "admin_username": "admin",
    "admin_password": "admin",
    "jwt_secret": "dev-only-change-me",
    "jwt_default_expire_seconds": 86400
  }
}
```

- `database.path` 相对路径相对进程 cwd；启动时创建父目录并 migrate v1+v2。
- 仅支持 `database.type=sqlite`。

## 鉴权矩阵（摘要）

| 路径 | 无凭证 | 管理 JWT | 客户端 Key | 禁用 Key |
|------|--------|----------|------------|----------|
| `GET /health` | 200 | 200 | 200 | 200 |
| `/api/v1/*` | 401 | 按业务 | 401 | 401 |
| `/v1/models` | 401 | 401 | 200 | 401 |
| `/v1/chat/completions` | 401 | 401 | 按业务 | 401 |

Key 前缀固定 **`sk-modelhub-`**。SQLite 只存 SHA-256 哈希，不落明文；重启后仍可校验。

## 与桌面壳边界

| 项 | 约定 |
|----|------|
| 发布链路 | 安装包内嵌本二进制；见 `gateway/README.md` |
| 数据目录 | SQLite 默认 `data/data.db`（相对 cwd）；自有 schema |
| 进程清理 | 只按托管 PID / 测试端口结束，不按进程名杀全机 |
| Key 前缀 | `sk-modelhub-...` |

## 测试

```powershell
cargo fmt --manifest-path gateway-rust/Cargo.toml -- --check
cargo check --manifest-path gateway-rust/Cargo.toml
cargo test --manifest-path gateway-rust/Cargo.toml
cargo clippy --manifest-path gateway-rust/Cargo.toml --all-targets -- -D warnings
```

集成测试绑定 `127.0.0.1:0` 随机端口 + 临时 SQLite，覆盖：

- 登录 → 建 Key → `/v1/models` 鉴权矩阵
- 登录 → 渠道 CRUD → 分组 CRUD → models（分组名列表）
- API Key 跨进程实例持久化
- wiremock 上游 200 转发、SSE 多帧 + `[DONE]`、未知分组、轮询
- mock chat 后 log list / clear / page_size 上限 / 无 JWT 401

## 目录

```text
gateway-rust/
├── Cargo.toml
├── README.md
├── testdata/config.json
├── src/
│   ├── lib.rs
│   ├── main.rs          # serve（默认无 subcommand = serve）
│   ├── config.rs
│   ├── error.rs
│   ├── http.rs
│   ├── response.rs
│   ├── server.rs
│   ├── db/              # open / migrate v1+v2
│   ├── auth/            # JWT / admin / middleware
│   ├── apikey/          # model / memory + sqlite store / hash
│   ├── channel/         # model / store / service
│   ├── group/           # model / store / service
│   ├── log/             # request_logs store / service
│   ├── router/          # 分组 → 渠道 + model_name；轮询
│   ├── upstream/        # reqwest 非流式 + SSE 流式转发
│   └── routes/          # login / apikey / channel / group / log / v1 models / chat
└── tests/
    ├── http_server.rs
    ├── auth_matrix.rs
    ├── channel_group_matrix.rs
    ├── chat_forward.rs
    └── log_matrix.rs
```
