# 执行计划：Rust 网关鉴权与 API Key

## 清单

1. [x] 钉扎依赖：`jsonwebtoken`、`rand`、`sha2`、`hex`、`chrono`（若需 expire_at）
2. [x] 扩展 `GatewayConfig` 可选 `auth` 段与默认 admin/JWT 行为
3. [x] 实现 `response` 成功信封 + 401 统一体（含顶层 `message`）
4. [x] 实现 JWT 签发/校验与 admin 登录/status
5. [x] 实现 `ApiKeyStore` + 内存存储 + CRUD 路由
6. [x] 实现 `/v1/models` 客户端鉴权中间件（Bearer / x-api-key）
7. [x] 组装 `AppState` 与 router 分组；保持 `/health` 与 JSON 404
8. [x] 单测 + 随机端口集成鉴权矩阵
9. [x] 更新 `gateway-rust/README.md`、backend spec 鉴权契约
10. [x] fmt/check/test/clippy；确认 Tauri/octopus 路径未改；提交归档

## 验证

```powershell
cargo fmt --manifest-path gateway-rust/Cargo.toml -- --check
cargo check --manifest-path gateway-rust/Cargo.toml
cargo test --manifest-path gateway-rust/Cargo.toml
cargo clippy --manifest-path gateway-rust/Cargo.toml --all-targets -- -D warnings
```

手工 curl（随机/测试端口）：

```text
POST /api/v1/user/login
GET  /api/v1/user/status
POST /api/v1/apikey/create
GET  /v1/models  (bad key 401 / good key 200)
```

## 审查门

- 管理 JWT ≠ 客户端 Key
- 完整 Key 仅创建一次；store 无明文
- 401 带可解析 `message`
- 成功管理 API 使用 `{data:...}`
- 测试不占用 8080、不杀 octopus
- 不修改发布链路

## 回滚

回退 `gateway-rust` 鉴权模块与依赖，保留 HTTP 骨架即可。
