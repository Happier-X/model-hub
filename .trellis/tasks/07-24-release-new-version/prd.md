# 发布新版本

## 目标

将 `master` 当前已验证内容发布为 Windows 稳定版本 `v0.0.4`，让用户通过 GitHub Release 获取 NSIS 安装包，并能通过应用内 updater 安全升级。

## 背景与已确认事实

- 当前版本为 `0.0.3`，版本来源分布在：
  - `package.json`
  - `src-tauri/Cargo.toml`
  - `src-tauri/tauri.conf.json`
  - `src-tauri/tauri.release.conf.json`
  - `src-tauri/Cargo.lock` 中 `model-hub` 包版本（由 Cargo 更新）
- 最新标签为 `v0.0.3`。
- 当前 `master` 与 `origin/master` 同步，发布规划开始前工作区仅包含本 Trellis 任务目录。
- `.github/workflows/release-windows.yml` 在推送 `v*` tag 时构建 Windows NSIS、生成 Tauri updater 签名资产和 `latest.json`、创建 GitHub Release、生成 `SHA256SUMS.txt` 并校验发布资产。
- 发布正文读取 `changelog/v<version>.md`。
- 从 `v0.0.3` 到当前 HEAD 的用户可见变化包括：
  - 首页展示最近一次成功请求的模型信息。
  - “概览”更名为“首页”，首页配置能力拆分到设置页。
  - 新增默认关闭、可拖动和记忆位置的 Windows 桌面悬浮状态条，展示最近成功上游模型及代理状态。
- 上述变化是向后兼容的功能增强和界面调整，不涉及数据库 schema 破坏性变化。

## 范围

1. 目标 SemVer 固定为 `0.0.4`，发布 tag 固定为 `v0.0.4`。
2. 同步所有应用版本来源，不允许同一发布出现版本漂移。
3. 新增对应版本的中文 changelog，覆盖用户可见功能、行为变化、安装与更新信息。
4. 运行发布前自动化门禁：
   - `pnpm lint`
   - `pnpm typecheck`
   - `pnpm test:unit`
   - `pnpm build`
   - `cd src-tauri && cargo fmt --check`
   - `cd src-tauri && cargo check`
   - `cd src-tauri && cargo test`
5. 提交版本与 changelog 变更。
6. 创建并推送版本 tag，触发 GitHub Actions Windows 发布。
7. 检查工作流和 GitHub Release：Release 非草稿/非预发布，包含 NSIS、`latest.json`、`.sig`、`SHA256SUMS.txt`，且 updater manifest 版本与 tag 一致。

## 验收标准

- AC1：所有版本源均为 `0.0.4`，`cargo metadata` 与 Tauri 配置读取到相同版本。
- AC2：`changelog/v<version>.md` 存在，内容只描述 `v0.0.3..HEAD` 的用户可见变化，不泄露敏感信息。
- AC3：全部发布前门禁通过。
- AC4：版本提交已推送至 `origin/master`，目标 tag 指向该提交并已推送。
- AC5：`release-windows` 工作流成功。
- AC6：GitHub Release 标题为 `Model Hub v<version>`，非 draft、非 prerelease，并包含可下载安装和应用内更新所需全部资产。
- AC7：Release 中 `latest.json.version == <version>`，`windows-x86_64.url` 指向同 Release 内资产，且 signature 非空。

## 范围外

- 不修改 updater 公钥或签名私钥。
- 不改变发布工作流、安装模式、GitHub Release 仓库或更新端点。
- 不发布 macOS/Linux 构建。
- 不在本任务中新增产品功能或修复与发布阻断无关的问题。

## 风险与回滚

- 推送 tag 会触发公开发布，属于不可无痕撤销的外部操作；必须先完成版本提交和全部门禁。
- 若 tag 推送前失败：修复后重新验证，不创建 tag。
- 若工作流在 Release 创建前失败：修复后重跑同一 tag 的工作流。
- 若已创建错误 Release：停止应用内推广，删除错误 Release/tag 后修正版本重新发布；不得复用已有用户可能下载的错误资产。

## 关键决策

- 目标版本为 `0.0.4`：当前变化均向后兼容，沿用现有 `0.0.x` 发布节奏。
- 发布类型为稳定版：GitHub Release 必须保持 `draft=false`、`prerelease=false`。
