# 实施计划

1. 将五处版本升为 `0.0.3`（含 `Cargo.lock` 中 model-hub）。
2. 新增 `changelog/v0.0.3.md`。
3. 视需要修正 README 中过时的「当前版本」文案。
4. 运行：`pnpm test:unit`、`pnpm typecheck`、`pnpm lint`；运行相关 Cargo 测试。
5. 提交发版变更。
6. 推送 `master`，创建并推送 `v0.0.3`。
7. 检查 GitHub Actions 与 Release 资产。

## 校验命令

```powershell
pnpm test:unit
pnpm typecheck
pnpm lint
# 相关 Rust 测试按仓库既有范围执行（settings / updater 相关等）
```

## 回滚点

- 推送 tag 前：可改提交或撤销本地 tag。
- 推送 tag 后：禁止覆盖同 tag；用更高版本修复。
