# 应用更新与发布说明

## 当前能力

项目已配置 Tauri Updater 的签名、公钥与 Windows 发布资产生成。当前 Vue 管理台尚未提供手动检查更新入口；用户通过 GitHub Release 获取新安装包。后续接入界面时，应直接调用已注册的 Tauri Updater 插件，并保持用户确认后再下载与安装。

更新清单地址：

```text
https://github.com/Happier-X/model-hub/releases/latest/download/latest.json
```

更新资产必须通过 Tauri 签名校验。应用升级不会删除应用数据目录中的 `config/shell.json` 和 SQLite 数据库。

## 生成签名密钥

在可信开发机生成一次密钥对：

```powershell
pnpm tauri signer generate -w "$HOME\.tauri\model-hub-updater.key" -p "请使用密码管理器生成的密码"
```

- 私钥绝不能提交到 Git、Issue、日志或 Release 附件。
- 公钥写入 `src-tauri/tauri.conf.json` 与发布配置。
- 密钥轮换必须先发布能信任新公钥的过渡版本。

## GitHub Actions Secrets

在仓库 Actions Secrets 中配置：

```text
TAURI_SIGNING_PRIVATE_KEY
TAURI_SIGNING_PRIVATE_KEY_PASSWORD
```

发布工作流在构建前检查私钥，缺失时拒绝发布。

## 发布步骤

1. 同步修改 `package.json`、`src-tauri/Cargo.toml` 与 Tauri 配置中的版本号。
2. 新增对应的 `docs/release-notes-vX.Y.Z.md`。
3. 完成质量检查并推送代码。
4. 推送版本标签，例如：

```bash
git tag v0.1.0
git push origin v0.1.0
```

5. Windows 工作流只构建 `src-tauri` 中的 Tauri 应用，并发布：
   - NSIS 安装包；
   - NSIS 签名；
   - `latest.json`；
   - `SHA256SUMS.txt`；
   - 对应版本发布说明。
6. 在 Release 页面确认 `latest.json` 的版本、URL、签名和资产名称一致。

## 失败处理

- 签名或清单验证失败时不得发布。
- 已发布标签的资产不得原地覆盖；修复后发布更高版本。
- 用户数据保留在应用数据目录，安装更新不执行旧数据库迁移。

本文不构成安全或法律意见。
