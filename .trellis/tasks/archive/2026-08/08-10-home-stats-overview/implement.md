# Implement：首页统计总览

## 执行清单（按依赖排序）

### 1. 后端 schema（`src-tauri/src/db/migrate.rs`）
- [ ] 建表 SQL 加 `input_tokens INTEGER NOT NULL DEFAULT 0`、`output_tokens INTEGER NOT NULL DEFAULT 0`
- [ ] 旧库 ensure：复用 `request_logs_has_column` 模式补 `ALTER TABLE ADD COLUMN`（注意旧列兼容分支测试 `migrate_backfills_legacy_request_logs_names_into_current_columns` 已有 input_tokens 列的夹具——验证通过不回归）

### 2. 后端日志模型（`src-tauri/src/domain/log.rs`）
- [ ] `NewRequestLog` 增加 `input_tokens` / `output_tokens: i64`
- [ ] `insert_log` 双写补列（旧列兼容分支 + 新列分支各补 INSERT 列与参数）
- [ ] `list_logs` 查询列不变（日志页不展示 token，可选展示——本期不加列到前端日志表）

### 3. 转发链路（`src-tauri/src/proxy/forward.rs`）
- [ ] `rewrite_model` 增加流式注入：请求为流式、模型为 OpenAI 兼容家族、未带 `stream_options` 时注入 `{"stream_options":{"include_usage":true}}`（`apply_thinking_effort` 模式）
- [ ] 非流式成功分支：解析 `bytes` JSON `usage.prompt_tokens/completion_tokens` 写入 `NewRequestLog`
- [ ] 流式：`StreamState` 增加 usage 累积（每 chunk `serde_json` 解析顶层 `usage`，非空覆盖）；`on_success` 签名带 token，成功日志写入
- [ ] 所有 `NewRequestLog { ... }` 调用点补 `input_tokens`/`output_tokens`（非流式 1 处、流式回调 3 处、中间失败 2 处、测试 2 处）
- [ ] 新增纯函数便于测试：`extract_usage_from_json(&[u8]) -> Option<(i64, i64)>`（非流式）与流式 chunk 的 usage 提取（同一函数，chunk JSON 顶层 usage 即可）

### 4. 统计命令
- [ ] `domain/log.rs`：`request_overview(&self) -> Result<RequestOverview, AppError>`，两段 SQL（总 / 今日），本地自然日边界复用现有 day 计算
- [ ] `commands.rs`：`get_request_overview` tauri command
- [ ] 结构体 `RequestOverview { total: OverviewRow, today: OverviewRow }`、`OverviewRow { requests, input_tokens, output_tokens, use_time_ms, cost: f64 }`
- [ ] 测试：成功口径过滤（含 2xx+error 空）、today 边界、空库全 0

### 5. 前端
- [ ] `src/api/tauri.ts`：`RequestOverview` / `OverviewRow` 类型 + `getRequestOverview()`
- [ ] `HomePage.vue`：新「统计总览」卡片替换「今日请求」卡片；两行（总计/今日）× 6 指标（请求次数、输入 tokens、输出 tokens、总 tokens、耗时、费用「-」）；耗时格式化函数（放 `src/utils/` + node:test 单测）
- [ ] 随 `refreshStats` 一起并行加载，错误单独提示

### 6. 验证
- [ ] `cd src-tauri && cargo test --lib`（用户 dev 占用 exe 时用 --lib）
- [ ] `pnpm typecheck && pnpm lint && pnpm test:unit && pnpm build`
- [ ] 用户 dev 验证：发一次非流式 + 流式请求 → 首页总计/今日增长；失败请求不计数

## 风险文件 / 回滚点

- `forward.rs`（流式注入 + 旁路解析）——最高风险；注入逻辑独立函数，可整体注释回滚
- `migrate.rs` / `log.rs`（schema + 双写）——ensure 幂等
- `HomePage.vue`（布局改动）
- 回滚检查点：每个阶段结束跑 `cargo test --lib` 与 `pnpm typecheck`

## 遵循约定

- 实现前读 `.trellis/spec/backend/database-guidelines.md` 与 `frontend/component-guidelines.md`
- shadcn 组件用 `@/components/ui/*`；耗时格式化等纯函数放 `src/utils/`，测试用 node:test（`.ts` 扩展名导入）
