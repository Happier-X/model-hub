# 执行计划：首页每日请求量热力图

## 前置

- [x] `happier-ui` 已升级 `0.0.3 → 0.0.6`（`package.json` / `pnpm-lock.yaml` 已改，未提交）
- [x] 确认 `HHeatmap` 导出与 props（`data`/`loading` 等）

## 步骤

### 1. 后端领域层 `src-tauri/src/domain/log.rs`
- [ ] 新增常量 `DAILY_COUNTS_DEFAULT_DAYS=365`、`DAILY_COUNTS_MAX_DAYS=400`
- [ ] 新增私有 `local_day_start_unix(ts: i64) -> i64`（本地日归桶）
- [ ] 新增私有 `daily_window_bounds(days: u32) -> (i64, i64)`（复用 `local_day_bounds_unix`）
- [ ] 新增 `DailyCount` / `RequestDailyCounts`（`#[derive(Debug, Clone, Serialize)]`）
- [ ] 新增 `Stores::request_daily_counts(&self, days: u32) -> Result<RequestDailyCounts, AppError>`
  - `clamp(1, MAX)` 兜底
  - 单 SQL 取窗口内 `time`，应用层 `HashMap<i64,i64>` 分桶
  - 仅输出 `count>0`，按 `day_start_unix` 升序

### 2. IPC `src-tauri/src/commands.rs`
- [ ] 新增 `get_request_daily_counts(proxy, days: Option<u32>)`，`unwrap_or(365)` 后调用领域方法

### 3. 注册 `src-tauri/src/lib.rs`
- [ ] `invoke_handler` 追加 `commands::get_request_daily_counts`（放在 `get_last_success_request` 附近）

### 4. 前端 API `src/api/tauri.ts`
- [ ] 新增 `DailyCount` / `RequestDailyCounts` 接口
- [ ] 新增 `getRequestDailyCounts(days?: number)`

### 5. 前端 UI `src/pages/HomePage.vue`
- [ ] `import { HHeatmap } from "happier-ui"` + `import type { HHeatmapData }`
- [ ] 新增 `daily` / `dailyError` / `dailyLoading` ref
- [ ] `heatmapData` computed 映射（秒 ×1000 → ms）
- [ ] `refreshStats()` 并入热力图拉取（`Promise.all`）
- [ ] 新增热力图 `HCard`（放「今日请求」卡下方）；`:loading` + 错误文案

### 6. 后端单测 `domain/log.rs` tests
- [ ] `daily_counts_buckets_by_local_day`
- [ ] `daily_counts_respects_window_and_clamp`
- [ ] `daily_counts_empty_db_returns_no_buckets`
- [ ] （可选）用 `with_conn` 手插不同 `time` 验证多桶

## 验证命令

```bash
cd src-tauri && cargo test && cargo check
cd .. && pnpm lint && pnpm typecheck && pnpm test:unit
```

## Review Gate

- 全部校验命令绿；热力图在 `pnpm tauri dev`（如可运行）或至少 typecheck 层确认无类型错。

## Rollback

- 纯新增，`git revert` 单 commit；无 DB 迁移回退。

## Commit

- 单 commit（沿项目习惯，master 直接提交），含 `package.json`/`pnpm-lock.yaml` 升级。
- 建议信息：`feat(home): 首页新增每日请求量热力图（HHeatmap）`
