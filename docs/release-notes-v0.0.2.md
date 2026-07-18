# Model Hub v0.0.2 发布说明

## 本次更新

- 设置页新增“网关监听端口”，支持 `1–65535`。
- 默认仍监听 `127.0.0.1:8080`。
- 端口配置持久化保存，重启应用后仍然有效。
- 网关运行时禁止修改端口：请先停止，保存后再手动启动。
- 修复端口占用提示要求修改设置、但设置页没有入口的问题。
- 应用不会自动结束占用端口的其他进程。

## 安装与升级

1. 下载 Release 附件中的 NSIS 安装包（`.exe`）。
2. 使用 `SHA256SUMS.txt` 校验安装包。
3. 退出旧版 Model Hub 后运行安装程序升级。
4. 若默认 8080 被占用，进入“设置”→“网关监听端口”，停止网关后保存其他空闲端口，再手动启动。

安装包继续内嵌钉扎 **octopus v0.9.28**，无需自行下载或放置 `octopus.exe`。

### Windows SmartScreen / 未知发布者

本版本未做代码签名。Windows 可能显示“未知发布者”或 SmartScreen 提示。请仅从本仓库 GitHub Releases 下载并核对 SHA-256。

## 鉴权说明

| 用途 | 凭证 | 路径 |
|------|------|------|
| 管理 UI / 管理 API | 管理 JWT | `/api/v1/*` |
| 外部客户端 | 网关 API Key（`sk-octopus-...`） | `/v1/*` |

管理 JWT 不能代替客户端网关 API Key。请在“API 密钥”页创建完整的 `sk-octopus-...` Key。

## 内置网关与 AGPL

- 内嵌组件：[bestruirui/octopus](https://github.com/bestruirui/octopus) v0.9.28（AGPL-3.0）。
- 对应 commit：`b7b053e7fd81911e2062359e93f9dcbd58114bb0`。
- 对应源码：https://github.com/bestruirui/octopus/archive/refs/tags/v0.9.28.tar.gz
- 许可证与说明见 Release 附件及仓库 `third-party/octopus/`。

本说明不构成法律意见。

## 已知限制

- 仅支持 Windows。
- 暂无应用内自动更新。
- 安装包尚未代码签名。
- 当前仍使用内嵌网关侧车；Rust 原生网关处于后续规划阶段。
