# 网关（gateway-rust）

Model Hub 通过 **Rust 原生网关**（`model-hub-gateway`）提供 LLM 聚合能力。桌面壳负责启停与健康检查，业务数据落在应用 `gateway_dir`。

Windows 安装包**仅内嵌** `sidecar/model-hub-gateway.exe`，用户**无需**自行准备网关二进制。首次启动时从安装资源按 SHA-256 部署到 `bin_dir`。

## 开发准备

二进制**不进 Git**。

```powershell
pnpm prepare:gateway-rust
# 或
powershell -ExecutionPolicy Bypass -File scripts/prepare-bundled-gateway-rust.ps1
# 可选覆盖：
$env:MODEL_HUB_GATEWAY_BIN = "$PWD\tools\gateway-rust\model-hub-gateway.exe"
# 或
$env:MODEL_HUB_GATEWAY_RUST_BIN = "$PWD\tools\gateway-rust\model-hub-gateway.exe"
```

## 二进制解析优先级

1. 环境变量 `MODEL_HUB_GATEWAY_BIN`（须指向存在的文件）
2. 环境变量 `MODEL_HUB_GATEWAY_RUST_BIN`
3. 若安装资源存在 `resource_dir/sidecar/model-hub-gateway.exe`：按哈希部署到 `bin_dir/model-hub-gateway.exe`
4. 否则使用已有 `bin_dir/model-hub-gateway.exe`
5. 仍缺失 → 设置页可行动错误

## 启动命令

```text
model-hub-gateway --config data/config.json
```

工作目录为 `gateway_dir`。壳写入同一配置文件。

## 配置约定

工作目录：`gateway_dir`  
配置文件：**`data/config.json`**  
数据库相对路径：**`data/data.db`**

```json
{
  "server": { "host": "127.0.0.1", "port": 8080 },
  "database": { "type": "sqlite", "path": "data/data.db" },
  "log": { "level": "info" }
}
```

## 本产品强制约定

| 项 | 约定 |
|----|------|
| 监听地址 | 默认 **`127.0.0.1`** |
| 端口 | 默认 **8080**（设置页保存后自动重启生效） |
| 管理 UI | **无登录页**；静默 `POST /api/v1/user/login`（默认 admin/admin） |
| 客户端网关 Key | 前缀 **`sk-modelhub-...`**；与管理 JWT 分离 |

## 两套凭证

| 路径 | 鉴权 |
|------|------|
| `/api/v1/*` | Bearer **管理 JWT** |
| `/v1/*` | Bearer **`sk-modelhub-...`** 或 `x-api-key` |

## 启停

- 应用内：设置页「启动网关 / 停止网关」
- 系统托盘：显示 / 启动网关 / 停止网关 / 退出
- **关闭主窗口**：默认隐藏到托盘，**不**停止网关
- **托盘「退出」**：结束壳**托管的**网关子进程（不按进程名杀全机）

## 故障排查

1. 未找到 exe → 安装版应自带；开发跑 `pnpm prepare:gateway-rust` 或设置 env
2. 端口占用 → 设置页换端口或结束占用进程
3. 管理 API 401 → 设置页粘贴 Token，或确认默认 admin 未改密
4. 客户端 `/v1/*` 401 → **API 密钥** 页创建完整 `sk-modelhub-...`
