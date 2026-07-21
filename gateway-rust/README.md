# model-hub-gateway（实验）

> **状态：实验性。** 本 crate 是 Rust 原生网关，当前实现 **HTTP 骨架 + SQLite 持久化 + 渠道/分组 CRUD + 管理 JWT / 客户端 API Key 鉴权 + 非流式/SSE 流式 Chat 转发**，供后续发布接入切片开发。  
> **不能**替代当前 Model Hub 发布版内嵌的 **octopus v0.9.28** 侧车，也 **不会** 被 Tauri 壳默认拉起。

## 目标

- 独立进程，默认只绑定本机 `127.0.0.1`
- 配置文件契约对齐 octopus：`data/config.json` 的 `server` / `database` / `auth`
- `GET /health` 稳定 JSON 200；未知路径 JSON 404
- **SQLite only**：启动时按 `database.path` 打开/创建库并自动迁移
- **两套凭证分离**：
  - 管理 API：`Authorization: Bearer <JWT>`
  - 客户端 `/v1/*`：`Bearer sk-octopus-...` 或 `x-api-key`
- 客户端 API Key 仅 create 返回完整明文；存储只保留哈希 + 脱敏
- 渠道上游 Key 可明文存于本机 SQLite；日志禁止打印完整 Key
- **客户端 `model` = 分组名**；上游 `model` = group item 的 `model_name`
- `POST /v1/chat/completions`：非流式整包 JSON；`stream=true` 透明代理上游 `text/event-stream`
- Ctrl-C 优雅退出；测试使用随机端口 + 临时库，不按进程名清理 octopus

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

- `database.type` **必须**为 `sqlite`；其它类型启动失败。
- `database.path` 相对路径相对进程 cwd；启动时创建父目录并 migrate v1。
- `auth` 缺省：用户名/密码 `admin`/`admin`；`jwt_secret` 未配置时进程启动生成随机密钥并 warning（重启后 Token 失效）。
- 校验：`host` 合法 IP、`port != 0`；默认绑定 `127.0.0.1:8080`。
- 上游 HTTP：非流式整包超时默认 **60s**；流式连接超时 **10s**、整响应兜底 **300s**（`reqwest` + rustls + stream）。
- 日志禁止打印完整 JWT / 客户端 Key / 上游 channel_key / 用户 messages 全文。

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

Key 前缀固定 **`sk-octopus-`**。SQLite 只存 SHA-256 哈希，不落明文；重启后仍可校验。

### 渠道 CRUD（均需管理 JWT）

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/api/v1/channel/list` | 含 `base_urls` / `keys`；`type` 为数字 |
| POST | `/api/v1/channel/create` | 对齐 UI：name, type, enabled, base_urls, keys, model, custom_model, proxy, auto_sync, auto_group, custom_header |
| POST | `/api/v1/channel/update` | 部分更新；支持 `keys_to_update` / `keys_to_add` |
| POST | `/api/v1/channel/enable` | `{ "id", "enabled" }` |
| DELETE | `/api/v1/channel/delete/:id` | 删除（级联 base_urls / keys） |

### 分组 CRUD（均需管理 JWT）

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/api/v1/group/list` | 含 `items`；`mode` 为数字（1=轮询） |
| POST | `/api/v1/group/create` | name, mode, match_regex, items[] |
| POST | `/api/v1/group/update` | 支持 `items_to_delete` / `items_to_add` |
| DELETE | `/api/v1/group/delete/:id` | 删除（级联 items） |

成功响应一律 `{ "data": ... }`。

### 客户端 OpenAI 兼容

#### `GET /v1/models`

- 有效 Key → `200` OpenAI list 格式，`data[].id` = **分组名**
- 无分组时 `data` 可为 `[]`
- 无凭证 / 坏 Key / 禁用 Key / **管理 JWT** → `401`（含顶层 `message`）

```json
{
  "object": "list",
  "data": [
    { "id": "my-group", "object": "model", "owned_by": "model-hub" }
  ]
}
```

#### `POST /v1/chat/completions`

- 客户端 Key 鉴权；管理 JWT → 401
- 请求体至少含 `model`（**分组名**）+ `messages`
- 路由：`mode=1` 轮询 items；其它 mode 暂取首个可用 item
- 加载 item 绑定渠道：需 enabled；取首个 base_url + 首个 enabled key
- 上游 URL：`{base_url 去尾斜杠}/chat/completions`
- 上游 Header：`Authorization: Bearer {channel_key}`；尽力合并 `custom_header`
- 改写上游 body：`model` → item.`model_name`
- **非流式**（`stream` 缺省/`false`）：上游状态码与 JSON body **透传**；网络失败 → 502
- **流式**（`stream: true`）：保留 `stream: true` 发给上游；成功时透传 `Content-Type: text/event-stream` 与字节流（不整包缓冲完整 SSE）；上游非 2xx 整包透传错误 body；网络失败 → 502
- 未知分组 → 404；无 items / 无可用渠道 → 400

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

