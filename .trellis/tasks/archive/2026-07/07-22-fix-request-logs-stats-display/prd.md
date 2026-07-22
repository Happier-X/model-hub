# 修复请求日志与统计不显示

## Goal

客户端（含 Pi 流式）发起 Chat 后，日志页与概览「今日请求」应能看到记录。

## Root cause

1. **主因（用户机实测）**：旧 `request_logs` 仍有 `request_model_name` 等 NOT NULL 列；当前 INSERT 只写新列 → `NOT NULL constraint failed`，写库失败后 UI 无日志/统计。
2. 流式路径 `defer_request_log`：body 未完整消费 / 被 drop 时终态回调不触发。
3. `insert_log` 失败曾被 `let _ =` 静默忽略（现已 tracing 警告）。

## Requirements

- R1：流式 body 被 drop 且未终态时，仍写入一条中断类日志。
- R2：`insert_log` 失败写 tracing 警告。
- R3：旧库双写 `request_model_name/channel_name/actual_model_name/use_time`。
- R4：非流式成功 + 流式 drop 集成测试。

## Acceptance Criteria

- [x] 非流式 chat 成功后 list_logs total ≥ 1
- [x] 流式未完整消费时仍有日志
- [x] 统计 total 与日志一致（同库）
- [x] 旧 `request_model_name` 表可 insert_log
- [x] cargo test 相关通过
