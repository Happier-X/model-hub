# 实施计划

1. 版本号四文件改为 `0.0.1`；检查 `Cargo.lock` 中 package 版本是否需同步。
2. 新建 `changelog/v0.0.1.md`（当前能力首发说明）。
3. 删除全部 `docs/release-notes-v*.md`。
4. 更新 `README.md`、`docs/in-app-updater.md` 发版路径与示例版本。
5. 修改 `.github/workflows/release-windows.yml`：Release body 读 `changelog/v{version}.md`，移除旧 release-notes 附件逻辑。
6. 提交代码与文档变更。
7. 删除 GitHub Releases + 远端/本地历史 tags。
8. 推送 `master`，打并推送新 `v0.0.1`。
9. 用 `gh` 确认工作流已触发；记录结果。

## 验证

- 四文件 version 均为 `0.0.1`。
- `changelog/v0.0.1.md` 存在且无旧版本相对叙事。
- `rg release-notes-v docs README .github` 无残留路径（除归档任务外）。
- `git tag -l` / `git ls-remote --tags` 仅剩新 `v0.0.1`（推送后）。
- Actions `release-windows` 在 `v0.0.1` 上运行。
