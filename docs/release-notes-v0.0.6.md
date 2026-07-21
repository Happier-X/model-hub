# Model Hub v0.0.6 发布说明

## 摘要

改进网关生命周期体验：打开应用默认启动网关；设置中修改监听端口并保存后自动重启。顺带缓解升级时内嵌网关文件被占用导致的「拒绝访问」部署失败。

## 主要变更

| 项 | 说明 |
|----|------|
| 自动启动 | 打开应用后默认尝试启动 `model-hub-gateway`（失败时状态条可见错误） |
| 改端口自动重启 | 保存端口 → 写 `shell.json` → stop → 按新端口 start |
| 部署被锁 | 目标 `bin\model-hub-gateway.exe` 被占用时，可回退旁路文件名部署并启动 |
| 设置页文案 | 运行中可改端口；仅启动/停止过渡中禁用 |

## 升级建议

- 自 v0.0.3 起可用设置页「检查更新」。
- 客户端 Key 前缀仍为 **`sk-modelhub-...`**（v0.0.5 起）；若仍使用 `sk-octopus-` 请重建密钥。

## 安装包内容

- Windows NSIS + Updater `.sig` + `latest.json` + `SHA256SUMS.txt`
- 内嵌 `sidecar/model-hub-gateway.exe`

## 已知限制

- 安装包未代码签名，可能触发 SmartScreen。
- 不自动结束非本应用托管的占用端口进程。
