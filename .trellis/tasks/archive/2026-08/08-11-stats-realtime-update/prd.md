# 首页统计实时更新：事件驱动刷新 + 恢复可见即刷新

## Goal

用户痛点：首页顶部四张统计卡片（`src/components/StatsCards.vue`：请求统计 / 全部统计 / 输入统计 / 输出统计）"看起来"不实时更新——发完请求数字不变。

目标：请求完成 → 四张统计卡片**立即**反映最新数据；窗口从托盘恢复后数据为最新。

## Confirmed Facts（已实测/代码确认）

1. **后端链路完全实时**：实测启动应用后通过代理（127.0.0.1:8888）发请求，`request_logs` 立即新增记录（id 递增、time 为请求时刻）。日志写入点 `src-tauri/src/proxy/forward.rs`、`server.rs` 的 `insert_log_best_effort` 在请求结束时同步写入 SQLite，无延迟、无缓存。
2. **`request_logs` 表行数恒 10000 是清理策略**：`purge_logs` 按 `max_rows`（默认 10000）删旧插新，计数不变不代表不写入。
3. **成功口径（2xx 且 error 空）总量数据（实测）**：请求 482 次、输入 token 1682 万、输出 8.2 万、耗时 382.65 万 ms。今日请求 210 次、今日输入 token 1272 万。
4. **前端 5 秒轮询逻辑本身正常**：`src/pages/HomePage.vue` `onMounted` → `setInterval(refreshOverviewOnly, 5000)`，`onUnmounted` 清理。
5. **显示层面**：卡片显示 `overview.total`，`src/utils/formatOctopus.ts` 的 `formatCount`/`formatMoney`/`formatTime` 用 K/M 缩写 + `toFixed(2)`：
   - 请求次数 482 → `"482.00"`，单次 +1 可见（< 1e3 不缩写）；
   - Token 1682 万 → `"16.82M"`、耗时 1.06h → `"1.06h"`，单次请求增量被舍入吞掉（M 级要凑 ~5 万 token 才跳 0.01M）。
6. **窗口隐藏冻结**：主窗口关闭是 hide 到托盘（`src-tauri/src/lib.rs:81-82`），WebView2 隐藏时 JS 定时器被冻结，从托盘恢复后需等下一个 tick 才有新数据，体感"旧数据"。
7. **刷新机制现状**：StatsChart 30s 轮询（`src/components/StatsChart.vue:121`）、LogsPage 仅挂载时刷新、Overlay 2.5s 轮询（`src/OverlayApp.vue`）。
8. **项目当前无 Rust→JS 事件推送先例**（全仓无 `emit`）。
9. **领域层结构**：`Stores { pub db: DbConn }`（`Arc<Mutex<Connection>>`，`src-tauri/src/domain/mod.rs:12`），可 Clone，所有日志写入均经 `Stores::insert_log`（`src-tauri/src/domain/log.rs:242`）→ 变更订阅回调放此层可覆盖全部写入路径且不依赖 tauri。

## Requirements

- R1：Rust 侧请求日志写入成功后推送事件（`stats-changed`），前端首页监听后立即重新拉取 overview。
- R2：首页监听窗口恢复可见（`visibilitychange` → visible / `focus`）时立即拉取最新 overview，解决托盘恢复旧数据。
- R3：保留 5 秒轮询作为兜底（事件丢失 / 页面在事件前已打开时仍能刷新）。
- R4：不改变统计口径（成功：2xx 且 error 空）、不改变卡片显示格式与样式（octopus 风格，不加行、不改缩写规则）。

## Acceptance Criteria

- [x] AC1：通过代理发起一次成功请求后，首页统计卡片在**事件驱动下立即**（无需等 5 秒轮询）反映新数据；请求次数 +1 可见。
- [x] AC2：主窗口从托盘恢复可见时，统计卡片立即拉取最新数据（无需等下一个轮询 tick）。
- [x] AC3：5 秒兜底轮询保留，事件监听失败/丢失时统计仍会更新。
- [x] AC4：`pnpm typecheck`、`cargo build` 通过；Rust 侧新增逻辑不破坏现有测试（`cargo test` 147+13+9 全绿），并为变更订阅新增单元测试（`insert_log_notifies_change_subscribers`）。

## Out of Scope

- 折线图（StatsChart）与热力图（daily）不随事件刷新：StatsChart 保持 30s 轮询，热力图保持页面加载时拉取（daily 365 天聚合较重）。
- 不改统计口径、不改卡片显示格式（Token/耗时等 M 级数值单次增量仍可能被舍入吞掉——属显示精度问题，本期不改）。
- 不改 LogsPage / Overlay 的刷新机制。

## Open Questions

（无阻塞项；以下两点为设计内默认，若用户有异议需在实现前提出）
- Token/耗时卡片单次请求增量在 M 级舍入下不可见：本期接受（数据确实实时刷新），如需可见另立任务。
- 事件刷新频率与轮询并发：请求完成频率 = 用户请求频率（人类节奏），直接刷新无节流；若未来高频可加 1s 节流。
