# Rust 网关鉴权与 API Key

## Goal

在 `gateway-rust` 上实现**管理 JWT** 与**客户端 API Key** 两套凭证分离的最小鉴权闭环，兼容现有 Model Hub UI 的登录/API Key 路径契约，为后续渠道/分组/Chat 子任务提供可复用鉴权中间件与存储边界。

## Background

- HTTP 骨架已完成：`GET /health`、JSON 404、配置、优雅退出。
- 现网 UI 依赖：
  - 静默 `POST /api/v1/user/login`（默认 `admin`/`admin`）
  - `GET /api/v1/user/status` 校验管理 Token
  - API Key CRUD：`/api/v1/apikey/*`
  - 客户端 `GET /v1/models` 等使用 Bearer `sk-octopus-...`（管理 JWT 不得当客户端 Key）
- 本任务仍**不**替换 Tauri/octopus 发布链路；仅扩展实验 crate。

## Requirements

### R1. 管理鉴权

- `POST /api/v1/user/login`：body `{ username, password, expire? }`，默认账号 `admin`/`admin`。
- 成功返回 UI 可解析的 `data.token` / `data.expire_at`（包在成功信封内）。
- `GET /api/v1/user/status`：需要有效管理 JWT；无效/缺失 → 401。
- 管理 JWT 不得作为客户端 `/v1/*` 凭证使用。

### R2. 客户端 API Key

- 前缀固定 **`sk-octopus-`**（与现网文档/UI 一致）。
- 管理端 CRUD（均需管理 JWT）：
  - `GET /api/v1/apikey/list`
  - `POST /api/v1/apikey/create` body 至少 `{ name, enabled? }`
  - `POST /api/v1/apikey/update` body 至少 `{ id, ... }`
  - `DELETE /api/v1/apikey/delete/:id`
- 创建响应中 `api_key` **仅完整返回一次**；列表中可返回脱敏/存档形态，但字段名仍为 `api_key`。
- 存储**不得**明文落盘完整 Key（内存/文件均只存哈希 + 前缀/后缀元数据）。

### R3. 客户端路径鉴权

- 占位路由：`GET /v1/models`（返回空列表或稳定占位 JSON，**不**做上游转发）。
- 鉴权：`Authorization: Bearer sk-octopus-...` 或 `x-api-key`。
- 错误/占位/禁用 Key → **401**；有效 Key → **非 401**（期望 200）。
- 管理 JWT 访问 `/v1/models` → 401。

### R4. 错误与信封

- 成功管理/业务响应使用 `{ "data": ... }`，以兼容 `gatewayHttp` 解包。
- 鉴权失败 401 JSON 必须包含前端可提取的 `message` 字段；同时可保留 `error.code/message`。
- `/health` 与未知路径行为保持骨架契约。

### R5. 安全与边界

- 默认仅本机绑定（继承骨架）。
- 日志禁止打印完整 Token/Key。
- 存储层抽象为 trait，本任务可用**内存实现**；SQLite 迁移留给后续数据模型任务。
- 不修改 Tauri 壳启停、不修改前端、不改发布 workflow。

## Acceptance Criteria

- [x] AC1：默认 admin 登录成功并拿到 JWT；错误密码 401。
- [x] AC2：无 Token / 坏 Token 访问 `user/status` 与 apikey 路由 401。
- [x] AC3：可创建 `sk-octopus-...` Key；list/update/delete 可用；完整 Key 仅创建时返回。
- [x] AC4：坏 Key / 管理 JWT 访问 `GET /v1/models` → 401；有效 Key → 200。
- [x] AC5：密钥不以明文持久化；单测/集成测覆盖鉴权矩阵；fmt/check/test/clippy 通过。
- [x] AC6：不改变现有 octopus/Tauri 发布与启停行为。

## Out of Scope

- 渠道/分组/Chat/SSE/日志业务。
- 真实 SQLite schema 与数据迁移。
- 改密 UI、多用户、RBAC。
- Tauri 切换 `MODEL_HUB_GATEWAY_IMPL`。
