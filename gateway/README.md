# 网关侧车（octopus 兼容）

Model Hub 阶段 1 通过 **网关进程** 提供 LLM 聚合能力。桌面壳负责启停与健康检查，业务数据落在应用 `gateway_dir`。

**v0.0.1 起**：Windows 安装包**内嵌**钉扎版 octopus，用户**无需**自行下载或放置 `octopus.exe`。首次启动时从安装资源复制到 `bin_dir`（按 SHA-256 判断是否覆盖）。

## 许可证

上游项目 [bestruirui/octopus](https://github.com/bestruirui/octopus) 使用 **AGPL-3.0**。下载、修改或随本应用分发其二进制/源码时，请自行遵守 AGPL 义务（包括源码提供与许可证传递），并保留上游致谢。

完整合规材料见仓库：

- `third-party/octopus/LICENSE-AGPL-3.0.txt`
- `third-party/octopus/NOTICE.md`
- `third-party/octopus/SOURCE.md`（对应源码 archive URL + commit）

## 版本钉扎（已验证）

| 项 | 值 |
|----|-----|
| 版本 | **v0.9.28** |
| tag commit | `b7b053e7fd81911e2062359e93f9dcbd58114bb0` |
| Windows x64 包 | `octopus-windows-x86_64.zip` |
| 下载 | https://github.com/bestruirui/octopus/releases/download/v0.9.28/octopus-windows-x86_64.zip |
| Zip SHA-256 | `17b071b66218f15b574efe08c73b4ec56d6adfd9c08aab3b216728b29ac0f92f` |
| Exe SHA-256 | `38c4238c5c8be0d3e718eb6192c9d06b2e1dcb4222179f625627c67b1e98c0d8` |
| 对应源码 | https://github.com/bestruirui/octopus/archive/refs/tags/v0.9.28.tar.gz |
| 校验启动 | `octopus version` 显示 `Version: v0.9.28` |
| 启动 | `octopus start --config data/config.json`（工作目录为 gateway 数据目录） |

构建/开发机准备内嵌二进制（**不提交** exe）：

```powershell
powershell -ExecutionPolicy Bypass -File scripts/prepare-bundled-octopus.ps1
# 可选开发覆盖：
$env:MODEL_HUB_GATEWAY_BIN = "$PWD\tools\octopus\octopus.exe"
```

兼容旧脚本名：`scripts/fetch-octopus-windows.ps1`（转发到 prepare 脚本）。

## 实现切换（实验）

默认仍启动 **octopus**。可选通过环境变量切换到实验性 Rust 网关（`gateway-rust` / `model-hub-gateway`）：

| 变量 | 说明 |
|------|------|
| `MODEL_HUB_GATEWAY_IMPL` | 缺省 / `octopus` / 未知值 → octopus；`rust`（大小写不敏感）→ Rust 网关 |
| `MODEL_HUB_GATEWAY_BIN` | 任意实现下的最高优先级二进制覆盖（文件必须存在） |
| `MODEL_HUB_GATEWAY_RUST_BIN` | 仅 `impl=rust` 时使用；次于 `MODEL_HUB_GATEWAY_BIN` |

```powershell
# 默认：与现网一致（octopus）
# 实验：使用 Rust 网关
$env:MODEL_HUB_GATEWAY_IMPL = "rust"
cargo build --manifest-path gateway-rust/Cargo.toml --release
$env:MODEL_HUB_GATEWAY_RUST_BIN = "$PWD\gateway-rust\target\release\model-hub-gateway.exe"
# 或复制到 app data bin_dir 下的 model-hub-gateway.exe
```

启动参数差异：

| 实现 | 命令（cwd=`gateway_dir`） |
|------|---------------------------|
| octopus | `{bin} start --config data/config.json` + `OCTOPUS_*` |
| rust | `{bin} --config data/config.json`（不注入 `OCTOPUS_*`） |

两种实现共用壳写入的 `data/config.json`（host/port/sqlite 路径）以及端口占用、健康检查、停止托管子进程逻辑。

> **警告：勿混用同一 SQLite。** octopus 与 `gateway-rust` 的数据库 schema **不兼容**。在同一 `gateway_dir` 下切换实现可能导致启动失败或数据损坏。切换前请备份/清空 `data/data.db`，或使用独立数据目录。生产用户请保持默认 octopus。

发布安装包 **默认仍内嵌 octopus**；本仓库当前 **不会** 把 `model-hub-gateway` 打进 NSIS。

## 二进制解析优先级

### octopus（默认）

1. 环境变量 `MODEL_HUB_GATEWAY_BIN`（开发/高级覆盖；须指向存在的文件）
2. 若安装资源存在 `resource_dir/sidecar/octopus.exe`：以内嵌为准，**原子部署**到 `bin_dir/octopus.exe`（目标不存在或 SHA-256 不同则覆盖；相同则跳过复制），然后使用 `bin_dir` 副本
3. 否则（开发未内嵌）：直接使用已存在的 `bin_dir/octopus.exe`
4. 仍缺失 → 设置页可行动错误；窗口仍可打开

说明：安装态下内嵌资源是版本真源；`bin_dir` 是运行时部署目标（非独立于内嵌的更高优先项）。

### rust（`MODEL_HUB_GATEWAY_IMPL=rust`）

1. `MODEL_HUB_GATEWAY_BIN`（与上相同，两种实现通用覆盖）
2. `MODEL_HUB_GATEWAY_RUST_BIN`（须指向存在的文件）
3. `bin_dir/model-hub-gateway.exe`
4. 仍缺失 → 可行动错误（提示 `cargo build --manifest-path gateway-rust/Cargo.toml`）；窗口仍可打开，壳不崩溃

## 配置约定（壳侧写入）

工作目录：`gateway_dir`  
配置文件：**`data/config.json`**（不是根目录 `config.json`）  
数据库相对路径：**`data/data.db`**

示例：

```json
{
  "server": { "host": "127.0.0.1", "port": 8080 },
  "database": { "type": "sqlite", "path": "data/data.db" },
  "log": { "level": "info" }
}
```

环境变量覆盖（与上游一致）：`OCTOPUS_SERVER_HOST`、`OCTOPUS_SERVER_PORT`、`OCTOPUS_DATABASE_TYPE`、`OCTOPUS_DATABASE_PATH`、`OCTOPUS_LOG_LEVEL`。

## 本产品强制约定

| 项 | 约定 |
|----|------|
| 监听地址 | 默认 **`127.0.0.1`** |
| 端口 | 默认 **8080** |
| 管理 UI | Model Hub **无登录页**；静默 `POST /api/v1/user/login`（默认 admin/admin） |
| 客户端网关 Key | **必须**使用侧车签发的 `sk-octopus-...`（应用 **API 密钥** 页创建）；与管理 JWT 分离 |

## 两套凭证

| 路径 | 鉴权 | 说明 |
|------|------|------|
| `/api/v1/*` | Bearer **管理 JWT** | 渠道/分组/API Key/日志等管理接口 |
| `/v1/*` | Bearer **`sk-octopus-...`** 或 `x-api-key` | OpenAI 兼容客户端（models / chat 等） |

管理创建 Key：

- `POST /api/v1/apikey/create` body：`{"name":"local-client","enabled":true}`
- `GET /api/v1/apikey/list` / `POST /api/v1/apikey/update` / `DELETE /api/v1/apikey/delete/:id`

## 联调发现（v0.9.28）

1. **配置路径**：必须 `data/config.json` + `start --config data/config.json`。
2. **渠道 `type` 字段**：该版本二进制绑定为 **数字**；传字符串会返回 `Invalid JSON format`。OpenAI Chat 使用 **`type: 0`**。
3. **对外 `/v1/*`**：校验网关 API Key（前缀 `sk-octopus-`）。错误/占位 Key → **401**；正确 Key → `/v1/models` 可 200（data 可为空）。
4. **清理进程**：开发脚本应只结束测试端口/测试 PID，不要 `Stop-Process -Name octopus`，以免误杀你本机正式实例。

## 启停

- 应用内：设置页「启动网关 / 停止网关」
- 系统托盘：显示 / 启动网关 / 停止网关 / 退出
- **关闭主窗口**：默认隐藏到托盘，**不**停止网关
- **托盘「退出」或应用真正退出**：壳结束**其托管的**子进程（不按进程名杀全机 octopus）

## 故障排查

1. 未找到 exe → 安装版应自带；开发跑 prepare 脚本，或放到 `bin_dir` / 设置 `MODEL_HUB_GATEWAY_BIN`
2. 端口占用 → 换端口或结束占用进程（不要误杀其他 octopus）
3. 管理 API 401 → 设置页粘贴 Token，或确认默认 admin 未改密
4. 创建渠道 Invalid JSON → 确认使用数字 `type`（前端已按 v0.9.28 适配）
5. 客户端 `/v1/*` 401 → 到 **API 密钥** 页创建 Key，并确认 Header 使用完整 `sk-octopus-...`（不是管理 JWT）
