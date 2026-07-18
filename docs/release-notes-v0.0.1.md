# Model Hub v0.0.1 发布说明

## 简介

首个公开 Windows 安装版。安装后**无需自行下载或放置 `octopus.exe`**，应用内嵌钉扎 **octopus v0.9.28** 网关侧车，首次启动时自动部署到应用数据目录。

## 安装

1. 下载 Release 附件中的 **NSIS 安装包**（`.exe`）。
2. （推荐）用 `SHA256SUMS.txt` 校验安装包哈希。
3. 运行安装程序，按向导完成安装。
4. 启动 **Model Hub**，设置页确认网关为「内置网关 v0.9.28」并可启动。

### Windows SmartScreen / 未知发布者

本版本**未做代码签名**。Windows 可能提示「Windows 已保护你的电脑」或未知发布者：

- 可点击「更多信息」→「仍要运行」（文案因系统版本略有差异）。
- 仅从本仓库 **GitHub Releases** 下载，并核对 SHA-256。

## 鉴权说明（重要）

| 用途 | 凭证 | 路径 |
|------|------|------|
| 管理 UI / 管理 API | 管理 JWT（应用静默登录或设置页粘贴） | `/api/v1/*` |
| 外部客户端（Cursor 等） | 网关 API Key（`sk-octopus-...`） | `/v1/*` |

- **不要**把管理 JWT 当作客户端 `api_key`。
- 在应用 **API 密钥** 页创建 Key，再用于 `GET /v1/models`、`POST /v1/chat/completions`。
- 详见 [docs/client-integration.md](./client-integration.md)、[docs/chat-onboarding.md](./chat-onboarding.md)。

## 内置网关与 AGPL

- 内嵌组件：[bestruirui/octopus](https://github.com/bestruirui/octopus) **v0.9.28**（AGPL-3.0）。
- 许可证全文：安装资源 / 仓库 `third-party/octopus/LICENSE-AGPL-3.0.txt`。
- NOTICE 与对应源码链接：`third-party/octopus/NOTICE.md`、`SOURCE.md`。
- 对应源码 archive：  
  https://github.com/bestruirui/octopus/archive/refs/tags/v0.9.28.tar.gz  
  commit：`b7b053e7fd81911e2062359e93f9dcbd58114bb0`

本说明不构成法律意见。

## 开发覆盖（可选）

高级用户仍可用环境变量覆盖侧车路径：

```powershell
$env:MODEL_HUB_GATEWAY_BIN = "C:\path\to\custom\octopus.exe"
```

优先级：`MODEL_HUB_GATEWAY_BIN` → 安装包内嵌资源按哈希部署到 app data `bin/octopus.exe` → （无内嵌时）已有 `bin` 副本。

## 已知限制

- 仅 Windows；无 macOS/Linux 安装包。
- 无应用内自动更新。
- 未代码签名（见上）。
- 阶段 1 依赖 Go 侧车；Rust 原生网关为后续规划，见任务 `rust-native-gateway`。

## 校验文件

Release 附带 `SHA256SUMS.txt`，至少覆盖 NSIS 安装包。