# 创建渠道
curl.exe -s -X POST http://127.0.0.1:18081/api/v1/channel/create `
  -H "Authorization: Bearer TOKEN" -H "Content-Type: application/json" `
  -d "{\"name\":\"demo\",\"type\":0,\"enabled\":true,\"base_urls\":[{\"url\":\"https://api.openai.com/v1\",\"delay\":0}],\"keys\":[{\"enabled\":true,\"channel_key\":\"sk-test\",\"remark\":\"\"}],\"model\":\"gpt-4o-mini\",\"custom_model\":\"\",\"proxy\":false,\"auto_sync\":false,\"auto_group\":0,\"custom_header\":[]}"

# 创建分组（name = 客户端 model）
curl.exe -s -X POST http://127.0.0.1:18081/api/v1/group/create `
  -H "Authorization: Bearer TOKEN" -H "Content-Type: application/json" `
  -d "{\"name\":\"my-group\",\"mode\":1,\"match_regex\":\"\",\"items\":[{\"channel_id\":1,\"model_name\":\"gpt-4o-mini\",\"priority\":1,\"weight\":1}]}"

# 创建客户端 Key
curl.exe -s -X POST http://127.0.0.1:18081/api/v1/apikey/create `
  -H "Authorization: Bearer TOKEN" -H "Content-Type: application/json" `
  -d "{\"name\":\"local-client\",\"enabled\":true}"

# 客户端 models（替换 SK）
curl.exe -s http://127.0.0.1:18081/v1/models -H "Authorization: Bearer SK"

# 非流式 chat（model = 分组名）
curl.exe -s -X POST http://127.0.0.1:18081/v1/chat/completions `
  -H "Authorization: Bearer SK" -H "Content-Type: application/json" `
  -d "{\"model\":\"my-group\",\"messages\":[{\"role\":\"user\",\"content\":\"hi\"}]}"

# 流式 SSE（model = 分组名）
curl.exe -s -N -X POST http://127.0.0.1:18081/v1/chat/completions `
  -H "Authorization: Bearer SK" -H "Content-Type: application/json" `
  -d "{\"model\":\"my-group\",\"messages\":[{\"role\":\"user\",\"content\":\"hi\"}],\"stream\":true}"
```

## 鉴权矩阵

| 路径 | 无凭证 | 管理 JWT | 有效 sk | 无效/禁用 sk |
|------|--------|----------|---------|--------------|
| `/health` | 200 | 200 | 200 | 200 |
| `/api/v1/user/login` | 可调 | 可调 | 可调 | 可调 |
| `/api/v1/user/status` | 401 | 200 | 401 | 401 |
| `/api/v1/apikey/*` | 401 | 按业务 | 401 | 401 |
| `/api/v1/channel/*` | 401 | 按业务 | 401 | 401 |
| `/api/v1/group/*` | 401 | 按业务 | 401 | 401 |
| `/v1/models` | 401 | 401 | 200 | 401 |
| `/v1/chat/completions` | 401 | 401 | 按业务 | 401 |

## 与 octopus / Tauri 边界

| 项 | 约定 |
|----|------|
| 当前发布链路 | 仍使用内嵌 octopus；见 `gateway/README.md` |
| Tauri `GatewayRuntime` | **不修改** 默认侧车启动路径 |
| 数据目录 | SQLite 默认 `data/data.db`（相对 cwd）；schema 自有，非 1:1 复制 octopus |
| 进程清理 | 不按 `octopus` 进程名结束任何进程 |

## 测试

```powershell
cargo fmt --manifest-path gateway-rust/Cargo.toml -- --check
cargo check --manifest-path gateway-rust/Cargo.toml
cargo test --manifest-path gateway-rust/Cargo.toml
cargo clippy --manifest-path gateway-rust/Cargo.toml --all-targets -- -D warnings
```

集成测试绑定 `127.0.0.1:0` 随机端口 + 临时 SQLite，覆盖：

- 登录 → 建 Key → `/v1/models` 鉴权矩阵
- 登录 → 渠道 CRUD（keys_to_*）→ 分组 CRUD（items_to_*）→ apikey → models（分组名列表）
- API Key 跨进程实例持久化
- wiremock 上游 200 转发、SSE 多帧 + `[DONE]`、未知分组、轮询切换 model_name

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
│   ├── db/            # open / migrate v1
│   ├── auth/          # JWT / admin / middleware
│   ├── apikey/        # model / memory + sqlite store / hash
│   ├── channel/       # model / store / service
│   ├── group/         # model / store / service
│   ├── router/        # 分组 → 渠道 + model_name；轮询
│   ├── upstream/      # reqwest 非流式 + SSE 流式转发
│   └── routes/        # login / apikey / channel / group / v1 models / chat
└── tests/
    ├── http_server.rs
    ├── auth_matrix.rs
    ├── channel_group_matrix.rs
    └── chat_forward.rs
```
