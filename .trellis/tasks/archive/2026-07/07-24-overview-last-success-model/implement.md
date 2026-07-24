# 实施计划

1. 在 `domain/log.rs` 增加 `LastSuccessRequest` 与 `last_success_request()`，补 Rust 测试。
2. 在 `commands.rs` / `lib.rs` 暴露 `get_last_success_request`。
3. 在 `src/api/tauri.ts` 增加类型与 `getLastSuccessRequest`。
4. 修改 `OverviewPage.vue`：展示卡片、空态、与 stats 一并刷新。
5. 运行：`cargo test`（log 相关）、`pnpm test:unit`、`pnpm typecheck`、`pnpm lint`。
6. 质量检查（trellis-check）后提交。

## 校验命令

```powershell
cd src-tauri; cargo test last_success -- --nocapture
pnpm test:unit
pnpm typecheck
pnpm lint
```

## 回滚点

- 可整提交回滚；无 DB 迁移，无数据破坏风险。
