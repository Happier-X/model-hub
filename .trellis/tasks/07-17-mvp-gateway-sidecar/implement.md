# 执行计划：MVP 网关侧车集成

## Checklist

1. [x] 新增 `src-tauri/src/gateway/*` 状态机、配置、spawn、health、命令
2. [x] `lib.rs` 注册命令与退出钩子；可选自动 start
3. [x] `gateway/README.md` 钉扎说明 + AGPL
4. [x] 前端 status/设置启停；`api/tauri.ts` 扩展
5. [x] 单元测试 + `pnpm build` + `cargo test/check`
6. [ ] 若本机有/能下载 octopus.exe：手动 start→status→stop 冒烟（未下载真实二进制；缺失路径与状态机已测）
7. [x] 更新 backend spec（gateway 模块与 invoke 契约）

## Validation

```bash
pnpm build
pnpm lint
cargo test --manifest-path src-tauri/Cargo.toml
cargo check --manifest-path src-tauri/Cargo.toml
# 可选：放置 octopus.exe 后 pnpm tauri dev
```

## Rollback

删除 `src-tauri/src/gateway/` 与相关 UI 绑定，恢复状态条「未集成」。
