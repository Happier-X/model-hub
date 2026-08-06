# 实现计划：所有依赖升级到最新版本

## 阶段 1：前端 npm（先做，验证绿再动 Rust）

1. **基线确认**
   - [ ] `pnpm outdated` 记录当前/最新快照
   - [ ] 读 `tsconfig.app.json` / `tsconfig.node.json` 核对 TS6 移除项（moduleResolution / baseUrl 等）
   - [ ] 读 `eslint.config.js` 现状（flat config 兼容性）
   - [ ] 读 `happier-ui` 0.1.1 changelog（组件 API 变更）

2. **升级**
   - [ ] `pnpm up --latest`（deps + devDeps 全部 latest）—— 或逐组升级便于定位
   - [ ] typescript 单独 pin 到 6.x latest（npm up 可能拉到 7，需显式指定 `pnpm add -D typescript@^6`）
   - [ ] 更新 `pnpm-lock.yaml`

3. **适配与修复**
   - [ ] eslint 10：`eslint.config.js` 适配（@eslint/js 10、typescript-eslint、eslint-plugin-vue 组合）
   - [ ] vue-router 5：API 变更适配
   - [ ] happier-ui 0.1.1：组件用法适配（如 API 变更）
   - [ ] vite 8 / @vitejs/plugin-vue / @tailwindcss/vite：vite.config.ts 适配
   - [ ] TS6 严格模式新报错修复

4. **前端验证**
   - [ ] `pnpm typecheck` 绿
   - [ ] `pnpm lint` 绿
   - [ ] `pnpm test:unit` 绿
   - [ ] `pnpm build` 绿
   - [ ] `pnpm outdated` 复查（除有意保留项外无过期）

## 阶段 2：后端 Rust（npm 绿后）

5. **升级**
   - [ ] 逐 crate 更新 `Cargo.toml` 到 latest（保留 `=x.y.z`）
   - [ ] `cargo update` 更新 `Cargo.lock`

6. **适配与修复**
   - [ ] reqwest 0.13：feature `rustls-tls`→`rustls`；`query`/`form` feature 按需
   - [ ] rusqlite 0.40：API 变更适配（如需要）
   - [ ] tauri-plugin-single-instance 2.4.x、axum 0.8.9 等：编译错误修复

7. **后端验证**
   - [ ] `cargo check` 绿
   - [ ] `cargo test` 绿
   - [ ] `cargo build` 绿

## 阶段 3：集成验证

8. **集成**
   - [ ] 核对 @tauri-apps/api/cli/plugins ↔ tauri crate 版本对齐
   - [ ] `pnpm tauri build` 通过（或 tauri dev 编译）
   - [ ] 手动冒烟：应用启动、首页/供应商/分组/日志页可用

## 阶段 4：收尾

9. **质量**
   - [ ] `pnpm typecheck` / `pnpm lint` / `pnpm test:unit` / `pnpm build` 全绿
   - [ ] `cargo test` 全绿
   - [ ] 无 TODO 残留
   - [ ] 更新 spec（如依赖/构建约定有变化）

## 验证命令

```bash
pnpm outdated
pnpm typecheck && pnpm lint && pnpm test:unit && pnpm build
cargo check && cargo test
pnpm tauri build
```

## 风险文件 / 回滚点

- `package.json` + `pnpm-lock.yaml`（npm 全量升级，单 commit）
- `src-tauri/Cargo.toml` + `Cargo.lock`（cargo 全量升级，单 commit）
- 代码适配文件：`eslint.config.js`、`vite.config.ts`、`tsconfig*.json`、Rust 源码中 reqwest/rusqlite 用法
- 每阶段独立 commit：`chore(deps): 升级前端 npm 依赖到 latest` / `chore(deps): 升级后端 Rust 依赖到 latest` / 适配修复并入对应阶段
- 单包回退点：vue-router 5 / vite 8 / happier-ui 0.1.1 等若适配成本过高，回退到上一可用版本并记录原因

## start 前

- [x] prd / design / implement 齐备
- [ ] implement.jsonl / check.jsonl 已填真实 spec
- [ ] 用户批准本规划摘要
