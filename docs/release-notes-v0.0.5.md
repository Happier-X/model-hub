# Model Hub v0.0.5 发布说明

## 摘要

彻底移除 octopus 相关运行时兼容与命名残留，仅保留 Rust 原生网关 `model-hub-gateway`。客户端 API Key 前缀改为 **`sk-modelhub-`**。

## 主要变更

| 项 | 说明 |
|----|------|
| 网关 | 仅 `model-hub-gateway`；无 `MODEL_HUB_GATEWAY_IMPL=octopus`、无 octopus 二进制解析 |
| 客户端 Key | 前缀 **`sk-modelhub-...`**（不再生成/接受 `sk-octopus-...`） |
| 迁移 CLI | 删除 `migrate-octopus` |
| 仓库材料 | 删除 `third-party/octopus/` 归档目录 |

## 破坏性变更（必读）

1. **旧客户端 Key 全部失效**：请在应用 **API 密钥** 页重新创建，并更新客户端配置中的 `Authorization` / `api_key`。
2. **无法再通过 CLI 从历史 octopus SQLite 一键导入**；需要手工在 UI 重建渠道/分组/密钥，或自行处理数据。
3. 若本机仍有旧 `tools/octopus/` 缓存，产品代码不会使用它。

## 安装包内容

- Windows NSIS 安装程序 + Updater 签名资产（`.sig`）+ `latest.json` + `SHA256SUMS.txt`
- 内嵌 `sidecar/model-hub-gateway.exe`
- **不含** octopus 二进制与 AGPL 发布附件

## 升级建议

| 来源版本 | 建议 |
|----------|------|
| v0.0.3 / v0.0.4 | 可用应用内「检查更新」；更新后**重建客户端 Key** |
| v0.0.1 / v0.0.2 | 先手动安装本版或带 Updater 的基线，再使用应用内更新 |

## 验证提示

1. 安装后启动网关，设置页状态为运行中。
2. 创建渠道、分组与 **`sk-modelhub-...`** Key。
3. `curl` / 客户端请求 `/v1/models` 与 `/v1/chat/completions` 使用新 Key。

## 已知限制

- Windows 安装包未做代码签名，可能触发 SmartScreen「未知发布者」提示。
- 默认管理账号仍为 `admin` / `admin`（仅本机绑定场景）。
