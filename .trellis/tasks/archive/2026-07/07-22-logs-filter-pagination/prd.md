# 日志筛选与分页

## Goal

日志页支持**服务端分页**与**常用筛选**，便于本机联调时定位故障转移与错误请求，而不只是固定拉前 100 条。

## 现状

- 后端 `list_logs(page, page_size)` 已有 LIMIT/OFFSET，page_size clamp 1–100。
- 无总数、无筛选条件。
- 前端固定 `listLogs(1, 100)`，无翻页、无筛选 UI。

## Requirements

- R1：查询结果返回 **items + total**（及 page / page_size），供分页控件使用。
- R2：筛选（均可空，组合为 AND）：
  - `group_name`：子串匹配（大小写不敏感优先，SQLite `LIKE` 可接受）；
  - `status_class`：`all` | `2xx` | `4xx` | `5xx` | `error`（error = 有 error 文本或 status≥400，实现选一种并写清）；
  - `failover_only`：仅展示发生故障转移的记录（from/to 非空）。
- R3：前端：页码、每页条数（如 20/50/100）、筛选控件、刷新后重置到第 1 页（改筛选时）。
- R4：清空日志后列表与 total 归零。
- R5：领域层单测：插入多条后分页边界与筛选正确。
- R6：不改变 insert_log 字段与代理写日志行为。

## Acceptance Criteria

- [x] 日志超过一页时可翻页，total 正确。
- [x] 按分组名筛选生效。
- [x] 按状态类别筛选生效。
- [x] 仅故障转移筛选生效。
- [x] `cargo test` 相关通过；`pnpm typecheck` / `pnpm lint` 通过。

## Out of Scope

- 全文搜索 messages（日志本就不存 messages）。
- 导出 CSV。
- 实时 WebSocket 推送日志。
