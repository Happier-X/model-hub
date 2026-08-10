# PRD：首页统计总览（请求次数 / token / 费用 / 耗时，总计 + 今日）

## Goal

首页顶部改为统计总览：两行指标（一行总计、一行今日），每行展示 7 项：
请求次数、消耗 token、消耗费用、输入 tokens、输入费用、输出 tokens、输出费用、消耗耗时（用户后补）。

## 已确认事实（repository evidence）

- 现有 `RequestStats`（`src/api/tauri.ts:108` / `src-tauri/src/domain/log.rs:416`）只有 total/success/failure/failover + 时间段，**无 token/费用/耗时**。
- 当前 `request_logs` 表（`src-tauri/src/db/migrate.rs:36`）列：time/group_name/provider_name/upstream_model/status_code/use_time_ms/error/failover_*，**无 input_tokens/output_tokens/cost**。
- 旧版 gateway-rust 表曾有 `input_tokens / output_tokens / cost` 列（`migrate.rs:707` 测试夹具），迁移逻辑保留旧列但当前 insert 不写这些列（`log.rs:174` 起只写 10 列）。
- `forward.rs` 无 usage 提取（无 prompt_tokens/completion_tokens 解析）。
- 系统无模型单价表（grep price/cost 无业务实现）。
- `use_time_ms` 已有（每请求耗时），聚合即可。
- 「今日」按本地自然日（现有 day_start_unix/day_end_unix 逻辑可复用）。

## Requirements

- R1 后端转发时记录每次请求的 token 用量（输入/输出），支持非流式与流式响应。
- R2 费用：本期不计算，展示为「-」（用户已选方案 A；后续任务再做单价配置）。
- R3 后端统计命令返回「总计」与「今日」两组指标：请求次数、输入/输出/总 tokens、总耗时（费用字段预留）。
- R4 前端首页顶部渲染两行统计卡片（总计 / 今日），费用列显示「-」。

## 已确认决策

- D1 费用方案 = A：本期不记录/不计算费用，UI 展示「-」，单价配置后续任务处理。
- D2 统计口径：所有指标（次数/token/耗时）均只统计成功请求（2xx 且 error 为空）。
- D3 耗时展示：自适应单位（<1s 毫秒、≥1s 显示 x.x s、≥60s 显示 x 分 y 秒）。

## Acceptance Criteria

- AC1 首页顶部出现统计总览：两行（总计 / 今日），每行含请求次数、输入/输出/总 tokens、总耗时、费用（「-」）。
- AC2 数字来自真实转发记录且仅统计成功请求（新产生成功请求后刷新可见增长，失败请求不计入）。
- AC3 旧库迁移无异常（旧 request_logs 表已存在 input_tokens 等列时 insert 不炸）。
- AC4 typecheck / lint / unit（cargo + node）/ build 全绿。

## Out of Scope（初版）

- 费用单价的可视化配置 UI（设置页）——取决于费用方案决策。
- 历史数据回填（旧日志无 token/费用记录，无法补）。

## Open Questions（阻塞规划）

- ~~Q1 费用方案~~ → 已决：A（本期「-」，后续再做单价）
- ~~Q2 耗时单位~~ → 已决：自适应（D3）
- ~~Q3 统计口径~~ → 已决：仅成功请求（D2）
