# model-hub-gateway（实验骨架）

> **状态：实验性。** 本 crate 是 Rust 原生网关的 **HTTP 服务骨架**，用于后续鉴权 / SQLite / 渠道 / Chat 切片开发。  
> **不能**替代当前 Model Hub 发布版内嵌的 **octopus v0.9.28** 侧车，也 **不会** 被 Tauri 壳默认拉起。

## 目标

- 独立进程，默认只绑定本机 `127.0.0.1`
- 配置文件契约对齐 octopus：`data/config.json` 的 `server.host` / `server.port`
- `GET /health` 稳定 JSON 200
- 未知路径 JSON 404（不返回 HTML）
- Ctrl-C 优雅退出；测试使用随机端口，不按进程名清理 octopus

## 运行

在仓库根目录：

```powershell
cargo run --manifest-path gateway-rust/Cargo.toml -- --config gateway-rust/testdata/config.json
```

默认配置路径为 `data/config.json`（相对当前工作目录）。**文件不存在会启动失败**，不会静默使用内存默认值。

```powershell
# 使用默认路径（需先准备 data/config.json）
cargo run --manifest-path gateway-rust/Cargo.toml
```

健康检查示例（端口以配置为准，testdata 为 `18081`）：

```powershell
curl.exe http://127.0.0.1:18081/health
curl.exe -i http://127.0.0.1:18081/unknown
```

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
  }
}
```

当前仅消费 `server`；`database` / `log` 保留字段以便与壳写入的配置兼容。校验规则：

- `host` 非空且为合法 IP（不做 DNS）
- `port != 0`
- 默认值为 `127.0.0.1:8080`（仅 `GatewayConfig::default()` / 测试）
- 显式配置 `0.0.0.0` / `::` 不静默改写，但会打安全 warning

## HTTP 契约（骨架）

### `GET /health`

```json
{
  "status": "ok",
  "service": "model-hub-gateway",
  "version": "0.1.0"
}
```

### 未知路径

```json
{
  "error": {
    "code": "NOT_FOUND",
    "message": "未找到请求的接口"
  }
}
```

## 与 octopus / Tauri 边界

| 项 | 约定 |
|----|------|
| 当前发布链路 | 仍使用内嵌 octopus；见 `gateway/README.md` |
| Tauri `GatewayRuntime` | **本任务不修改** 默认侧车启动路径 |
| 数据目录 | 不复制 Tauri app data 业务库；后续通过 HTTP 契约接入 |
| 进程清理 | 不按 `octopus` 进程名结束任何进程 |

## 测试

```powershell
cargo fmt --manifest-path gateway-rust/Cargo.toml -- --check
cargo check --manifest-path gateway-rust/Cargo.toml
cargo test --manifest-path gateway-rust/Cargo.toml
cargo clippy --manifest-path gateway-rust/Cargo.toml --all-targets -- -D warnings
```

集成测试绑定 `127.0.0.1:0` 随机端口，并用 oneshot 触发优雅退出。

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
│   └── server.rs
└── tests/
    └── http_server.rs
```
