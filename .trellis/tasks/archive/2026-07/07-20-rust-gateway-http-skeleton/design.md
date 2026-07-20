# 设计：Rust 网关 HTTP 服务骨架

## 目录与 crate 边界

新增独立 crate：

```text
gateway-rust/
├── Cargo.toml
├── README.md
├── src/
│   ├── lib.rs       # 可测试的 config/app/server API
│   ├── main.rs      # CLI、tracing、信号与退出码
│   ├── config.rs    # JSON 配置读取/校验
│   ├── error.rs     # 启动配置错误
│   ├── http.rs      # axum router/state/error response
│   └── server.rs    # listener + graceful shutdown
└── tests/
    └── http_server.rs
```

本子任务不把 crate 加入 Tauri 的构建依赖，避免改变当前 Windows 发布体积和侧车路径。根仓库暂不建立 Cargo workspace，以免影响 `src-tauri` 的现有 lock/构建；各自使用独立 `Cargo.toml/Cargo.lock`。

## 技术选择

- `tokio`：异步 runtime、signal、TCP listener。
- `axum`：HTTP router、JSON response、fallback。
- `serde` / `serde_json`：配置和响应。
- `thiserror`：配置/启动错误。
- `clap`：`--config` 参数。
- `tracing` / `tracing-subscriber`：无密钥结构化日志。

依赖使用精确版本或提交 lock 文件，遵循仓库钉版本要求。

## 配置契约

沿用 octopus 外层结构，以便后续壳切换：

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
  }
}
```

本骨架只消费 `server`；`database/log` 可通过 serde 默认/忽略未知字段保持向前兼容。配置文件缺失时：命令默认路径仍是 `data/config.json`，但文件不存在则启动失败，而不是悄悄用默认值。代码层的 `GatewayConfig::default()` 供测试和未来显式默认配置使用。

校验：

- `host` 非空。
- `port != 0`。
- `host.parse::<IpAddr>()` 成功；当前骨架只接受 IP，不做 DNS 解析。
- `0.0.0.0` / `::` 不禁止，但产生安全 warning；默认固定 loopback。若严格产品约束需要禁止公网绑定，后续壳 UI 决策处理。本子任务保持显式配置可测试，不静默改值。

## HTTP 契约

### `GET /health`

```json
HTTP/1.1 200
Content-Type: application/json

{
  "status": "ok",
  "service": "model-hub-gateway",
  "version": "0.1.0"
}
```

版本来自 `env!("CARGO_PKG_VERSION")`。

### Fallback

```json
HTTP/1.1 404
Content-Type: application/json

{
  "error": {
    "code": "NOT_FOUND",
    "message": "未找到请求的接口"
  }
}
```

未来业务错误沿用 `{ error: { code, message } }`，不得暴露内部路径/密钥。

## 服务器 API

```rust
pub fn build_router(state: AppState) -> Router;
pub async fn serve(listener: TcpListener, app: Router, shutdown: impl Future) -> io::Result<()>;
pub async fn run(config: GatewayConfig) -> Result<(), GatewayError>;
```

测试通过预先绑定 `127.0.0.1:0` 获取随机端口，调用 `serve` 并用 oneshot channel 触发退出。

## 生命周期

`main`：

1. 解析 CLI。
2. 读取/校验配置。
3. 初始化 tracing。
4. 绑定 listener。
5. `axum::serve(...).with_graceful_shutdown(shutdown_signal())`。
6. Windows/Unix 均至少处理 `tokio::signal::ctrl_c()`；Windows Ctrl-Break 可作为后续增强，骨架不引入平台专用 unsafe。

优雅关闭设置有限等待由 axum/tokio 连接行为控制；本任务无数据库/后台 worker。

## 测试设计

- config：合法、缺失、损坏、空 host、端口 0、默认 loopback。
- router：使用 `tower::ServiceExt::oneshot` 测 `/health` 与 fallback。
- server integration：随机端口启动、HTTP 请求、oneshot shutdown、task 正常结束。
- 禁止使用固定 8080，避免干扰用户现有 octopus。

## 兼容与回滚

- 不修改 `src-tauri/src/gateway/*`。
- 不修改 release workflow、Tauri resources 或 `MODEL_HUB_GATEWAY_BIN`。
- 回滚仅删除 `gateway-rust/` 和文档/spec 记录，现有应用无行为变化。
