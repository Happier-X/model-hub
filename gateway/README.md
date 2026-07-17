# 网关侧车（octopus 兼容）

Model Hub 阶段 1 通过 **外部网关进程** 提供 LLM 聚合能力。桌面壳负责启停与健康检查，业务数据落在应用 `gateway_dir`。

## 许可证

上游项目 [bestruirui/octopus](https://github.com/bestruirui/octopus) 使用 **AGPL-3.0**。下载、修改或随本应用分发其二进制/源码时，请自行遵守 AGPL 义务（包括源码提供与许可证传递），并保留上游致谢。

## 版本钉扎（已验证）

| 项 | 值 |
|----|-----|
| 版本 | **v0.9.28** |
| Windows x64 包 | `octopus-windows-x86_64.zip` |
| 下载 | https://github.com/bestruirui/octopus/releases/download/v0.9.28/octopus-windows-x86_64.zip |
| 校验启动 | `octopus version` 显示 `Version: v0.9.28` |
| 启动 | `octopus start --config data/config.json`（工作目录为 gateway 数据目录） |

开发机一键下载（不提交 exe）：

```powershell
powershell -ExecutionPolicy Bypass -File scripts/fetch-octopus-windows.ps1
$env:MODEL_HUB_GATEWAY_BIN = "$PWD\tools\octopus\octopus.exe"
```

## 放置路径

- `bin_dir/octopus.exe`（应用数据目录下的程序目录）
- 或环境变量 `MODEL_HUB_GATEWAY_BIN` 指向完整 exe 路径

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
- 退出应用：壳尝试结束**其托管的**子进程

## 故障排查

1. 未找到 exe → 下载脚本或放到 `bin_dir` / 设置 `MODEL_HUB_GATEWAY_BIN`
2. 端口占用 → 换端口或结束占用进程（不要误杀其他 octopus）
3. 管理 API 401 → 设置页粘贴 Token，或确认默认 admin 未改密
4. 创建渠道 Invalid JSON → 确认使用数字 `type`（前端已按 v0.9.28 适配）
5. 客户端 `/v1/*` 401 → 到 **API 密钥** 页创建 Key，并确认 Header 使用完整 `sk-octopus-...`（不是管理 JWT）
