# 应用内更新说明

## 当前策略

Model Hub 仅在“设置”页手动检查正式 GitHub Release，不会在启动时联网，也不跟踪预发布版本。发现更新后，用户确认才会下载、安装和重启。

更新源：

```text
https://github.com/Happier-X/model-hub/releases/latest/download/latest.json
```

更新包必须通过 Tauri Updater 签名校验；签名失败不会安装，当前版本继续运行。

## 首个基线版本

v0.0.1 和 v0.0.2 没有 Updater 公钥和更新配置，不能应用内升级。用户需要手动安装首个带 Updater 的 Windows 版本，之后才能通过应用内更新后续版本。

应用内更新不会覆盖 `%APPDATA%/Model Hub` 下的端口配置 `config/shell.json`、网关 SQLite、日志或内嵌侧车部署目录。

## 生成密钥

在可信开发机生成一次密钥对：

```powershell
pnpm tauri signer generate -w "$HOME\.tauri\model-hub-updater.key" -p "请使用密码管理器生成的密码"
```

- `.key` 是私钥，绝对不能提交 Git、写入 Issue、日志或 Release 附件。
- `.key.pub` 的内容是公钥，写入 `src-tauri/tauri.conf.json` 和发布配置。
- 私钥丢失或密码丢失后，已有客户端无法验证新密钥签名；轮换必须先发布桥接版本，不能直接替换公钥。

## GitHub Actions Secrets

在仓库 Settings → Secrets and variables → Actions 中配置：

```text
TAURI_SIGNING_PRIVATE_KEY
TAURI_SIGNING_PRIVATE_KEY_PASSWORD（如果密钥设置了密码）
```

Workflow 在构建前检查 `TAURI_SIGNING_PRIVATE_KEY`。未配置时直接失败，不发布没有签名的更新资产。

## 发布步骤

1. 修改 `package.json`、`Cargo.toml`、Tauri 配置的同一版本号。
2. 更新对应 `docs/release-notes-vX.Y.Z.md`。
3. 提交并推送主分支。
4. 推送匹配版本的 tag，例如：

```bash
git tag v0.0.3
git push origin master
 git push origin v0.0.3
```

5. Windows Actions 生成并发布：
   - NSIS 安装包
   - NSIS `.sig`
   - `latest.json`
   - `SHA256SUMS.txt`
   - AGPL/NOTICE/SOURCE 材料
6. 在 Release 页面确认 `latest.json` 中的版本、下载 URL、签名与公开资产名称一致。

## 失败处理

- 检查失败：设置页显示网络错误，当前版本不变。
- 下载/签名校验失败：拒绝安装，用户可从 GitHub Release 手动下载。
- 用户取消确认：不下载或不重启。
- 安装重启前若网关运行，应用退出流程会停止托管侧车；用户数据由 app data 保留。
- 不覆盖已发布 tag 的资产；修复后发布更高版本。

本文不构成安全或法律意见。
