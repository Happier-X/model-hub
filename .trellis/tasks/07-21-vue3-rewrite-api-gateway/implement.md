# 实现计划：Vue3 重写 + CC Switch 式故障转移

## 总原则

- 整仓重写，不兼容旧 React / 旧 `gateway-rust` 契约。
- 先 Rust 代理内核（可测），再 Vue 管理台，再 Tauri 壳收口。
- 每阶段可独立验收；失败可回滚到上一提交。

## 阶段 0：脚手架与清理边界

- [x] 0.1 新建 Vue 3 + Vite + TS + Tailwind 前端入口（替换 `src/` React）。
- [x] 0.2 `package.json` 依赖切换：移除 React，加入 `vue` / `vue-router`。
- [x] 0.3 `src-tauri` 去掉侧车启停，改为进程内代理任务。
- [x] 0.4 旧 React pages 已删；`gateway-rust/` 目录可残留但非运行时依赖（README 标明废弃）。
- [x] 0.5 数据目录：SQLite + 端口配置（`paths` + `settings`）。

**验证**：`pnpm install`；`pnpm typecheck` / `pnpm lint` 通过。

## 阶段 1：持久化与领域 CRUD（Rust）

- [x] 1.1 SQLite 迁移：providers / groups / group_items / api_keys / request_logs（含 failover 字段）。
- [x] 1.2 Provider / Group（有序队列）/ ApiKey store。
- [x] 1.3 Tauri commands：list/create/update/delete；Key 创建返回一次明文。
- [x] 1.4 单测：CRUD、队列排序、Key 校验。

## 阶段 2：代理 HTTP + 鉴权 + 基础转发

- [x] 2.1 进程内 HTTP server（默认 `127.0.0.1`）。
- [x] 2.2 `GET /health`；`GET /v1/models`（Key 鉴权 + 分组列表）。
- [x] 2.3 `POST /v1/chat/completions`：转发 + model 改写；非流式 + SSE。
- [x] 2.4 客户端 Key 强制；401。
- [x] 2.5 请求日志写入（脱敏）。

## 阶段 3：故障转移 + 默认熔断（核心）

- [x] 3.1 有序候选 + `auto_failover`。
- [x] 3.2 `forward_with_retry`。
- [x] 3.3 非流式读完 body。
- [x] 3.4 流式 prime 首包后透传。
- [x] 3.5 熔断状态机 + 默认常量。
- [x] 3.6 健康快照 + 故障转移日志字段。
- [x] 3.7 集成测：缺 Key 401、models 列表、5xx→换源成功。

## 阶段 4：Vue 管理台

- [x] 4.1 布局 + 导航：概览、供应商、分组、Key、日志。
- [x] 4.2 Tauri invoke 对接。
- [x] 4.3 分组队列与自动故障转移开关（基础 UI，拖拽可后续增强）。
- [x] 4.4 健康 + 日志列表。
- [x] 4.5 概览：Base URL / 启停 / 端口。

## 阶段 5：壳收口与验收

- [x] 5.1 启动自动起代理；退出停代理。
- [x] 5.2 README 标明旧侧车废弃；`package.json` 去掉 prepare:gateway-rust 脚本。
- [x] 5.3 README 已重写。
- [ ] 5.4 按 PRD AC1–AC12 手工完整过一遍（`pnpm tauri dev` 真机）。
- [x] 5.5 `pnpm lint` / `pnpm typecheck` / `cargo test` 已通过（见会话记录）。

**验证**：完整 AC 勾选。

## 建议验证命令

```powershell
pnpm install
pnpm lint
# Vue 类型检查（以最终 package scripts 为准）
pnpm exec vue-tsc --noEmit

cd src-tauri
cargo test
cargo check

# 根目录
pnpm tauri dev
```

手工：

1. 创建两个 Provider（一个故意错误 Key/URL）。
2. 分组队列：坏源在前、好源在后；开自动故障转移。
3. 带客户端 Key 请求 chat → 成功且日志有转移。
4. 无 Key → 401。
5. 流式请求正常输出。

## 风险文件 / 区域

- `src-tauri/src/**`：壳与代理合并，易纠缠 → 模块边界按 design §5。
- 流式 body：注意 WebView 不消费 `/v1`，客户端是外部进程。
- 删除旧前端时勿误删 `docs/` 有用说明；客户端文档需重写。

## 父/子任务建议（可选）

单任务可交付；若工期拆分：

1. `proxy-core`：阶段 1–3  
2. `vue-admin`：阶段 4  
3. `tauri-shell`：阶段 0 壳改造 + 阶段 5  

依赖写在子任务 PRD：vue 依赖 proxy commands；shell 依赖 proxy start API。

## 启动实现前检查清单

- [x] `prd.md` 验收标准可测  
- [x] `design.md` 架构与边界  
- [x] `implement.md` 本文件  
- [ ] `implement.jsonl` / `check.jsonl` 非 seed  
- [ ] 用户评审通过后 `task.py start`  
