# Model Hub v0.0.4 发布说明

## 亮点

- **默认网关改为 Rust 原生**（`model-hub-gateway`），安装包**不再内嵌** octopus 二进制。
- 发布体积与合规叙事简化：Release **不再**附带 AGPL 侧车附件（仓库内 `third-party/octopus/` 仅作历史/可选回退参考）。
- 安装后用户仍**无需**手工放置网关 exe：内嵌 `model-hub-gateway.exe`，首次启动按哈希部署到应用数据目录。

## 相对 v0.0.3 的主要变化

### 网关

| 能力 | 说明 |
|------|------|
| 默认实现 | 仅 **Rust** `model-hub-gateway`（已移除 octopus 运行时兼容） |
| 鉴权 | 管理 JWT + 客户端 `sk-octopus-...`（历史前缀保留） |
| 数据 | SQLite：渠道 / 分组 / API Key / 请求日志 |
| Chat | 非流式 + SSE 流式；客户端 `model` = **分组名** |
| 迁移 | `model-hub-gateway migrate-octopus --source ... --dest ...` |

### 桌面壳

- 设置页文案同步为默认 Rust 网关。
- 端口设置、托盘隐藏/退出、应用内更新流程与 v0.0.3 相同。

## 从旧版升级注意

1. **v0.0.1 / v0.0.2**：无应用内更新能力，请手动安装本版或先装 v0.0.3 再更新。
2. **v0.0.3 → v0.0.4**：可用设置页「检查更新」（需 GitHub Release 资产与签名就绪）。
3. **数据目录**：若你曾用默认 octopus 写过 `data/data.db`，切换到 Rust 后 schema 不兼容。请：
   - 备份后使用 `migrate-octopus` 导入到新库，或
   - 清空 / 更换 gateway 数据目录后重新配置。

## 安装与安全提示

- 安装包**未做代码签名**，Windows SmartScreen /「未知发布者」属预期，请从本仓库 GitHub Releases 下载核对 SHA-256。
- 默认仅监听 `127.0.0.1`；端口可在设置页停止网关后修改。

## 开发者

```powershell
pnpm prepare:gateway-rust
pnpm tauri dev
# 可选兼容侧车：
```

## 校验（发布方）

Release 应包含：NSIS 安装包、`.sig`、`latest.json`、`SHA256SUMS.txt`、本发布说明。

本文不构成法律意见。
