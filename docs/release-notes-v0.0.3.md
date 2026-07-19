# Model Hub v0.0.3 发布说明

## 本次更新

- 设置页新增应用内更新：手动检查正式 GitHub Release。
- 更新包通过 Tauri 签名校验后，用户确认才下载、安装和重启。
- 下载过程中显示进度；取消或失败不会影响当前版本。
- Windows Release 增加 Updater `latest.json`、`.sig` 和 SHA-256。
- 保留网关端口设置、内嵌 octopus v0.9.28、用户配置和 SQLite 数据。

## 首次使用更新功能

v0.0.1/v0.0.2 没有 Updater 配置，不能直接应用内升级。请先手动安装本版本作为更新基线；之后后续正式 Release 可在“设置”页检查更新。

更新不会在应用启动时联网。进入“设置”页，点击“检查更新”，发现新版本后确认下载和安装。网关运行时，重启更新会安全停止托管网关。

## 签名与安全

更新包必须通过 Tauri 签名校验；签名失败时不会安装。Windows 安装包未做代码签名，SmartScreen 可能提示未知发布者，请从 GitHub Releases 下载并核对 `SHA256SUMS.txt`。

## 鉴权与网关

- 管理 UI 使用管理 JWT。
- 外部客户端 `/v1/*` 必须使用 API 密钥页创建的完整 `sk-octopus-...` Key。
- 默认监听 `127.0.0.1:8080`；端口可在设置页停止网关后修改。

## AGPL 合规

- 内嵌组件：[bestruirui/octopus](https://github.com/bestruirui/octopus) v0.9.28（AGPL-3.0）。
- 对应 commit：`b7b053e7fd81911e2062359e93f9dcbd58114bb0`。
- 对应源码：https://github.com/bestruirui/octopus/archive/refs/tags/v0.9.28.tar.gz
- 许可证、NOTICE、SOURCE 随仓库与 Release 附件提供。

本说明不构成法律意见。
