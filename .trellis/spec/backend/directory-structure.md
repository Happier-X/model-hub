# Directory Structure

> 后端与桌面壳代码如何组织。

---

## Overview

单体仓库，当前桌面壳布局：

```
/
├── gateway/
│   └── README.md              # 侧车二进制钉扎、解析优先级、AGPL
├── third-party/octopus/       # AGPL 全文、NOTICE、对应源码链接
├── tools/octopus/             # 本地下载的 exe（gitignore，勿提交）
├── scripts/
│   ├── prepare-bundled-octopus.ps1  # 下载+SHA-256 校验
│   └── e2e-octopus-smoke.py
├── gateway-rust/              # 实验性 Rust 原生网关独立 crate（不接入当前发布链路）
│   ├── src/                   # config/http/server 可测试边界
│   └── tests/                 # 随机本机端口集成测试
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
│   │       └── binary.rs      # 解析优先级 + 内嵌部署
│   ├── capabilities/
│   ├── icons/
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   └── tauri.release.conf.json  # NSIS + bundle.resources 内嵌侧车
├── .github/workflows/release-windows.yml
├── src/                       # 前端 SPA
└── .trellis/
```

Windows 侧车解析优先级：`MODEL_HUB_GATEWAY_BIN` → （若存在）安装资源 `sidecar/octopus.exe` 按哈希原子部署到 `{bin_dir}/octopus.exe` → 否则直接使用已有 `bin_dir` 副本。安装态以内嵌为版本真源；勿将大型 exe 提交进 Git。

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

## 实验性 Rust 网关 crate 契约

### Scope / Trigger

修改 `gateway-rust/` 的配置、路由、监听或生命周期时遵守本节。该 crate 与 `src-tauri` 使用独立 `Cargo.toml/Cargo.lock`，暂不建立根 Cargo workspace，也不加入 Tauri 构建依赖。

### 稳定骨架契约

- CLI：`--config` 默认 `data/config.json`；文件缺失、损坏或配置非法必须非零退出，不得静默回退。
- 配置：消费 `server.host` / `server.port`（默认 `127.0.0.1:8080`；host 仅 IP，端口不得为 0）；可选 `auth`（`admin_username`/`admin_password`/`jwt_secret`/`jwt_default_expire_seconds`，缺省 admin/admin；`jwt_secret` 缺失时启动随机生成并 warning）。
- HTTP：`GET /health` 返回 `{status, service, version}` JSON 200；fallback 返回 `{error:{code,message}}` JSON 404。
- 鉴权与持久化（实验已实现）：
  - 管理：`POST /api/v1/user/login`（默认 admin/admin）签发 HS256 JWT；`GET /api/v1/user/status`、`/api/v1/apikey/*`、`/api/v1/channel/*`、`/api/v1/group/*` 需管理 JWT。
  - 成功管理/业务响应使用 `{ "data": ... }` 信封；鉴权失败 401 必须含顶层 `message`，并可保留 `error.{code,message}`。
  - 客户端：`GET /v1/models` 占位；`Authorization: Bearer sk-octopus-...` 或 `x-api-key`；管理 JWT 访问 `/v1/*` → 401。
  - API Key 前缀固定 `sk-octopus-`；完整明文仅 create 返回一次；默认 `SqliteApiKeyStore` 只存哈希 + 脱敏（`MemoryApiKeyStore` 仍可用于单测）。
  - SQLite：仅 `database.type=sqlite`；启动 `db::open_from_config` + migrate v1（api_keys/channels/channel_keys/channel_base_urls/groups/group_items）；相对路径相对 cwd。
  - 渠道：`type` 数字；create/list/update/enable/delete；update 支持 `keys_to_update` / `keys_to_add`；上游 channel_key 本机可明文，日志禁打完整 Key。
  - 分组：`mode` 数字（1=轮询）；create/list/update/delete；update 支持 `items_to_delete` / `items_to_add`。
  - 日志禁止打印完整 Token/客户端 Key/上游 Key。
- 生命周期：通过预绑定 `TcpListener` + graceful shutdown；测试只绑定 `127.0.0.1:0` + 临时库，不结束任何 octopus 进程。
- 模块边界：`config` / `http` / `server` / `error` / `db` / `auth` / `apikey` / `channel` / `group` / `routes` / `response` 保持可测试；后续 Chat 通过 router/state 扩展，不把业务塞进 Tauri 壳。
- 发布边界：当前 Tauri `GatewayRuntime` 与内嵌 octopus 链路保持不变；`gateway-rust` README 必须明确实验状态。

### 验证

```powershell
cargo fmt --manifest-path gateway-rust/Cargo.toml -- --check
cargo check --manifest-path gateway-rust/Cargo.toml
cargo test --manifest-path gateway-rust/Cargo.toml
cargo clippy --manifest-path gateway-rust/Cargo.toml --all-targets -- -D warnings
```

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
- `gateway_set_port(app, port: u32) -> Result<GatewayStatus, InvokeError>`

TypeScript：`gatewayStart` / `gatewayStop` / `gatewayStatus` / `gatewaySetPort`（`src/api/tauri.ts`）。

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

