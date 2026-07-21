# model-hub-gateway（实验）

> **状态：实验性。** 本 crate 是 Rust 原生网关，当前实现 **HTTP 骨架 + 管理 JWT / 客户端 API Key 鉴权闭环**，供后续 SQLite / 渠道 / Chat 切片开发。  
> **不能**替代当前 Model Hub 发布版内嵌的 **octopus v0.9.28** 侧车，也 **不会** 被 Tauri 壳默认拉起。

## 目标

- 独立进程，默认只绑定本机 `127.0.0.1`
- 配置文件契约对齐 octopus：`data/config.json` 的 `server.host` / `server.port`
- `GET /health` 稳定 JSON 200；未知路径 JSON 404
- **两套凭证分离**：
  - 管理 API：`Authorization: Bearer <JWT>`
  - 客户端 `/v1/*`：`Bearer sk-octopus-...` 或 `x-api-key`
- 完整 API Key 仅创建时返回一次；存储只保留哈希 + 脱敏元数据
- Ctrl-C 优雅退出；测试使用随机端口，不按进程名清理 octopus

## 运行

在仓库根目录：

```powershell
cargo run --manifest-path gateway-rust/Cargo.toml -- --config gateway-rust/testdata/config.json
```

默认配置路径为 `data/config.json`（相对当前工作目录）。**文件不存在会启动失败**，不会静默使用内存默认值。

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

- 当前消费 `server` 与可选 `auth`；`database` / `log` 保留字段以便与壳写入的配置兼容。
- `auth` 缺省：用户名/密码 `admin`/`admin`；`jwt_secret` 未配置时进程启动生成随机密钥并 warning（重启后 Token 失效）。
- 校验：`host` 合法 IP、`port != 0`；默认绑定 `127.0.0.1:8080`。
- 日志禁止打印完整 JWT / API Key。

## HTTP 契约

### `GET /health`

```json
{
  "status": "ok",
  "service": "model-hub-gateway",
  "version": "0.1.0"
}
```

### 管理登录

`POST /api/v1/user/login`

```json
// 请求
{ "username": "admin", "password": "admin", "expire": 86400 }

// 成功 200
{ "data": { "token": "<jwt>", "expire_at": "<unix 秒字符串>" } }

// 失败 401
{
  "message": "用户名或密码错误",
  "error": { "code": "UNAUTHORIZED", "message": "用户名或密码错误" }
}
```

### 管理状态

`GET /api/v1/user/status` + `Authorization: Bearer <jwt>` → `{ "data": "ok" }`

### API Key CRUD（均需管理 JWT）

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/api/v1/apikey/list` | 列表；`api_key` 为脱敏值 |
| POST | `/api/v1/apikey/create` | body `{ "name", "enabled?" }`；**完整 Key 仅此返回一次** |
| POST | `/api/v1/apikey/update` | body `{ "id", "name?", "enabled?", ... }` |
| DELETE | `/api/v1/apikey/delete/:id` | 删除 |

Key 前缀固定 **`sk-octopus-`**。内存 store 只存 SHA-256 哈希，不落明文。

### 客户端占位

`GET /v1/models`（OpenAI 风格，**不**包管理信封）：

- 有效 Key → `200` `{ "object": "list", "data": [] }`
- 无凭证 / 坏 Key / 禁用 Key / **管理 JWT** → `401`（含顶层 `message`）

### 未知路径

```json
{
  "error": {
    "code": "NOT_FOUND",
    "message": "未找到请求的接口"
  }
}
```

## curl 示例（端口以配置为准，testdata 为 `18081`）

```powershell
# 登录
curl.exe -s -X POST http://127.0.0.1:18081/api/v1/user/login `
  -H "Content-Type: application/json" `
  -d "{\"username\":\"admin\",\"password\":\"admin\",\"expire\":86400}"

# 状态（替换 TOKEN）
curl.exe -s http://127.0.0.1:18081/api/v1/user/status -H "Authorization: Bearer TOKEN"

# 创建客户端 Key
curl.exe -s -X POST http://127.0.0.1:18081/api/v1/apikey/create `
  -H "Authorization: Bearer TOKEN" -H "Content-Type: application/json" `
  -d "{\"name\":\"local-client\",\"enabled\":true}"

# 客户端 models（替换 SK）
curl.exe -s http://127.0.0.1:18081/v1/models -H "Authorization: Bearer SK"
```

## 鉴权矩阵

| 路径 | 无凭证 | 管理 JWT | 有效 sk | 无效/禁用 sk |
|------|--------|----------|---------|--------------|
| `/health` | 200 | 200 | 200 | 200 |
| `/api/v1/user/login` | 可调 | 可调 | 可调 | 可调 |
| `/api/v1/user/status` | 401 | 200 | 401 | 401 |
| `/api/v1/apikey/*` | 401 | 按业务 | 401 | 401 |
| `/v1/models` | 401 | 401 | 200 | 401 |

## 与 octopus / Tauri 边界

| 项 | 约定 |
|----|------|
| 当前发布链路 | 仍使用内嵌 octopus；见 `gateway/README.md` |
| Tauri `GatewayRuntime` | **不修改** 默认侧车启动路径 |
| 数据目录 | 本任务使用内存 API Key store；后续 SQLite 任务替换 |
| 进程清理 | 不按 `octopus` 进程名结束任何进程 |

## 测试

```powershell
cargo fmt --manifest-path gateway-rust/Cargo.toml -- --check
cargo check --manifest-path gateway-rust/Cargo.toml
cargo test --manifest-path gateway-rust/Cargo.toml
cargo clippy --manifest-path gateway-rust/Cargo.toml --all-targets -- -D warnings
```

集成测试绑定 `127.0.0.1:0` 随机端口，覆盖登录 → 建 Key → `/v1/models` 鉴权矩阵。

## 目录

```text
gateway-rust/
├── Cargo.toml
├── README.md
├── testdata/config.json
├── src/
│   ├── lib.rs
│   ├── main.rs
│   ├── config.rs
│   ├── error.rs
│   ├── http.rs
│   ├── response.rs
│   ├── server.rs
│   ├── auth/          # JWT / admin / middleware
│   ├── apikey/        # model / store / hash
│   └── routes/        # login / apikey / v1 models
└── tests/
    ├── http_server.rs
    └── auth_matrix.rs
```
