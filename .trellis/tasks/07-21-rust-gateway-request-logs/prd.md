# Rust 网关请求日志

## Goal

在 `gateway-rust` 记录 chat 转发请求日志，并提供与现有日志页兼容的 `list` / `clear` 管理 API。

## Background

- UI `RelayLog` 字段：`id, time, request_model_name, channel_name, actual_model_name, input_tokens, output_tokens, use_time, cost, error`
- API：`GET /api/v1/log/list?page=&page_size=`（page_size 上限 100）、`DELETE /api/v1/log/clear`
- 需管理 JWT；成功信封 `{ data }`

## Requirements

### R1. 持久化

- migrate v2：`request_logs` 表
- 字段对齐 UI；`time` 为 Unix 秒（或 UI 可兼容的秒/毫秒；优先秒）
- `page_size` clamp 1..=100；`page` 从 1 起

### R2. 记录时机

- 非流式 chat：转发结束后写一条日志（成功/业务错误/上游错误）
- 流式 chat：至少记录开始路由结果 + 结束时状态（可用 use_time；tokens 可从非流式 usage 解析，流式可为 0）
- 鉴权失败（401）**不**记业务日志（或可选记，默认不记）
- 路由失败（未知分组等）应记 error 非空
- 不落盘完整 messages / 密钥

### R3. API

- `GET /api/v1/log/list` → `{ data: RelayLog[] }`，按 id/time 倒序
- `DELETE /api/v1/log/clear` → `{ data: null }` 或空成功
- 均需管理 JWT

### R4. 边界

- 不实现服务端 keyword 过滤（UI 本地过滤）
- 不改前端/Tauri/发布
- cost 可固定 0；tokens 尽力从非流式 JSON `usage` 解析

## Acceptance Criteria

- [x] AC1：非流式 mock chat 后 list 可见对应日志字段。
- [x] AC2：page/page_size 分页与上限 100。
- [x] AC3：clear 后 list 为空。
- [x] AC4：无 JWT → 401。
- [x] AC5：日志不含完整 Key/messages。
- [x] AC6：fmt/check/test/clippy 通过；不改壳发布。

## Out of Scope

- 导出 CSV、按用户计费、实时 SSE 推送日志
