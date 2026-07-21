# 设计：Rust 网关鉴权与 API Key

## 模块划分

```text
gateway-rust/src/
├── auth/
│   ├── mod.rs
│   ├── jwt.rs          # HS256 签发/校验
│   ├── admin.rs        # 默认 admin 校验
│   └── middleware.rs   # 提取 Bearer / x-api-key
├── apikey/
│   ├── mod.rs
│   ├── model.rs        # ApiKeyRecord / Create/Update
│   ├── store.rs        # ApiKeyStore trait + MemoryApiKeyStore
│   └── service.rs      # 生成 sk-octopus-、hash、CRUD
├── routes/
│   ├── mod.rs
│   ├── admin_user.rs   # login / status
│   ├── apikey.rs       # CRUD
│   └── v1_models.rs    # 占位 /v1/models
├── response.rs         # 成功信封 + 统一 401
└── http.rs             # 组装 router + state
```

## 依赖

在现有依赖上增加：

- `jsonwebtoken`：管理 JWT
- `sha2` + `rand`：Key 生成与哈希
- `hex` 或 base64：编码哈希
- `tower-http`（可选，仅若需要 layer 工具）
- `uuid`（可选；优先用随机字节 + 前缀）

依赖继续精确钉版本并更新 `Cargo.lock`。

## 配置扩展

`config.json` 可选扩展（缺省使用安全默认）：

```json
{
  "server": { "host": "127.0.0.1", "port": 8080 },
  "auth": {
    "admin_username": "admin",
    "admin_password": "admin",
    "jwt_secret": "dev-only-change-me",
    "jwt_default_expire_seconds": 86400
  }
}
```

- `jwt_secret` 缺省：进程启动时随机生成并 warning「重启后 Token 失效」（实验可接受）。
- 不在日志打印 secret/password。

## HTTP 契约（对齐 UI）

### 成功信封

```json
{ "data": <T> }
```

`gatewayHttp` 在存在 `data` 字段时解包。

### 登录

`POST /api/v1/user/login`

请求：

```json
{ "username": "admin", "password": "admin", "expire": 86400 }
```

响应 200：

```json
{
  "data": {
    "token": "<jwt>",
    "expire_at": "<RFC3339 或 unix 字符串/数字，UI 不强依赖解析>"
  }
}
```

失败 401：

```json
{
  "message": "用户名或密码错误",
  "error": { "code": "UNAUTHORIZED", "message": "用户名或密码错误" }
}
```

### 状态

`GET /api/v1/user/status` + `Authorization: Bearer <jwt>`

200：

```json
{ "data": "ok" }
```

### API Key

字段对齐前端 `ApiKey`：

```ts
{ id, name, api_key, enabled, expire_at?, max_cost?, supported_models? }
```

- create：生成 `sk-octopus-` + 高熵随机；响应 `api_key` 为完整明文一次。
- list：`api_key` 返回脱敏值（例如 `sk-octopus-****abcd`），**不是**完整明文。
- update：支持 `name`/`enabled` 等；不可通过 update 取回完整 Key。
- delete：按 id 删除。

### `/v1/models`

鉴权通过后返回 OpenAI 风格占位：

```json
{ "object": "list", "data": [] }
```

（客户端路径可不包 `{data:}` 管理信封，与 OpenAI 兼容；`clientProbe` 不依赖信封。）

## 鉴权矩阵

| 路径 | 无凭证 | 管理 JWT | 有效 sk | 无效/禁用 sk |
|------|--------|----------|---------|--------------|
| `/health` | 200 | 200 | 200 | 200 |
| `/api/v1/user/login` | 可调 | 可调 | 可调 | 可调 |
| `/api/v1/user/status` | 401 | 200 | 401 | 401 |
| `/api/v1/apikey/*` | 401 | 按业务 | 401 | 401 |
| `/v1/models` | 401 | 401 | 200 | 401 |

## 存储

```rust
trait ApiKeyStore: Send + Sync {
  fn list(&self) -> Vec<ApiKeyPublic>;
  fn create(&self, ...) -> Result<ApiKeyCreated>;
  fn update(&self, ...) -> Result<()>;
  fn delete(&self, id: i64) -> Result<()>;
  fn find_by_raw_key(&self, raw: &str) -> Option<ApiKeyRecord>;
}
```

- `MemoryApiKeyStore`：`Arc<Mutex<...>>` 或 `RwLock`。
- 哈希：`SHA-256(raw_key)` hex；校验时恒定时间比较优先（`subtle` 可选）。
- 完整 Key 仅在 create 返回结构体中出现，不写入 store 字段。

## JWT

- HS256；claims：`sub=admin`、`exp`、`iat`。
- middleware 从 `Authorization: Bearer` 提取。
- 客户端 middleware：Bearer 或 `x-api-key`；若值像 JWT（三段点分）且校验通过，仍**拒绝**作为客户端 Key（按矩阵：管理 JWT → `/v1/*` 401）。实现上更简单：客户端路径只接受前缀 `sk-octopus-`。

## AppState

```rust
pub struct AppState {
  pub config: Arc<GatewayConfig>,
  pub auth: Arc<AuthService>,
  pub api_keys: Arc<dyn ApiKeyStore>,
}
```

router 分组：

- public: `/health`, `/api/v1/user/login`
- admin layer: `/api/v1/user/status`, `/api/v1/apikey/*`
- client layer: `/v1/models`

## 测试

- 单元：密码校验、JWT 签发校验、Key 生成前缀、哈希不存明文、脱敏。
- 集成：随机端口完整登录→建 Key→models 鉴权矩阵。
- 禁止固定 8080；不操作 octopus 进程。

## 与壳/前端边界

- 不改 `src/`、`src-tauri/` 默认行为。
- README 更新鉴权实验状态与 curl 示例。
- 后续 SQLite 任务替换 Memory store，路径契约不变。
