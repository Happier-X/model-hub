# 发布 v0.1.0 执行计划

> 参考 v0.0.9 发布任务（`.trellis/tasks/archive/2026-08/08-03-release-v0-0-9/`）的已验证流程。

## 顺序清单

1. **写 changelog**
   - 新建 `changelog/v0.1.0.md`，记录 3 个变更（TDZ 修复 / octopus 分组交互 / llm_benchmark 榜单）+ 安装与更新说明（参考 v0.0.9 格式）。

2. **同步版本号** `0.0.9` → `0.1.0`（5 处）
   - `package.json` → `"version": "0.1.0"`
   - `src-tauri/Cargo.toml` → `version = "0.1.0"`
   - `src-tauri/Cargo.lock` → `model-hub` 包 `version = "0.1.0"`
   - `src-tauri/tauri.conf.json` → `"version": "0.1.0"`
   - `src-tauri/tauri.release.conf.json` → `"version": "0.1.0"`

3. **验证构建**
   - `pnpm build`（vue-tsc + vite build）通过。
   - `cd src-tauri && cargo check`（可选，环境可用时跑；验证 Rust 侧不因版本号破坏）。

4. **提交**
   - `git commit` 消息：`chore(release): v0.1.0`
   - 仅纳入 changelog + 5 个版本文件（不含功能代码、.trellis 任务文件）。

5. **打 tag 并推送**
   - `git tag v0.1.0`
   - `git push origin v0.1.0`
   - 确认 `.github/workflows/release-windows.yml` 被触发（workflow 触发条件应含 v* tag）。
   - 注意：GitHub TLS 曾连接失败，若 push 失败需重试/检查网络。

6. **验证 Remote Release**
   - `gh run list` 查看 release-windows 运行状态。
   - Release 完成后 `gh release view v0.1.0` 确认资产（NSIS exe + latest.json + SHA256SUMS.txt）。

## 验证命令

```bash
pnpm build
cd src-tauri && cargo check 2>&1 | tail -3   # 可选
gh run list --workflow=release-windows.yml --limit 3
gh release view v0.1.0 --json assets --jq '.assets[].name'
```

## 风险与回滚

- 远端 push 因网络失败：重试；不改代码。
- push 后 CI 失败：查看 workflow 日志定位；必要时在发布前状态回滚版本文件。
- tag 冲突（v0.1.0 已存在）：先 `git ls-remote --tags origin v0.1.0` 确认，异常时停下询问。
