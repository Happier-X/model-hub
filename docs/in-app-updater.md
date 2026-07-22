# 应用更新与发布说明

## 当前能力

项目已配置 Tauri Updater 的签名、公钥与 Windows 发布资产生成。应用本体为 **Vue 3 + 进程内 Rust 代理**，发布包**不含**侧车 `model-hub-gateway.exe`。

### 管理台检查更新

概览页提供 **「检查更新」** 入口，以及可选的 **「进入概览时自动检查更新」**（写入 `config/shell.json` 的 `check_update_on_startup`，**默认 false**）。

流程如下：

1. 调用 `@tauri-apps/plugin-updater` 的 `check()`，对照 `latest.json`。
2. **无更新**：手动检查时提示当前已是最新；启动自动检查时保持安静（不打扰）。
3. **有更新**：展示新版本号与可选发布说明（body）；**须用户点击确认**后才执行 `downloadAndInstall`（从不静默安装）。
4. 下载过程可展示进度（Started / Progress / Finished）；安装成功后调用 `@tauri-apps/plugin-process` 的 `relaunch()` 重启应用。
5. 失败时：手动检查展示中文错误并可重试；启动自动检查失败仅短提示，不阻断使用。
6. 浏览器开发态（`pnpm dev`，无 Tauri 壳）会提示「请在桌面应用内检查更新」。
7. 修改监听端口并保存时，会保留 `check_update_on_startup` 字段，不会被擦除。

权限沿用 default capability 中的 `updater:default` 与 `process:allow-restart`，无需额外配置。

用户仍可通过 GitHub Release 页面手动下载完整安装包。

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
2. 新增对应的 `docs/release-notes-vX.Y.Z.md`（例如 [v0.1.0](./release-notes-v0.1.0.md)）。
3. 完成质量检查并推送代码。
4. （可选）本机验证安装包构建：

```powershell
pnpm release:windows
```

该命令与 CI 一致：`tauri build --bundles nsis -c src-tauri/tauri.release.conf.json`。正式上传仍以 tag 触发的 Actions 为准。
5. 推送版本标签，例如：

```bash
git tag v0.1.0
git push origin v0.1.0
```

6. Windows 工作流只构建 `src-tauri` 中的 Tauri 应用（进程内代理，无需 `prepare:gateway-rust`），并发布：
   - NSIS 安装包；
   - NSIS 签名；
   - `latest.json`；
   - `SHA256SUMS.txt`；
   - 对应版本发布说明。
7. 在 Release 页面确认 `latest.json` 的版本、URL、签名和资产名称一致。

## 失败处理

- 签名或清单验证失败时不得发布。
- 已发布标签的资产不得原地覆盖；修复后发布更高版本。
- 用户数据保留在应用数据目录，安装更新不执行旧数据库迁移。

本文不构成安全或法律意见。
