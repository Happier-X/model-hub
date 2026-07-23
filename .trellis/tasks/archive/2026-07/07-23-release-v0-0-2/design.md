# 技术设计

## 版本同步

同步修改：
- `package.json`
- `src-tauri/Cargo.toml`
- `src-tauri/Cargo.lock` 中 model-hub package
- `src-tauri/tauri.conf.json`
- `src-tauri/tauri.release.conf.json`

目标版本统一为 `0.0.2`。

## 更新日志

新增 `changelog/v0.0.2.md`：
- 修复「配置到 Pi」Tauri 2 命令参数 camelCase 错误；
- 同步修复模型榜单强制刷新参数；
- 新安装默认代理端口由 8080 改为 8888；
- 已有 shell.json 端口配置不覆盖。

## 发布流程

1. 质量检查。
2. 提交版本/changelog。
3. 推送 master（包括此前尚未推送的修复与任务归档提交）。
4. tag `v0.0.2` 并推送。
5. 等待 release-windows，确认资产与 latest.json。
