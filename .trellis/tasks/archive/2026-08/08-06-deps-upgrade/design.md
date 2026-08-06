# 设计：所有依赖升级到最新版本

## 边界

| 层 | 改动 |
|----|------|
| 前端 | `package.json` + `pnpm-lock.yaml`；可能的代码适配（eslint flat config、vue-router、happier-ui、vite 插件） |
| 后端 | `src-tauri/Cargo.toml` + `Cargo.lock`；可能的 Rust 代码适配（reqwest feature 改名等） |
| 集成 | Tauri 前后端版本对齐 |

## 升级目标版本（调研结论）

### 前端 npm

| 包 | 当前 | 目标 | 备注 |
|----|------|------|------|
| typescript | 5.9.2 | **6.x latest** | TS7 Corsa API 未稳定，vue-tsc/typescript-eslint 不兼容；升 6.x |
| eslint | 9.35.0 | 10.x | flat config 已就绪（项目已用 eslint.config.js），升 10 |
| vue | 3.5.18 | 3.5.41 | minor，低风险 |
| vue-router | 4.5.1 | 5.x | major，需查 breaking changes |
| vite | 7.1.5 | 8.x | major，需验证 @vitejs/plugin-vue / @tailwindcss/vite 兼容 |
| tailwindcss | 4.1.13 | 4.3.3 | minor，低风险 |
| @tailwindcss/vite | 4.1.13 | 4.3.3 | minor，低风险 |
| happier-ui | 0.0.6 | 0.1.1 | 自有组件库，需查 changelog / API 变更 |
| @tanstack/vue-form | 1.33.2 | 1.33.3 | patch，低风险 |
| @lucide/vue | 1.26.0 | 1.28.0 | minor，低风险 |
| @tauri-apps/api | 2.11.0 | 2.11.1 | patch，与 crate 对齐 |
| @tauri-apps/cli | 2.11.3 | 2.11.4 | patch |
| @tauri-apps/plugin-process | 2.3.1 | latest 2.x | 与 crate 对齐 |
| @tauri-apps/plugin-updater | 2.10.1 | latest 2.x | 与 crate 对齐 |
| @eslint/js | 9.35.0 | 10.x | 与 eslint 对齐 |
| typescript-eslint | 8.43.0 | 8.x latest | 需与 TS 6.x 兼容 |
| eslint-plugin-vue | 10.4.0 | 10.10.0 | minor |
| globals | 16.3.0 | 17.x | major，需查变更 |
| vue-tsc | 3.0.5 | 3.3.9 | 需与 TS 6.x 兼容 |
| vue-eslint-parser | 10.2.0 | 10.4.1 | minor |
| @vitejs/plugin-vue | 6.0.1 | 6.0.8 | minor |

> 实施时以 `npm outdated` 实际输出为准（版本会随时间推移）。

### 后端 Rust（crates.io latest 调研）

| crate | 当前 | 目标 | 备注 |
|-------|------|------|------|
| tauri | =2.11.5 | =2.11.5 | **已是 latest**，无需升级 |
| tauri-build | =2.6.3 | latest 2.x | 查 latest |
| tauri-plugin-updater | =2.10.1 | latest 2.x | 与前端对齐 |
| tauri-plugin-process | =2.3.1 | latest 2.x | 与前端对齐 |
| tauri-plugin-single-instance | =2.3.4 | =2.4.3 | major minor 跳升 |
| axum | =0.8.4 | =0.8.9 | 0.x minor = breaking，需 cargo check |
| reqwest | =0.12.23 | =0.13.4 | **major**：`rustls-tls` feature 改名 `rustls`，`query`/`form` 变可选 feature |
| rusqlite | =0.32.1 | =0.40.1 | **major**：VTab API 破坏（本项目 bundled 常规 CRUD 影响小） |
| tokio | =1.47.1 | latest 1.x | minor |
| serde/serde_json | 1.x | latest | patch |
| chrono | =0.4.41 | latest 0.4.x | 查 latest |
| thiserror | 2.0.16 | latest 2.x | 查 latest |
| futures-util | =0.3.31 | latest 0.3.x | 查 latest |
| bytes | =1.10.1 | latest 1.x | 查 latest |
| tower-http | 0.6.8 | latest 0.6.x | 查 latest |
| tracing / tracing-subscriber | 0.1.x / 0.3.x | latest | 查 latest |
| tempfile（dev） | 3.20 | latest 3.x | 查 latest |
| tower（dev） | =0.5.2 | latest 0.5.x | 查 latest |
| wiremock（dev） | =0.6.5 | latest 0.6.x | 查 latest |

> 实施时以 `cargo search` / `cargo update` 实际输出为准。

## 关键风险与适配点

### 1. TypeScript 6.x（约束）
- TS 6.0 引入 `--stableTypeOrdering`（7.0 中强制默认），可提前开启验证
- 硬移除：`moduleResolution: node/node10`（本项目 tsconfig 用 `bundler`？需核对）、`baseUrl` 等
- **需核对 `tsconfig.app.json` / `tsconfig.node.json` 是否触碰 TS6/7 移除项**

### 2. vue-router 4→5
- 需查官方 breaking changes；常见影响：路由元信息类型、`createWebHistory`、导航守卫签名

### 3. eslint 10
- 项目已有 `eslint.config.js`（flat config）→ 主体可复用
- `@eslint/js` 10 + `typescript-eslint` 8.x latest + `eslint-plugin-vue` 10.10 的兼容组合需验证

### 4. happier-ui 0.0.6→0.1.1
- 0.0.x→0.1.x 可能有组件 API 变更；实施时读 changelog，检查 `src` 中所有 `H*` 组件用法

### 5. reqwest 0.13
- Cargo.toml：`features = ["json", "rustls-tls", "stream"]` → `["json", "rustls", "stream"]`
- 若代码用 `query()` 需加 `query` feature（本次升级后验证）
- TLS 方法名软改名，旧名保留，一般无需改代码

### 6. rusqlite 0.40
- bundled 常规 CRUD API 兼容；若用到 `Sqlite*` 前缀类型名需适配（早版本已改名）
- cargo check 验证

## 数据流 / 合同（不变）

- 本任务不改业务逻辑，仅升级工具链与依赖
- 前端 `src/api/tauri.ts` invoke 封装、后端 IPC 命令签名均不改变
- 版本对齐契约：@tauri-apps/api 2.x ↔ tauri crate 2.x；plugin-process / plugin-updater 两侧同步

## 回滚策略

- npm：`git revert` 或 `pnpm install` 回旧 lock；分阶段提交便于定位
- cargo：`git revert` Cargo.toml/lock；依赖无数据迁移
- **回退点**：如果某 major 升级（如 vue-router 5、vite 8）适配成本过高或引入行为回归，单包回退到上一可用版本，记录原因到 prd Notes，不阻塞其余升级

## 验证矩阵

```bash
# 前端
pnpm typecheck && pnpm lint && pnpm test:unit && pnpm build
# 后端
cargo check        # 快速编译验证
cargo test         # Rust 单测（如有）
# 集成
pnpm tauri build   # 或 pnpm tauri dev 手动冒烟
```
