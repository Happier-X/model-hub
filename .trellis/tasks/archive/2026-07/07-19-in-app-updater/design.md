# 设计：应用内更新

## 1. 总体架构

采用 Tauri 2 官方 `tauri-plugin-updater` + `tauri-plugin-process`，更新源固定为 GitHub Releases 静态 manifest：

```text
设置页手动检查
  → @tauri-apps/plugin-updater.check()
  → https://github.com/Happier-X/model-hub/releases/latest/download/latest.json
  → 仅接受正式版 Windows x86_64 更新
  → 用户确认
  → update.downloadAndInstall(progress)
  → 用户确认
  → @tauri-apps/plugin-process.relaunch()
```

不在启动阶段调用检查，不连接任意用户输入 URL，不跟踪 prerelease。

## 2. Tauri 配置

在 `src-tauri/tauri.conf.json` 和发布配置中配置：

```json
{
  "bundle": {
    "createUpdaterArtifacts": true
  },
  "plugins": {
    "updater": {
      "pubkey": "<生产公钥，仅此值进入仓库>",
      "endpoints": [
        "https://github.com/Happier-X/model-hub/releases/latest/download/latest.json"
      ],
      "windows": {
        "installMode": "basicUi"
      }
    }
  }
}
```

`basicUi` 保留 Windows 安装交互；应用自身仍要求用户确认下载/安装/重启。若 Tauri 版本对 `createUpdaterArtifacts` 要求 `v1Compatible`，以本地 CLI 校验结果为准并同步 workflow。

公钥为空时不能生成可用发布版；开发构建可使用不启用 updater 的配置，但不能提交占位密钥冒充正式配置。

## 3. 前端契约

新增 `src/api/updater.ts`，封装：

```ts
checkForUpdate(): Promise<UpdateInfo | null>
installUpdate(update, onProgress): Promise<void>
relaunchApp(): Promise<void>
```

设置页新增 `UpdaterPanel`：

- 当前版本从 `@tauri-apps/api/app.getVersion()` 显示。
- “检查更新”按钮仅在空闲时可用。
- 状态：空闲、检查中、最新、发现更新、下载中、已下载待重启、成功/失败。
- 新版本展示 `version`、`date`、`body`，由用户确认 `window.confirm` 后安装。
- 下载进度按已下载字节/总长度显示；未知长度显示已下载字节。
- 安装结束再次确认是否立即重启；取消则保持应用运行并提示下次可重启。
- 失败显示不含密钥的可行动信息。

更新安装前：

```text
若 gateway.state == running：提示“安装更新将重启应用并停止托管网关，是否继续？”
```

用户取消不调用下载/安装；下载失败不调用 relaunch。

## 4. Rust 与权限

- `src-tauri/src/lib.rs` 注册 `tauri_plugin_updater::Builder::new().build()`。
- 注册 `tauri_plugin_process::init()`。
- `src-tauri/capabilities/default.json` 增加 updater 默认权限，以及 process relaunch 权限，按实际插件生成权限名称用 `cargo tauri permissions`/构建错误校准。
- 更新插件完成签名校验；前端不得绕过插件直接下载 exe。

## 5. GitHub Actions

现有 Windows workflow 增加：

1. `pnpm install --frozen-lockfile`。
2. prepare octopus。
3. 设定 `TAURI_SIGNING_PRIVATE_KEY` 与可选 `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`，值来自 GitHub Secrets。
4. Tauri build 生成 NSIS、`.sig` 和 `latest.json` 所需 artifact。
5. Release 上传：NSIS、NSIS `.sig`、`latest.json`、SHA256、AGPL 合规材料和 v0.0.x release notes。

注意：Tauri Action 官方更擅长生成 GitHub Release updater manifest；若继续使用当前 `softprops/action-gh-release`，必须在 Windows runner 上确认 `latest.json` 生成和 asset URL，再上传，不能只上传普通 NSIS。

发布前检查：

- `TAURI_SIGNING_PRIVATE_KEY` Secret 已配置；未配置时 workflow 明确失败，不发布无签名 updater。
- `latest.json` 的版本与 tag、NSIS `.sig` 和下载 URL 一致。
- 轮换公钥需要先发布兼容更新链路，禁止直接替换导致旧客户端无法验证。

## 6. 兼容与数据安全

- Updater 只替换安装目录文件，不删除 `%APPDATA%/Model Hub` 下的 `shell.json`、`gateway/data/data.db`、日志或内嵌部署副本。
- 应用退出事件继续调用 `stop_managed`，因此重启时托管侧车会被停止。
- v0.0.1/v0.0.2 未包含公钥和 updater 配置，不能应用内升级；必须手动安装首个启用 updater 的版本。
- 更新后若新版本资源侧车哈希变化，现有部署逻辑按 SHA-256 替换；用户业务数据不变。

## 7. 风险与回滚

| 风险 | 处理 |
|---|---|
| 签名 Secret 缺失 | CI 在构建前失败，不发布不安全包 |
| manifest/签名不匹配 | 客户端拒绝安装；CI 校验 asset 与 JSON |
| GitHub 网络失败 | UI 显示失败，当前版本继续运行 |
| 安装失败 | Tauri Windows installer 保留当前版本；不主动删 app data |
| 侧车运行中更新 | 安装/重启前确认，退出流程停止托管子进程 |
