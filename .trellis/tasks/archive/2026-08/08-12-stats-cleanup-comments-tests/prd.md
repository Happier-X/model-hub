# 修复统计遗留注释与补 clear_logs 聚合语义测试

## Goal

清理 stats-daily-aggregate 交付时遗留的两处小瑕疵：过时的费用注释误导阅读，`clear_logs` 保留聚合表的语义无测试锁定。

## Confirmed Facts

- 瑕疵 1：`src-tauri/src/domain/log.rs:591`，`request_overview` 的 doc 注释含过时行“费用字段本期恒 0，单价配置在后续任务引入”——实际费用已按 `model_pricing` 单价现算（08f07aa 后），与现状不符。
- 瑕疵 2：`clear_logs`（log.rs:452）只执行 `DELETE FROM request_logs`，`daily_request_stats` 聚合表保留（设计意图：清空日志列表不抹掉累计统计）；当前无专门测试锁定此语义。

## Requirements

- R1：删除 `request_overview` doc 中过时注释行；保留“仅成功请求（2xx 且 error 为空）”等仍有效的说明。
- R2：新增领域测试锁定 `clear_logs` 语义：清理后 `request_logs` 为空、`daily_request_stats` 行与累计值完整保留；`request_overview` 总计不受影响。
- R3：不改任何实现逻辑；`cargo test` 全量通过、`cargo build` 通过。

## Acceptance Criteria

- [ ] AC1：`request_overview` doc 注释不再含“费用恒 0”误导文字。
- [ ] AC2：新增 `clear_logs_keeps_daily_stats` 测试通过：clear 后明细 0 行、聚合行保留、overview total 不变。
- [ ] AC3：`cargo test` 全量通过（既有测试不回归）；`cargo build` 通过。

## Out of Scope

- 不改 `clear_logs` 行为本身、不改聚合/统计实现、不触碰前端。
- 不处理 `src-tauri/target-build-check/`（未跟踪构建产物，与本任务无关）。
