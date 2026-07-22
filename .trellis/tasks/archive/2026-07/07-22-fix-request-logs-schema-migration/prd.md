# 修复 request_logs 缺列迁移

## Goal

修复旧 SQLite 中 `request_logs` 缺少 `status_code` 等列时，概览统计 / 日志查询失败（`no such column: status_code`），迁移幂等且不丢历史日志行。

## Background

`CREATE TABLE IF NOT EXISTS` 不会改已有表。早期或残缺 schema 的 `request_logs` 可能仅有部分列；新代码查询 `status_code` / `use_time_ms` / failover 字段会失败。

## Requirements

- R1：启动迁移检测 `request_logs` 目标列是否存在。
- R2：缺失时 `ALTER TABLE ... ADD COLUMN`，类型与默认值对齐当前 schema。
- R3：已存在则幂等；不重建表、不删行。
- R4：单测：缺列旧表迁移后可统计 / 查询。

## Acceptance Criteria

- [x] 缺 `status_code` 的旧表经 `migrate`/`open_db` 后 `request_stats` 可用。
- [x] 既有日志行保留；新列取默认值。
- [x] 重复迁移成功。
- [x] `cargo test --lib` 相关通过。
