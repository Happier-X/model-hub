# Model Hub v0.0.3 发布说明

## 本次更新

- 设置页新增应用内更新：手动检查正式 GitHub Release。
- 更新包通过 Tauri 签名校验后，用户确认才下载、安装和重启。
- 下载过程中显示进度；取消或失败不会影响当前版本。
- Windows Release 增加 Updater `latest.json`、`.sig` 和 SHA-256。
- **默认网关切换为 gateway-rust**（`model-hub-gateway`）；发布包**不再内嵌** octopus 二进制与 AGPL 合规附件。
- 保留网关端口设置、用户配置；从旧版 octopus 升级请 `migrate-octopus` 或新建 SQLite，勿混用旧库。

## 首次使用更新功能

v0.0.1/v0.0.2 没有 Updater 配置，不能直接应用内升级。请先手动安装本版本作为更新基线；之后后续正式 Release 可在“设置”页检查更新。

更新不会在应用启动时联网。进入“设置”页，点击“检查更新”，发现新版本后确认下载和安装。网关运行时，重启更新会安全停止托管网关。

## 签名与安全

更新包必须通过 Tauri 签名校验；签名失败时不会安装。Windows 安装包未做代码签名，SmartScreen 可能提示未知发布者，请从 GitHub Releases 下载并核对 `SHA256SUMS.txt`。

## 鉴权与网关

- 管理 UI 使用管理 JWT。
- 外部客户端 `/v1/*` 必须使用 API 密钥页创建的完整 `sk-octopus-...` Key（历史前缀兼容）。
- 默认监听 `127.0.0.1:8080`；端口可在设置页停止网关后修改。
- 默认实现为 rust；高级用户可设 `MODEL_HUB_GATEWAY_IMPL=octopus` 并自备二进制回退（发布包不提供）。

## 合规说明

- 默认网关为仓库内 `gateway-rust`，发布包**不再**分发 octopus 二进制。
- 可选自备的历史组件 [bestruirui/octopus](https://github.com/bestruirui/octopus) 为 AGPL-3.0；仓库 `third-party/octopus/` 仅作历史/开发回退参考。

本说明不构成法律意见。
