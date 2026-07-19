# 执行计划：应用内更新

## 清单

1. [x] 生成生产 updater 密钥对；仅提交公钥，将私钥写入 GitHub Actions Secret（密码 Secret 可选）
2. [x] 安装并钉扎 `@tauri-apps/plugin-updater`、`@tauri-apps/plugin-process` 与对应 Rust crates
3. [x] 注册 Rust 插件；配置 capabilities 最小权限
4. [x] 配置 updater 公钥、固定 GitHub `latest.json` endpoint、Windows `basicUi` 和 updater artifacts
5. [x] 新增 `src/api/updater.ts`：检查、下载进度、安装、重启与错误归一化
6. [x] 设置页新增更新面板：当前版本、手动检查、更新说明、进度、确认安装/重启、错误态
7. [x] 保证网关运行时更新确认，重启沿用现有退出停侧车生命周期
8. [x] 改造 `.github/workflows/release-windows.yml`：Secret 门禁、签名、`.sig`、`latest.json`、Release assets
9. [x] 增加 `docs/in-app-updater.md`：密钥生成、Secrets、首个基线版本、发布、轮换、失败处理
10. [x] 更新 README/release 文档/spec；确保 v0.0.1/v0.0.2 手动基线升级说明
11. [x] lint/build/fmt/test/check；本地配置与编译验证通过
12. [ ] 发布一个首个带 Updater 的版本；再用下一测试版本端到端验证应用内升级
13. [ ] 勾选 AC、spec 更新、提交归档

## 验证命令

```powershell
pnpm lint
pnpm build
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo test --manifest-path src-tauri/Cargo.toml
cargo check --manifest-path src-tauri/Cargo.toml
pnpm tauri build --bundles nsis -c src-tauri/tauri.release.conf.json
```

CI/Release 验证：

```text
NSIS setup.exe
NSIS setup.exe.sig
latest.json
SHA256SUMS.txt
AGPL/NOTICE/SOURCE
```

## 端到端门

1. 手动安装首个带 Updater 的基线版本。
2. 发布更高 patch 版本。
3. 设置页检查到新版本，显示 notes。
4. 取消安装：当前版本不变。
5. 再次检查并确认：下载进度可见，签名通过，安装完成。
6. 重启后版本更新；端口、shell.json、SQLite 和 API Key 数据保留。
7. 用错误签名 staging manifest 验证客户端拒绝安装。

## 回滚

- GitHub Release 出错：删除/撤回对应 Release，修正 manifest 后重新发布新版本；不覆盖已发布 tag 资产。
- 客户端异常：保留 GitHub 手动下载安装入口。
- 私钥泄露：停止发布，按文档走公钥轮换与桥接版本；不能只替换公钥。
