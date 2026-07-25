# 发布 v0.0.5

## Goal

将 Model Hub 从 0.0.4 发布到 0.0.5：同步版本号、撰写更新日志、更新 README，通过质量门禁后提交并推送 tag，触发 `release-windows` 工作流产出签名安装包与 updater 资产，校验 Release 完整性。

## Scope（v0.0.4 以来的用户可见变化）

### 新增 / 变更

- 主窗口改无边框 + 自定义标题栏（拖动 / 最小化 / 最大化还原 / 关闭；关闭仍藏托盘，代理继续运行）。（`c7b76c8`）
- 升级 happier-ui 0.0.2，各页面分区卡片与侧栏改用库组件（HCard / HSidebar）。（`c66feb3`）
- 请求日志仅保留最近 7 天内的最新 1000 条，控制本地体积。（`c3814a1`）

### 修复

- 故障转移候选全部耗尽时，2xx 错误信封升级为 502，避免向客户端返回误导性的成功状态码。（`dfe89be`）
- 修复 Windows 首次建窗死锁：`set_overlay_enabled` 改为 async。（`9cc0803`）

## Requirements

1. 版本号从 `0.0.4` 升到 `0.0.5`，覆盖：`package.json`、`src-tauri/Cargo.toml`、`src-tauri/tauri.conf.json`、`src-tauri/tauri.release.conf.json`、`src-tauri/Cargo.lock`（仅 `model-hub` 包）。
2. 新增 `changelog/v0.0.5.md`，沿用既有格式（新增 / 变更 / 修复 / 安装与更新）。
3. 更新 `README.md`：当前版本 `0.0.5`、更新日志链接指向 `changelog/v0.0.5.md`、示例 tag 改 `v0.0.5`。
4. 全量质量门禁通过。
5. 提交、推送、打 tag，触发并校验发布工作流。

## Non-Goals

- 不改任何功能代码（纯发布材料）。
- 不覆盖已存在的 tag / Release。

## Acceptance Criteria

- [ ] 五处版本号均为 `0.0.5`。
- [ ] `changelog/v0.0.5.md` 存在且内容与本次范围一致。
- [ ] `README.md` 版本、链接、示例 tag 均指向 0.0.5。
- [ ] `pnpm lint`、`pnpm typecheck`、`pnpm test:unit`、`pnpm build` 通过。
- [ ] `cargo fmt --check`、`cargo check`、`cargo test` 通过。
- [ ] 发布提交 `chore(release): v0.0.5` 已推送 master。
- [ ] tag `v0.0.5` 已推送，`release-windows` 工作流成功。
- [ ] Release 标题 `Model Hub v0.0.5`，非 draft/prerelease；资产含 NSIS `.exe`、`latest.json`、`.sig`、`SHA256SUMS.txt`；`latest.json.version == 0.0.5` 且 windows-x86_64 URL/签名有效。

## Notes

- tag 推送与 CI 触发为不可逆步骤，执行前需再次向用户确认。
- `gh` 未登录或网络不可用时，完成到本地提交/推送边界并报告剩余手动步骤，不虚构工作流结果。
