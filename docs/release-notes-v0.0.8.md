# Model Hub v0.0.8 发布说明

## 摘要

修复桌面 UI 无法访问本机网关的问题，并支持创建/编辑渠道时拉取上游模型列表。安装包内嵌**本地开放模式**网关（无需管理 Token / 客户端 API Key）。

## 主要变更

| 项 | 说明 |
|----|------|
| 本机 HTTP | UI 通过 **Tauri HTTP 插件**访问网关，修复 WebView `Failed to fetch` |
| HTTP 权限 | 修正本机 URL scope 非法 pattern（`[::1]` 已移除） |
| 网关 CORS | 额外放行浏览器/WebView 跨源访问本机网关 |
| 渠道模型 | 新建/编辑渠道可「拉取模型列表」（代理上游 `GET {base}/models`） |
| 鉴权 | 继续本地开放：管理 API 与 `/v1/*` 无需 Token（默认 `127.0.0.1`） |

## 升级建议

1. 安装本版后**完全退出**应用（托盘退出），再打开，确保 `bin\model-hub-gateway.exe` 被新哈希覆盖。
2. 若仍见「缺少管理 Token」：结束旧 `model-hub-gateway` 进程后重启应用。
3. 若 8080 被其他程序占用：在设置中更换端口并保存（自动重启）。
4. 创建渠道：填 Base URL + 上游 Key → **拉取模型列表** → 选择/确认模型 → 创建。

## 安全提醒

- 默认仅绑定 `127.0.0.1`。勿轻易改为局域网地址。
- 上游供应商 API Key 仍保存在本机渠道配置中。

## 安装包内容

- Windows NSIS + Updater `.sig` + `latest.json` + `SHA256SUMS.txt`
- 内嵌 `sidecar/model-hub-gateway.exe`
