# 实施计划

## 版本与材料

1. 将版本从 `0.0.3` 升到 `0.0.4`：
   - `package.json`
   - `src-tauri/Cargo.toml`
   - `src-tauri/tauri.conf.json`
   - `src-tauri/tauri.release.conf.json`
   - `src-tauri/Cargo.lock` 中 `model-hub` 包版本（改完 Cargo.toml 后用 `cargo check` 或 `cargo update -p model-hub` 让 lock 同步，不改其他依赖）。
2. 新增 `changelog/v0.0.4.md`，覆盖 `v0.0.3..HEAD` 用户可见变化（最近成功模型展示、首页更名与设置拆分、桌面悬浮状态条），沿用安装与更新说明段落。
3. 更新 `README.md`：当前版本 `0.0.4`、更新日志链接指向 `changelog/v0.0.4.md`、示例 tag 改为 `v0.0.4`。

## 质量门禁（全部需通过）

```powershell
pnpm lint
pnpm typecheck
pnpm test:unit
pnpm build
cd src-tauri; cargo fmt --check; cargo check; cargo test
```

## 提交与发布

4. 提交发布材料：`chore(release): v0.0.4`。
5. `git push origin master`。
6. 创建并推送 tag：`git tag v0.0.4 && git push origin v0.0.4`。
7. 观察 `release-windows` 工作流：`gh run watch` 或 `gh run list`。
8. 校验 Release 与资产：
   - Release 标题 `Model Hub v0.0.4`，非 draft/prerelease。
   - 资产含 NSIS `.exe`、`latest.json`、`.sig`、`SHA256SUMS.txt`。
   - `latest.json.version == 0.0.4`，`windows-x86_64.url` 指向同 Release 资产，signature 非空。

## 回滚点

- 步骤 4 前：直接改文件。
- 步骤 5 前：`git reset` 撤销本地发布提交。
- 步骤 6 前：`git tag -d v0.0.4` 撤销本地 tag。
- 步骤 6 后：不覆盖同 tag 资产；错误则删 Release/tag 后改发更高版本。

## 阻塞处理

- `gh` 未登录或网络不可用时，完成到本地提交/推送边界，向用户报告剩余手动发布步骤，不虚构工作流结果。
