# 增加应用内更新

## Goal

为 Model Hub 增加安全的应用内更新能力：用户可以在设置页检查 GitHub Releases 中的新版本，查看版本和更新说明，经用户确认后下载、安装并重启应用，不再需要手动下载新的 Windows 安装包。

## 已确认事实

- 当前应用为 Tauri 2，Windows NSIS 安装包通过 GitHub Actions 和 GitHub Release 发布。
- 当前发布版本为 v0.0.2。
- 现有应用尚未接入 Tauri Updater，也没有更新签名密钥配置。
- Tauri Updater 的安装包必须签名；私钥不能提交仓库，应放在 GitHub Actions Secret 或本机安全存储中。
- 更新服务必须保持 Windows-only、GitHub Release 作为发布源，并兼容未签名的历史 v0.0.1/v0.0.2 安装包升级策略。

## Requirements

### R1. 更新检查

- 设置页提供“检查更新”入口。
- 显示当前版本、检查中、已是最新、发现新版本、检查失败等状态。
- 更新元数据至少包括版本号、发布时间/更新说明、下载大小（若源提供）。
- 更新源使用本项目 GitHub Releases 的稳定 JSON endpoint，不允许从不可信任意 URL 更新。

### R2. 用户确认安装

- 发现新版本后必须由用户明确确认下载和安装。
- 下载期间展示进度、允许取消或安全失败，不阻塞网关业务 UI。
- 安装前展示目标版本和更新说明。
- 安装完成后提示重启；重启由用户确认，不做静默强制重启。

### R3. 更新安全

- 使用 Tauri Updater 签名校验；公钥随应用发布配置，私钥只通过 CI Secret 注入。
- 签名验证失败、下载失败、网络超时、版本格式异常时拒绝安装并展示可行动错误。
- 不使用真实密钥、API Key、JWT 或 updater 私钥写入 Git。
- 更新失败不能删除当前可运行版本；依赖 Tauri/NSIS 的安装替换与回滚机制。

### R4. 发布流水线

- GitHub Actions Windows 发布工作流生成 updater 所需的签名安装包/更新产物与 manifest。
- Release 附件继续包含 NSIS 安装包、SHA-256、AGPL 合规材料，并增加 updater 产物/manifest。
- 版本号、Release tag、更新 manifest 版本保持一致。
- 文档说明生成签名密钥、配置 GitHub Secret、发布和轮换流程；不在仓库保存私钥。

### R5. 兼容与体验

- 保持现有端口设置、内嵌 octopus、托盘生命周期和网关 API 鉴权行为。
- 更新安装包不会覆盖用户 app data、shell.json、网关 SQLite 或配置。
- 更新期间如果网关正在运行，退出/重启前明确提示；不得遗留托管侧车进程。

## Acceptance Criteria

- [ ] AC1：设置页可检查 GitHub Release 更新并显示明确状态。
- [ ] AC2：发现新版本后，用户确认即可下载、安装并重启；取消不会改变当前版本。
- [ ] AC3：签名校验失败的更新不会安装，且错误可见。
- [ ] AC4：GitHub Actions 可生成与发布 updater 产物、manifest、NSIS 和校验文件。
- [ ] AC5：用户数据、端口配置、侧车部署和 API 鉴权在更新后保持可用。
- [ ] AC6：文档覆盖签名 Secret 配置、发布步骤、回滚/失败处理；不泄露私钥。

## Out of Scope

- macOS/Linux 更新。
- 静默强制更新。
- 应用内升级 octopus 侧车版本（跟随应用 Release 资源）。
- 自建更新服务器或非 GitHub Release 更新源。
- 自动迁移 Rust 原生网关。

## Decisions Log

| # | 决策 | 选择 | 日期 |
|---|------|------|------|
| D1 | 更新签名私钥 | 生产私钥仅保存到 GitHub Actions Secret；公钥随应用配置 | 2026-07-19 |
| D2 | 安装方式 | 用户明确确认后下载/安装，不静默强制更新 | 2026-07-19 |
| D3 | 检查时机 | 仅设置页手动检查，不在启动时联网 | 2026-07-19 |
| D4 | 更新通道 | 仅正式 GitHub Release，不跟踪预发布版 | 2026-07-19 |

## 兼容说明

当前未集成 Updater 的 v0.0.1/v0.0.2 无法在应用内自更新；首个带 Updater 的版本必须手动安装一次，之后才能应用内更新。
