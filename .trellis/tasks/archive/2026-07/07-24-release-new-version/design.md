# 技术设计

## 版本同步

目标版本固定为 `0.0.4`，tag 固定为 `v0.0.4`。

同步范围：

- `package.json` 的应用版本。
- `src-tauri/Cargo.toml` 的 crate/app 版本。
- `src-tauri/Cargo.lock` 中 `model-hub` 包版本（由 Cargo 重新生成或精确同步）。
- `src-tauri/tauri.conf.json` 的 Tauri 应用版本。
- `src-tauri/tauri.release.conf.json` 的发布配置版本。
- `README.md` 中当前版本说明、更新日志链接和示例 tag。

禁止修改依赖版本、updater 公钥、签名 Secret、发布 workflow 或安装模式。

## 更新日志

新增 `changelog/v0.0.4.md`，只写 `v0.0.3..HEAD` 的用户可见变化，不写 task/archive/spec/journal 这类内部提交。

建议结构：

- `## 新增`
  - 首页展示最近一次成功请求的模型信息。
  - 桌面悬浮状态条：默认关闭，可在设置页开启，显示最近成功上游模型与代理状态，支持拖动并记住位置。
- `## 变更`
  - “概览”更名为“首页”。
  - 首页配置能力拆分到设置页，设置入口更集中。
- `## 安装与更新`
  - NSIS 安装包、SHA256、应用内更新清单说明，沿用 `v0.0.3` 文案。

## 发布流程

1. 发布前只做版本/changelog/README 等发布材料变更，不增加产品代码。
2. 运行完整质量门禁。
3. 提交发布材料：`chore(release): v0.0.4`。
4. 推送 `master` 到 `origin`。
5. 创建 annotated 或 lightweight tag `v0.0.4` 指向发布提交并推送。为保持现有历史可用 lightweight tag，除非仓库已有 annotated 要求。
6. GitHub Actions `release-windows` 自动创建 Release 和 updater 资产。
7. 使用 `gh` 或 GitHub API 检查工作流结论、Release 状态、资产清单和 `latest.json` 内容。

## 风险与回滚

- 推送 tag 前：可修改发布提交、重跑检查、删除本地 tag。
- 推送 tag 后且工作流未创建可用 Release：优先修复后重跑 workflow；若 tag/Release 内容错误且可能影响用户，删除错误 Release/tag 并改发更高版本，不复用已有用户可能下载的错误资产。
- 签名 Secret 缺失由 CI 报错；本地不得写入或提交私钥。
- 如果 GitHub CLI 未登录或网络不可用，停止在“本地发布材料已提交”状态，向用户明确剩余手动步骤。
