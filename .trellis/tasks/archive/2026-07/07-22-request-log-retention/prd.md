# 请求日志保留与清理

## Goal

避免 `request_logs` 无限增长：默认保留 30 天，启动/读写时自动清理过期行；日志页展示条数并支持一键清理过期。

## Requirements

- R1：默认保留 **30 天**（按 `time` unix 秒，删除 `time < now - 30*86400`）。
- R2：打开库 / 写日志 / 列日志时尽力自动清理过期（best-effort，失败不影响主路径）。
- R3：`purge_expired_logs` 命令；日志页「清理过期」按钮。
- R4：日志页展示当前库内总条数（可与筛选 total 区分：筛选 total 仍是过滤后；另显示库内总量或过期说明）。
- R5：保留「清空全部」。
- R6：单测覆盖删除边界。

## Acceptance Criteria

- [x] 超过 30 天的行可被 purge 掉
- [x] 30 天内行保留
- [x] 日志页可清理过期并看到条数
- [x] cargo test / typecheck 相关通过

## Out of Scope

- 可配置保留天数 UI（可先常量 30）
- 自动 VACUUM
