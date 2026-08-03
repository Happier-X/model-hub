# 发布 v0.0.9 执行计划

## 顺序清单

1. **写 changelog**
   - 新建 `changelog/v0.0.9.md`，记录 3 个功能/修复变更 + 安装与更新说明（参考 v0.0.8 格式）。

2. **同步版本号** `0.0.8` → `0.0.9`（5 处）
   - `package.json` → `"version": "0.0.9"`
   - `src-tauri/Cargo.toml` → `version = "0.0.9"`
   - `src-tauri/Cargo.lock` → `model-hub` 包 `version = "0.0.9"`
   - `src-tauri/tauri.conf.json` → `"version": "0.0.9"`
   - `src-tauri/tauri.release.conf.json` → `"version": "0.0.9"`

3. **验证构建**
   - `pnpm build`（前端）通过。
   - `cargo check`（src-tauri，可选，若环境可用）。

4. **提交**
   - `git commit` 消息：`chore(release): v0.0.9`
   - 仅纳入上述版本文件 + changelog。

5. **打 tag 并推送**
   - `git tag v0.0.9`
   - `git push origin v0.0.9`
   - 确认 `.github/workflows/release-windows.yml` 被触发（推 v* tag）。
   - 注意：GitHub TLS 曾连接失败，若 push 失败需重试/检查网络。

6. **验证 Remote Release**
   - 检查 GitHub Actions 是否触发、Release 资产是否生成（可选，由 CI 完成）。

## 风险与回滚

- 远端 push 因网络失败：重试；不改代码。
- push 后 CI 失败：查看 workflow 日志定位；必要时用 `chore(release)` 前状态回滚版本文件（不在本次发布内做 tag 删除除非用户明确要求）。