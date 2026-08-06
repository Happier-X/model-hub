# 所有依赖升级到最新版本

## Goal

前端 npm 与后端 Rust 依赖全部升级到最新 stable 版本，保证构建/测试全绿。

## Background（已确认事实）

- 前端 `package.json` 依赖：vue 3.5.18、vue-router 4.5.1、vite 7.1.5、typescript 5.9.2、eslint 9.35.0、tailwindcss 4.1.13、happier-ui 0.0.6、@tanstack/vue-form 1.33.2、@tauri-apps/api 2.11.0、@tauri-apps/cli 2.11.3、@tauri-apps/plugin-process 2.3.1、@tauri-apps/plugin-updater 2.10.1 等。
- 后端 `src-tauri/Cargo.toml`：依赖全部 `=x.y.z` 精确锁定（tauri 2.11.5、axum 0.8.4、reqwest 0.12.23、rusqlite 0.32.1、tauri-plugin-updater 2.10.1 等），edition 2021，rust-version 1.77.2。
- 本机工具链：cargo 1.95.0 / rustc 1.95.0，满足最新 crate 的 MSRV。
- `npm outdated` 显示 major 级跳升：eslint 9→10、vue-router 4→5、vite 7→8、typescript 5→7、@eslint/js 9→10、globals 16→17；minor/patch：vue 3.5.41、tailwindcss 4.3.3、happier-ui 0.1.1 等。
- Tauri 前后端版本必须匹配（@tauri-apps/api / @tauri-apps/cli ↔ tauri / tauri-plugin-* crate）。

## Decisions

| 决策 | 结论 |
|------|------|
| 范围 | 前端 npm + 后端 Rust **全部升到 latest stable**（用户确认 A） |
| TypeScript | 升到 **6.x latest**（TS7 Go 编译器 Corsa API 未稳定，vue-tsc / typescript-eslint 尚不兼容；用户确认 A） |
| 顺序 | 先 npm（typecheck/lint/test/build 绿）→ 再 cargo（cargo build 绿）→ tauri build 全量 |
| Rust 锁定 | 保留 `=x.y.z` 精确锁定（用户确认 A） |
| 验证失败 | 逐项适配修复，不轻易回退版本（除非破坏性变更超出适配成本，回退点见 design） |

## Requirements

1. **R1 前端 npm 升级**
   - `dependencies` + `devDependencies` 全部升到 npm latest stable
   - `pnpm-lock.yaml` 同步更新
   - major 升级适配：eslint 9→10（flat config）、typescript →6.x（vue-tsc 兼容）、vite 7→8（插件兼容）、vue-router 4→5（API 变更）、happier-ui 0.0.6→0.1.1（组件库 API 变更）等

2. **R2 后端 Rust 升级**
   - `src-tauri/Cargo.toml` 全部 crate 升到最新稳定版本，保留 `=x.y.z` 精确锁定

3. **R3 版本匹配**
   - @tauri-apps/api / @tauri-apps/cli ↔ tauri / tauri-plugin-* crate 版本对齐
   - 升级后 Tauri 2.x 主版本不变（不升 Tauri 3.x，除非 latest 已是 3.x 且用户确认）

4. **R4 验证矩阵全绿**
   - `pnpm typecheck` / `pnpm lint` / `pnpm test:unit` / `pnpm build` 全绿
   - `cargo build`（或 `cargo check`）全绿
   - `pnpm tauri build`（或至少 tauri dev 编译）验证前后端集成
   - 运行时回归：应用可启动、核心页面可用

## Out of Scope

- 不做功能改动/重构（仅升级适配）
- 不引入新依赖
- 不升级 Tauri 3.x（若存在）除非用户明确确认
- 不处理 Node 运行时版本问题（假设当前 Node 满足 vite 8 / TS 7 要求，否则需用户确认）

## Open Questions

1. ~~范围口径~~ → **A：全部 latest**（TS 例外：升 6.x，因 TS7 生态未就绪）
2. ~~Tauri 主版本~~ → 无需确认：tauri crate latest 即 2.11.5（当前版本），无 Tauri 3.x 需要升
3. ~~Rust 依赖锁定策略~~ → **A：保留 `=x.y.z` 精确锁定**
4. 验证失败时的回退策略（逐项回退 vs 继续适配）

## Acceptance Criteria

- [ ] AC1：`pnpm outdated` 无过期项（全部 latest）
- [ ] AC2：`pnpm typecheck` / `pnpm lint` / `pnpm test:unit` / `pnpm build` 全绿
- [ ] AC3：`cargo build` 全绿（crate 全部 latest，无冲突）
- [ ] AC4：Tauri 前后端版本匹配，`pnpm tauri build` 通过
- [ ] AC5：major 升级的代码适配已完成（无 TODO 残留）
- [ ] AC6：应用可启动，核心功能无回归（手动冒烟）

## Notes

- 复杂任务：收敛需求后补 `design.md` + `implement.md`，再 `task.py start`。
- 升级前确认 Node 版本满足 vite 8 / TS 7 / eslint 10 要求。
