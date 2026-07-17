# Directory Structure

> 后端与桌面壳代码如何组织。

---

## Overview

单体仓库，当前桌面壳布局：

```
/
├── gateway/
│   └── README.md              # 侧车二进制钉扎、放置路径、AGPL
├── src-tauri/                 # Tauri / Rust 壳
│   ├── src/
│   │   ├── main.rs
│   │   ├── lib.rs             # Builder、命令注册、退出停侧车
│   │   ├── paths.rs           # get_paths
│   │   ├── error.rs
│   │   └── gateway/           # 侧车进程管理
│   │       ├── mod.rs         # gateway_start/stop/status
│   │       ├── state.rs
│   │       ├── process.rs
│   │       ├── health.rs
│   │       ├── config.rs
│   │       └── binary.rs
│   ├── capabilities/
│   ├── icons/
│   ├── Cargo.toml
│   └── tauri.conf.json
├── src/                       # 前端 SPA
└── .trellis/
```

Windows 侧车默认可执行文件：`{bin_dir}/octopus.exe`；可用环境变量 `MODEL_HUB_GATEWAY_BIN` 覆盖。勿将大型 exe 提交进 Git。

---

## Module Organization

| 区域 | 放什么 | 不放什么 |
|------|--------|----------|
| `src-tauri/src/gateway/` | 子进程命令行、工作目录、环境变量、优雅退出、端口探测 | 业务路由、协议转换实现 |
| `src-tauri/src/paths.rs` | 解析 Windows 应用数据目录、配置/DB 路径 | 硬编码用户家目录零散字符串 |
| 侧车进程内部 | 渠道/分组/转发/SQLite | Tauri 窗口逻辑 |
| 管理 UI | 只通过 **HTTP** 调侧车管理 API + 少量 Tauri invoke（启停状态） | 把渠道 CRUD 做成只能走 invoke 的死绑 |

---

## Rules

1. **壳与网关边界**：业务状态以侧车 + SQLite 为准；壳不复制一份业务库。
2. **可替换侧车**：启动参数、数据目录、健康检查 URL 写成稳定契约，便于以后换 Rust 网关。
3. **Windows 路径**：使用官方 app data 约定（如 `%APPDATA%/<app>/` 或 Tauri `path` 插件），禁止写死 `C:\Users\某用户\...`。
4. **密钥**：上游 API Key 只存侧车数据/配置；日志禁止打印完整 Key。

---

## `get_paths` 跨层契约

### 1. Scope / Trigger

当新增或修改应用数据目录、Tauri invoke 返回字段、前端路径展示时，必须同步维护本契约。

### 2. Signatures

- Rust：`get_paths(app: AppHandle) -> Result<AppPaths, InvokeError>`（`src-tauri/src/paths.rs`）
- TypeScript：`getPaths(): Promise<AppPaths>`（`src/api/tauri.ts`）

### 3. Contracts

返回 JSON 字段统一使用 snake_case：

```json
{
  "app_data_dir": "<Tauri app_data_dir>",
  "config_dir": "<app_data_dir>/config",
  "gateway_dir": "<app_data_dir>/gateway",
  "bin_dir": "<app_data_dir>/bin"
}
```

命令调用前或调用中必须确保四个目录存在。前端不得自行拼接操作系统目录。

### 4. Validation & Error Matrix

| 条件 | 结果 |
|------|------|
| app data 可解析且可创建 | 返回四个绝对路径 |
| 目录创建失败 | 返回可序列化 invoke 错误，含失败路径，不含敏感信息 |
| 非 Tauri 浏览器预览 | 仅返回明确的「预览模式」占位值，不伪造真实系统路径 |

### 5. Good / Base / Bad Cases

- Good：设置页通过 `getPaths()` 展示真实路径。
- Base：浏览器 Vite 预览展示不可写的占位路径。
- Bad：前端写死 `%APPDATA%/model-hub` 或自行拼 `C:\\Users`。

### 6. Tests Required

- Rust 单元测试断言根路径派生 `config/gateway/bin`。
- `cargo test` 验证契约；前端构建确保字段类型一致。

### 7. Wrong vs Correct

```ts
// Wrong: 重复路径规则
const gateway = `${process.env.APPDATA}/model-hub/gateway`;

// Correct: 单一契约来源
const { gateway_dir: gateway } = await getPaths();
```

---

## 网关侧车 invoke 契约

### 1. Scope / Trigger

新增/修改侧车启停、健康检查、配置注入或前端网关状态展示时，必须同步本契约。

### 2. Signatures

- `gateway_start(app) -> Result<GatewayStatus, InvokeError>`
- `gateway_stop() -> Result<GatewayStatus, InvokeError>`
- `gateway_status(app) -> Result<GatewayStatus, InvokeError>`

TypeScript：`gatewayStart` / `gatewayStop` / `gatewayStatus`（`src/api/tauri.ts`）。

### 3. Contracts

`GatewayStatus` 字段（snake_case）：

| 字段 | 说明 |
|------|------|
| `state` | `idle` \| `starting` \| `running` \| `stopping` \| `error` |
| `host` | 默认 `127.0.0.1` |
| `port` | 默认 `8080` |
| `pid` | 可选 |
| `last_error` | 可行动错误文案 |
| `base_url` | `http://{host}:{port}` |
| `data_dir` | `gateway_dir` |
| `binary_path` | 解析到的 exe，可空 |

启动前写入 `{gateway_dir}/config.json`，并注入 `OCTOPUS_SERVER_HOST/PORT` 等环境变量。工作目录为 `gateway_dir`。

### 4. Validation & Error Matrix

| 条件 | code / 行为 |
|------|-------------|
| 缺少 exe | `GATEWAY_BINARY_MISSING`，提示放置路径或 `MODEL_HUB_GATEWAY_BIN` |
| 端口占用 | `GATEWAY_PORT_IN_USE` |
| 健康检查超时 | `GATEWAY_HEALTH_TIMEOUT`，清理残留子进程 |
| 配置写入失败 | `GATEWAY_CONFIG_FAILED` |
| 应用退出 | `RunEvent::Exit` 时 `stop_managed` |

### 5. Good / Base / Bad

- Good：设置页启停，状态条显示 running + port。
- Base：无 exe 时 UI 显示可行动错误，窗口仍可打开。
- Bad：前端写死 exe 绝对路径；默认监听 `0.0.0.0` 且无提示。

### 6. Tests Required

- config 默认 host/port/sqlite 与 env key 单测
- 二进制缺失错误单测
- 健康探测/端口可达单测
- 状态机字段单测

### 7. Wrong vs Correct

```ts
// Wrong
await fetch("http://127.0.0.1:8080/admin/start");

// Correct
await gatewayStart();
const { state, base_url } = await gatewayStatus();
```

---

## Anti-Patterns

- 在前端硬编码绝对路径到侧车 exe。
- 管理 API 与转发 API 混在同一无区分路由且无法文档化。
- 为「完整移植」在壳里重写一套与侧车重复的配置存储。
- 将 `octopus.exe` 无说明地提交进 Git。
