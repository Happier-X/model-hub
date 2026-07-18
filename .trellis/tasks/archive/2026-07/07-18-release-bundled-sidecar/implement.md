# 执行计划：v0.0.1 + GitHub Actions

## 清单

1. [x] 固化 octopus zip/exe SHA-256；重写/增强 `scripts/prepare-bundled-octopus.ps1`
2. [x] `third-party/octopus/` 合规文件
3. [x] 版本号统一 **0.0.1**（package.json / Cargo.toml / tauri.conf）
4. [x] `tauri.release.conf.json` + `pnpm release:windows`
5. [x] Rust：`bundled` 部署 + 解析优先级；单测
6. [x] 设置页/README/gateway 文案
7. [x] `.github/workflows/release-windows.yml` + `docs/release-notes-v0.0.1.md`
8. [x] 本地：lint/build/fmt/test/check/smoke；本地 `release:windows` 已产出 NSIS
9. [x] 创建后续任务 `07-18-rust-native-gateway` 父任务规划（未 start 实现）
10. [x] 勾选工程可验证 AC；spec 已更新；按委派要求不执行 git commit

## 验证

```powershell
powershell -ExecutionPolicy Bypass -File scripts/prepare-bundled-octopus.ps1
pnpm lint
pnpm build
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo test --manifest-path src-tauri/Cargo.toml
cargo check --manifest-path src-tauri/Cargo.toml
python scripts/e2e-octopus-smoke.py
pnpm release:windows   # 若本机 NSIS 工具链可用
```

推送 tag（用户操作，需远程权限）：

```bash
git tag v0.0.1
git push origin v0.0.1
```

## 审查门

- 无 env、无手工 exe 时安装态可启动内置网关
- CI 仅 Windows 发布；不提交大二进制
- Release 含 SHA-256 与 AGPL 说明
- 版本号 0.0.1 一致

## 回滚

- 去掉 workflow / resources；恢复仅外部侧车解析
