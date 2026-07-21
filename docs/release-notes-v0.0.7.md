# Model Hub v0.0.7 发布说明

## 摘要

本机桌面场景改为**本地开放模式**：管理 API 与客户端 `/v1/*` 均**不再要求**登录、管理 JWT 或 `sk-modelhub-...` API Key。

## 主要变更

| 项 | 说明 |
|----|------|
| 管理鉴权 | 移除静默 admin 登录、设置页粘贴管理 Token |
| 客户端鉴权 | `/v1/models`、`/v1/chat/completions` 无需 API Key |
| UI | 删除「API 密钥」页；仪表盘清单简化为启动网关 → 渠道 → 分组 |
| 文档 | curl 模板与对接说明改为无 Authorization |

## 安全提醒（必读）

- 默认仍绑定 **`127.0.0.1`**。同机进程可调用全部管理与 Chat 接口。
- **请勿**轻易将监听改为 `0.0.0.0` 或局域网地址，否则等于无鉴权对外开放。
- 上游供应商 API Key 仍保存在本机渠道配置中，请照常妥善保管。

## 升级建议

- 自 v0.0.3 起可用设置页「检查更新」。
- 旧版客户端配置中的 `api_key` / `Authorization` 可删除；`model` 仍填**分组名**。
- 若曾依赖 API 密钥页管理 Key：该功能已移除，不影响渠道/分组数据。

## 安装包内容

- Windows NSIS + Updater `.sig` + `latest.json` + `SHA256SUMS.txt`
- 内嵌 `sidecar/model-hub-gateway.exe`

## 已知限制

- 安装包未代码签名，可能触发 SmartScreen。
- 登录接口 `/api/v1/user/login` 仍可能存在于网关（兼容路径），但 UI 与默认调用链不再依赖。