启动前写入 **`{gateway_dir}/data/config.json`**，并以 `octopus start --config data/config.json` 启动；注入 `OCTOPUS_SERVER_HOST/PORT` 等环境变量。工作目录为 `gateway_dir`。数据库默认相对路径 `data/data.db`。

用户端口设置以 **`{config_dir}/shell.json`** 的 `gateway_port` 为唯一持久化真源，缺失或损坏时安全回退 `8080`。仅 `idle` / `error` 状态允许保存 `1..=65535`；运行、启动或停止中必须提示先停止。保存后不自动重启，下次手动启动时同步用于状态、Base URL、侧车配置、环境变量和健康检查。不得自动选择端口或结束占用进程。

Windows 保存 `shell.json` 时必须使用同目录临时文件与备份恢复策略：旧配置移至 `.bak`，新文件替换失败时恢复旧配置；读取主文件失败或缺失时允许从 `.bak` 恢复。禁止直接删除旧配置后裸 `rename`，否则替换失败会丢失用户端口。写盘失败必须恢复完整 `GatewayStatus`，不能只回滚端口而丢失原错误状态。

### 4. Validation & Error Matrix

| 条件 | code / 行为 |
|------|-------------|
| 缺少 exe | `GATEWAY_BINARY_MISSING`，提示内置网关/prepare/`MODEL_HUB_GATEWAY_BIN` |
| 内嵌部署失败 | `GATEWAY_BINARY_DEPLOY_FAILED`，提示目标路径占用或权限 |
| 端口占用 | `GATEWAY_PORT_IN_USE` |
| 非法端口 | `GATEWAY_INVALID_PORT` |
| 运行中修改端口 | `GATEWAY_PORT_CHANGE_BLOCKED`，提示先停止 |
| 健康检查超时 | `GATEWAY_HEALTH_TIMEOUT`，清理残留子进程 |
| 配置写入失败 | `GATEWAY_CONFIG_FAILED` |
| 应用退出 | `RunEvent::Exit` 时 `stop_managed` |

### 5. Good / Base / Bad

- Good：设置页启停，状态条显示 running + port。
- Base：无 exe 时 UI 显示可行动错误，窗口仍可打开。
- Bad：前端写死 exe 绝对路径；默认监听 `0.0.0.0` 且无提示。

### 6. Tests Required

- config 默认 host/port/sqlite 与 env key 单测
- 端口 `18080` 同步到配置文件、环境变量、状态与 Base URL
- shell 配置缺失/损坏回退、端口 0 拒绝、备份恢复与替换失败旧值保留
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


## 应用内更新跨层契约

### 1. Scope / Trigger

新增或修改 Tauri Updater、GitHub Release 资产、签名密钥、更新 UI 或重启流程时必须同步本契约。

### 2. Signatures

- 前端：`checkForUpdate() -> Promise<UpdateInfo | null>`
- 前端：`downloadAndInstallUpdate(update, onProgress) -> Promise<void>`
- 前端：`relaunchAfterUpdate() -> Promise<void>`
- Rust 插件：`tauri-plugin-updater` + `tauri-plugin-process`

### 3. Contracts

- endpoint 固定为 `https://github.com/Happier-X/model-hub/releases/latest/download/latest.json`。
- 仅设置页手动检查正式 Release；启动时不联网。
- 公钥可提交，`TAURI_SIGNING_PRIVATE_KEY` 仅存 GitHub Actions Secret。
- Release 必须同时提供 NSIS、`.sig`、`latest.json`、SHA256 和合规材料。
- 更新重启沿用 `RunEvent::Exit -> stop_managed`，不得遗留托管侧车。
- app data（`shell.json`、SQLite、日志）不得随安装更新删除。

### 4. Validation & Error Matrix

| 条件 | 行为 |
|---|---|
| 无新版本 | 显示“当前已是最新版本” |
| 网络/manifest 失败 | 显示可行动错误，当前版本继续运行 |
| 签名不匹配 | Updater 拒绝安装，不允许绕过插件下载 exe |
| 用户取消 | 不下载、不安装或不重启 |
| CI 缺私钥 | workflow 构建前失败，不发布未签名 updater |

### 5. Good / Base / Bad Cases

- Good：用户点击检查、确认安装、签名通过、重启后数据保留。
- Base：当前最新版，无后台请求和弹窗。
- Bad：把私钥写进仓库，或前端 fetch exe 后直接执行。

### 6. Tests Required

- `pnpm lint/build`、`cargo fmt/test/check`。
- 发布基线版后，用下一 patch 验证 `latest.json`、`.sig` 和真实应用内升级。
- 错误签名测试必须确认拒绝安装。

### 7. Wrong vs Correct

```text
错误：Release 只有 setup.exe，客户端无法验证更新。
正确：tauri-action 生成并上传 setup.exe、.sig 和 latest.json，私钥来自 Secret。
```

---
## Anti-Patterns

- 在前端硬编码绝对路径到侧车 exe。
- 管理 API 与转发 API 混在同一无区分路由且无法文档化。
- 为「完整移植」在壳里重写一套与侧车重复的配置存储。
- 将 `octopus.exe` 无说明地提交进 Git。
