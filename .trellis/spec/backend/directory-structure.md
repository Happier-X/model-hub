# Directory Structure

> 后端与桌面壳代码如何组织（内嵌代理架构）。

---

## Overview

```
/
├── src-tauri/                 # Tauri / Rust（壳 + 代理）
│   ├── src/
│   │   ├── main.rs
│   │   ├── lib.rs             # Builder、命令注册、退出停代理
│   │   ├── paths.rs           # get_paths
│   │   ├── settings.rs        # shell.json 端口
│   │   ├── error.rs
│   │   ├── commands.rs        # IPC：代理 + 领域 CRUD
│   │   ├── tray.rs
│   │   ├── db/                # SQLite 打开与 migrate
│   │   ├── domain/            # provider / group / log / leaderboard
│   │   └── proxy/             # 进程内 HTTP 代理
│   │       ├── runtime.rs     # ProxyHandle 启停/状态
│   │       ├── server.rs      # axum 路由 /health /v1/*
│   │       ├── forward.rs     # 上游转发 + 故障转移
│   │       └── circuit.rs     # 默认熔断
│   ├── tests/                 # 集成测（无鉴权访问/故障转移）
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   └── tauri.release.conf.json
├── src/                       # Vue 3 管理 UI
└── .trellis/
```

---

## Module Organization

| 区域 | 放什么 | 不放什么 |
|------|--------|----------|
| `proxy/` | 监听、转发、熔断、流式 prime（无客户端 Key 鉴权） | Vue UI |
| `domain/` | CRUD 与 SQLite 读写 | HTTP 路由细节 |
| `commands.rs` | Tauri invoke 薄封装 | 长业务逻辑（下沉 domain/proxy） |
| `paths.rs` / `settings.rs` | 目录与端口持久化 | 业务表 |

---

## Rules

1. **业务真源**：SQLite 在 `gateway_dir`（或应用数据目录下约定路径）；壳不复制第二份业务库。
2. **管理 vs 客户端**：管理走 IPC；外部客户端只走本机 HTTP `/v1/*`。
3. **Windows 路径**：用 Tauri `path` / `get_paths`，禁止写死用户家目录。
4. **密钥**：上游 Provider Key 本机可明文；不存不校验客户端 Key；日志禁打完整 Key。

---

## 跨层契约：`get_paths`

### Signatures

- Rust：`get_paths(app) -> Result<AppPaths, InvokeError>`
- TS：`getPaths(): Promise<AppPaths>`（`src/api/tauri.ts`）

### Contracts

```json
{
  "app_data_dir": "...",
  "config_dir": ".../config",
  "gateway_dir": ".../gateway",
  "bin_dir": ".../bin"
}
```

---

## 跨层契约：代理启停 IPC

### Signatures

- `proxy_start` / `proxy_stop` / `proxy_status` / `proxy_set_port`
- 领域：`list_providers` / `create_provider` / … / `list_groups` / … / `list_logs` / `clear_logs` / `list_health`

### ProxyStatus（snake_case）

| 字段 | 说明 |
|------|------|
| `state` | `idle` \| `starting` \| `running` \| `stopping` \| `error` |
| `host` | 默认 `127.0.0.1` |
| `port` | 默认 `8888` |
| `last_error` | 可行动错误 |
| `base_url` | `http://{host}:{port}` |
| `data_dir` | 代理数据目录 |

端口持久化：`{config_dir}/shell.json` 的 `gateway_port`。

打开应用：`try` 自动 `proxy.start`；`RunEvent::Exit` / 托盘「退出」时 `stop`（graceful + 超时 abort）；`ProxyHandle` Drop best-effort stop。

**单实例**（桌面）：`tauri-plugin-single-instance` 在 setup（含 `proxy.start`）之前注册；第二实例通知第一实例 `show_main_window` 后退出，不启第二套代理。

**关窗 vs 退出**：关窗隐藏到托盘，代理继续；仅托盘「退出」停止代理并释放端口。

---

## 客户端 HTTP 契约（MVP）

| 路径 | 鉴权 | 行为 |
|------|------|------|
| `GET /health` | 无 | 200 JSON |
| `GET /v1/models` | 无（忽略客户端鉴权头） | 分组名列表 |
| `POST /v1/chat/completions` | 无（忽略客户端鉴权头） | `model`=分组名；故障转移队列转发 |

---

## Anti-Patterns

- 再引入独立代理进程作为默认运行时。
- 管理 CRUD 只暴露不可调试的隐式全局状态。
- 提交 `tools/**/*.exe` 到 Git。
